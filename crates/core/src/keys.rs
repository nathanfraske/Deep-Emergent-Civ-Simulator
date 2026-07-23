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

//! Canonical draw keying (design Part 3.2, the R-RNG-COORD resolution).
//!
//! [`Rng::for_coords`](crate::rng::Rng::for_coords) is the order-sensitive fold
//! primitive. This module pins the schema over it so the whole engine keys every
//! draw the same way:
//!
//! - A typed [`Phase`] identifier. The active abiotic layer does not publish a closed
//!   domain registry; any canonical phase identity must come from its owning protocol.
//! - A fixed canonical coordinate order ([`DrawKey`]), so every site folds the same
//!   fields in the same places (region, locus, secondary locus, tick, phase, slot).
//!   The tick is always present, which the older `(entity, phase)` coordinate omitted.
//! - Explicit draw-site namespacing via [`DrawKey::slot`], so two distinct rolls in one
//!   (locus, phase, tick) stream do not collide on counter zero.
//! - A degrade rule: a coordinate that does not apply folds as [`ABSENT`], distinct
//!   from a present zero, so "no secondary locus" never aliases "secondary locus 0".
//!
//! The keying stays integer and counter-based: a draw is
//! `key.rng(seed).at(counter)`, a pure function of its coordinate.

use crate::rng::Rng;

/// A simulation phase identifier folded into a draw coordinate.
///
/// The active abiotic substrate carries the generic identifier without a closed list of
/// domain phases. Retired biology and civilization phase constants are available only
/// under the `legacy-domains` compatibility feature used by the parked workspace.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub struct Phase(pub u32);

#[cfg(feature = "legacy-domains")]
impl Phase {
    /// A perception roll: whether a mind perceives a trace.
    pub const PERCEPTION: Phase = Phase(0x01);
    /// Choosing a gossip listener.
    pub const GOSSIP: Phase = Phase(0x02);
    /// Choosing a naming-game partner and concept.
    pub const LANGUAGE: Phase = Phase(0x03);
    /// The innovation roll: whether to coin a fresh word.
    pub const INNOVATE: Phase = Phase(0x04);
    /// Minting a fresh word form.
    pub const COIN: Phase = Phase(0x05);
    /// A lineage innovating a regular form change (drift).
    pub const DRIFT: Phase = Phase(0x06);
    /// A modelled-dialogue draw (choosing an addressee, breaking a move-kind tie). Move-
    /// scoped draws are keyed without the addressee, addressee-scoped draws with it, each
    /// on its own slot (the determinism pins of Part 9.5).
    pub const CONVERSE: Phase = Phase(0x07);
    /// A reproduction draw (gamete strand choice, crossover, point mutation), keyed on the
    /// contributing parent and the locus so a lineage is bit-identical (design 25.4, 25.5).
    pub const REPRODUCE: Phase = Phase(0x08);
    /// A Wright-Fisher drift resample of an allele-frequency pool over one deep-time
    /// generation (design 25.7), keyed on the pool and the locus.
    pub const EVOLVE: Phase = Phase(0x09);
    /// Sampling an explicit genome from a pool's frequencies on promotion (design 25.8),
    /// keyed on the new being's id.
    pub const PROMOTE: Phase = Phase(0x0A);
    /// Drawing a child's bounded axiom-seed mutation on inheritance (design Part 28), keyed on
    /// the child's id and the axiom axis so two axes of one child never collide.
    pub const AXIOM_INHERIT: Phase = Phase(0x0B);
    /// A mortality roll: whether a being dies this life-cadence given its age hazard (design
    /// Part 20, the R-AGING life-process loop), keyed on the being and its age (the age occupies
    /// the tick coordinate), so a being faces the same hazard at the same age on replay.
    pub const MORTALITY: Phase = Phase(0x0C);
    /// A worldgen lattice draw (terrain genesis). Genesis-time, so its draws carry no
    /// tick; the field being sampled is the draw-site slot and the octave the region.
    pub const WORLDGEN: Phase = Phase(0x10);
    /// A biosphere generate-and-validate species-sample draw (R-BIOSPHERE): sampling a
    /// candidate species over the trait axes, keyed on the niche locus and the pre-dawn
    /// generation, with the axis at its own counter and the resample attempt on its own slot.
    pub const BIOSPHERE_SAMPLE: Phase = Phase(0x0D);
    /// A biosphere genesis draw: an organism's per-tissue composition or a consumer's
    /// physiology vector drawn at genesis, keyed on the species and the axis ordinal.
    pub const GENESIS: Phase = Phase(0x0E);
    /// A founder-fork draw (the founder effect): binomial-sampling a founder pool off a
    /// parent at a small effective size, keyed on the founder id, locus, and generation.
    pub const FOUND: Phase = Phase(0x0F);
    /// A speciation draw: the Orr-snowball roll growing a Dobzhansky-Muller incompatibility
    /// as lineages diverge, keyed on the ordered pair, the locus pair, and the generation so
    /// the count accumulates per sweep rather than re-rolling once.
    pub const SPECIATE: Phase = Phase(0x11);
    /// An exploration draw: the heading a being takes when it is searching for a resource it does
    /// not yet know of, keyed on the being and the exploration period so its search is a
    /// reproducible function of the seed, the being, and the tick, never of the camera (a being
    /// discovers the world by moving through it, it does not read the map like a god).
    pub const EXPLORE: Phase = Phase(0x12);
    /// A behaviour-controller draw (R-BEHAVIOR-EVOLVE): the initial random controller weights of a
    /// founder lineage and the bounded mutation of a controller weight on inheritance, keyed on the
    /// individual and the controller-parameter locus so a lineage's evolved behaviour is a
    /// reproducible function of the seed and its ancestry (design Part 8, the evolved-behaviour work).
    pub const CONTROLLER: Phase = Phase(0x13);
    /// A grammar-typology draw (R-LANG-TYPOLOGY): one parameter of a culture's typology
    /// profile sampled at culture genesis, keyed on the culture and the parameter's
    /// canonical position in the anchor-first sampling order, so a culture's grammar is a
    /// reproducible function of the seed, the culture, and the registry data (design 33.4).
    pub const LANG_TYPOLOGY: Phase = Phase(0x14);
    /// A mate-choice draw (R-REPRO): the random founder preference weights and their bounded
    /// mutation in the prototype selection loop that shows a mate-preference direction emerge
    /// under genome-derived offspring fitness, keyed on the lineage and the generation so a
    /// run replays bit for bit (design Part 25, the R-BEHAVIOR-EVOLVE selection precedent).
    pub const MATE_CHOICE: Phase = Phase(0x15);
    /// The per-being developmental-environment offset draw (design Part 25.6): a mean-zero
    /// symmetric deviation that makes a member's expressed cognition vary from its cohort, the
    /// environmental-variance (V_E) half of narrow-sense heritability. Keyed on the being's id
    /// (the tick coordinate carries the dawn's tick 0 or a birth's generation), so a member's
    /// developmental deviation is a reproducible function of the seed and the being rather than a
    /// single environment shared across the whole cohort. Non-heritable: it is applied at
    /// expression and never folded back into a pool's allele frequencies.
    pub const DEVELOPMENT: Phase = Phase(0x16);
    /// A knowledge-transmission copy draw (the transmission substrate): the bounded, mean-zero
    /// proficiency drift a learner incurs when copying a design from a holder, keyed on the LEARNER
    /// (the region coordinate), the holder and the design's content address, and the tick, so a
    /// copy-of-a-copy replays bit for bit and N learners copying one design from one holder on one
    /// tick draw N DISTINCT perturbations (keying on holder, design, and tick alone gave every
    /// learner the identical stream). The perturbation magnitude is a function of the copier's
    /// per-race perception and memory (Principle 9), never an authored per-race fidelity table.
    pub const TRANSMIT: Phase = Phase(0x17);
    /// A knowledge-loss erosion draw (the transmission substrate): the per-design, per-tick
    /// forgetting roll that erodes the proficiency of a design held by fewer than the
    /// minimum-viable practitioner count, keyed on the design and the tick so every below-floor
    /// holder erodes in lockstep and the erosion replays bit for bit. Its expectation is the
    /// reserved loss rate and it is always non-negative, so proficiency only erodes.
    pub const KNOW_LOSS: Phase = Phase(0x18);
    /// A belief-lifting per-mind dispersion draw (the belief facet-strength substrate, Part 54):
    /// the small symmetric mean-zero deviation added around the level-to-strength curve when an
    /// aggregate pool's prevailing belief is instantiated into a promoting mind's facet strength,
    /// keyed on the being, the belief's content hash, and the tick, so a mind promoted holding
    /// several beliefs perturbs each independently and the lift replays bit for bit. The dispersion
    /// magnitude is a reserved calibration; the draw keys on no belief's identity (Principle 9).
    pub const BELIEF_LIFT: Phase = Phase(0x19);
    /// The institution-crystallization tie-break draw (the Part 36 institution substrate): when
    /// two ripe coordination patterns share an exact canonical key and must be assigned a
    /// crystallization order, this stream breaks the tie, folding the master seed, the locus, the
    /// tick, and this phase, with the pattern's secondary key as the RNG locus. It is reached only
    /// for a genuine key tie; distinct patterns sort by their canonical key alone and never touch
    /// the stream, so crystallization order is a pure function of canonical state (Principle 3).
    pub const CRYSTALLIZE: Phase = Phase(0x1A);
    /// The developmental-growth draw (the emergent-anatomy morphogen kernel, Step 2): the bounded,
    /// integer Fixed-point recursion that grows a body's structure from a genome's expressed growth-rule
    /// parameters keys each stochastic growth branch (whether a segment spawns a child, and its geometry
    /// jitter) on this phase, folding the master seed, the growing being's id as the locus, the parent
    /// segment and child index, and the generation, with a per-draw-site slot. Growth is a pure function
    /// of the genome, the seed, and the being id, blind to any race, kind, or niche (Principle 9), and the
    /// grown structure folds into `state_hash` in canonical segment order.
    pub const MORPHOGEN: Phase = Phase(0x1B);
    /// A hypothesis-sampling draw (the ideation / experiential-discovery arc, piece 2, slice 2b): choosing
    /// which candidate action a being PROPOSES this tick from the binding graph, the belief-and-need-weighted
    /// roulette over the primitive-times-affordance candidates it could try. Keyed on the being and the tick
    /// (or sequence step), so a proposed hypothesis is a reproducible function of the seed, the being, and
    /// the tick, never of the camera: a being explores the action space by trying things, it does not read
    /// the answer like a god. Distinct from EXPLORE (a movement heading) so the two draws never collide on a
    /// shared counter.
    pub const HYPOTHESIZE: Phase = Phase(0x1C);
    /// An exploration-enact draw (the ideation / experiential-discovery arc, piece 2, slice 2c-2): whether a
    /// being ACTS on the candidate action it proposed this tick, gated by its own heritable exploration
    /// propensity, founder-zero. Keyed on the being and the tick, so whether a being tries its hypothesis is
    /// a reproducible function of the seed, the being, and the tick, never the camera. Distinct from
    /// HYPOTHESIZE (which candidate it proposes) so the propose and the enact draws never collide on a
    /// counter: a being that proposes an action still enacts it only when its exploration fires.
    pub const ENACT: Phase = Phase(0x1D);
    /// A deliberation-enact draw (the ideation / experiential-discovery arc, piece 4, slice 4b): whether a
    /// being ACTS on the believed-best action its planner recalled toward a goal this tick, gated by its own
    /// heritable DELIBERATION weight, founder-zero. Keyed on the being and the tick. Distinct from ENACT
    /// (whether it acts on an EXPLORED novel proposal) so the deliberate and the explore draws never collide
    /// on a counter: exploration tries the untried, deliberation exploits the best-believed, and a being's
    /// two heritable drives are drawn independently so selection can tune each.
    pub const DELIBERATE: Phase = Phase(0x1E);
    /// A creature reproduction/behaviour-selection draw (creature-selection step 2): the bounded, mean-zero
    /// per-weight controller perturbation applied when a creature is MINTED (the bootstrap variance seed that
    /// breaks the founder-zero deadlock) and when an OFFSPRING is bred (the inheritance mutation on the
    /// midparent blend). Keyed on the being's id (a creature id, disjoint from every founder id) and the tick,
    /// with the weight index as the per-draw counter, so each creature's per-weight deviations are distinct and
    /// the whole substrate replays bit for bit. Distinct from DEVELOPMENT (the founder expression offset) and
    /// MATE_CHOICE so the creature draws never collide with a founder's on a counter.
    pub const CREATURE_REPRO: Phase = Phase(0x1F);
    /// A derived-taxis TUMBLE draw (the run-and-tumble survival floor): whether a being RE-ORIENTS this tick
    /// under the always-on motility floor, drawn against a tumble probability that is a continuous function of
    /// the being's OWN interoceptive reserve derivative (a rising reserve lowers it so the being persists on
    /// its heading, a falling reserve raises it so it re-orients). Keyed on the being and the tick, so whether a
    /// being tumbles is a reproducible function of the seed, the being, and the tick, never the camera. Distinct
    /// from EXPLORE (the fixed-period search heading) so the floor's tumble draw never collides with the
    /// controller-driven explore on a shared counter.
    pub const TUMBLE: Phase = Phase(0x20);
}

/// The sentinel for a coordinate that does not apply to a draw (the degrade rule). An
/// absent coordinate folds as `ABSENT`, distinct from a present zero.
pub const ABSENT: u64 = u64::MAX;

/// A canonical draw coordinate (R-RNG-COORD). The field fold order in [`DrawKey::rng`]
/// is the contract: every engine draw is keyed by this schema, so coordinate order is
/// uniform across sites and each site is namespaced by its phase and slot. Construct
/// one with [`DrawKey::entity`] or [`DrawKey::pair`], refine with [`DrawKey::in_region`]
/// and [`DrawKey::slot`], then call [`DrawKey::rng`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DrawKey {
    region: u64,
    locus: u64,
    locus2: u64,
    tick: u64,
    phase: Phase,
    slot: u32,
}

impl DrawKey {
    /// A draw for one locus (an entity, a lineage) in a phase at a tick.
    #[inline]
    pub fn entity(locus: u64, tick: u64, phase: Phase) -> Self {
        DrawKey {
            region: ABSENT,
            locus,
            locus2: ABSENT,
            tick,
            phase,
            slot: 0,
        }
    }

    /// A draw concerning two loci (a perceiver and a trace, a listener and a subject).
    #[inline]
    pub fn pair(locus: u64, locus2: u64, tick: u64, phase: Phase) -> Self {
        DrawKey {
            region: ABSENT,
            locus,
            locus2,
            tick,
            phase,
            slot: 0,
        }
    }

    /// Set the spatial region coordinate (defaults to [`ABSENT`]).
    #[inline]
    pub fn in_region(mut self, region: u64) -> Self {
        self.region = region;
        self
    }

    /// Set the draw-site slot, namespacing distinct rolls within one phase so they do
    /// not collide on counter zero (defaults to 0).
    #[inline]
    pub fn slot(mut self, slot: u32) -> Self {
        self.slot = slot;
        self
    }

    /// Fold the coordinate into a stream. The field order here is the canonical fold and
    /// is part of the determinism contract.
    #[inline]
    pub fn rng(self, master_seed: u64) -> Rng {
        Rng::for_coords(
            master_seed,
            &[
                self.region,
                self.locus,
                self.locus2,
                self.tick,
                self.phase.0 as u64,
                self.slot as u64,
            ],
        )
    }
}

#[cfg(all(test, feature = "legacy-domains"))]
mod tests {
    use super::*;

    #[test]
    fn same_key_reproduces_the_same_stream() {
        let a = DrawKey::pair(10, 20, 5, Phase::GOSSIP).rng(7);
        let b = DrawKey::pair(10, 20, 5, Phase::GOSSIP).rng(7);
        assert_eq!(a, b, "same coordinate reproduces the same stream");
        assert_eq!(a.at(0), b.at(0));
    }

    #[test]
    fn distinct_phases_do_not_collide_on_counter_zero() {
        // The collision R-RNG-COORD names: two draw sites in the same (locus, tick)
        // stream must not return the same first draw.
        let seed = 0xC0FFEE;
        let perception = DrawKey::entity(42, 9, Phase::PERCEPTION).rng(seed).at(0);
        let gossip = DrawKey::entity(42, 9, Phase::GOSSIP).rng(seed).at(0);
        let language = DrawKey::entity(42, 9, Phase::LANGUAGE).rng(seed).at(0);
        assert_ne!(perception, gossip);
        assert_ne!(gossip, language);
        assert_ne!(perception, language);
        // The developmental-environment phase (0x16) must not alias the mate-choice phase
        // (0x15) it neighbours, nor the perception phase, on counter zero.
        let development = DrawKey::entity(42, 9, Phase::DEVELOPMENT).rng(seed).at(0);
        let mate_choice = DrawKey::entity(42, 9, Phase::MATE_CHOICE).rng(seed).at(0);
        assert_ne!(development, mate_choice);
        assert_ne!(development, perception);
        // The two transmission-substrate phases (0x17 TRANSMIT, 0x18 KNOW_LOSS) must be distinct
        // from each other and from the gossip and drift phases they conceptually neighbour, so a
        // transmission copy, a forgetting roll, a gossip exchange, and a sound-change drift never
        // alias on counter zero.
        let transmit = DrawKey::entity(42, 9, Phase::TRANSMIT).rng(seed).at(0);
        let know_loss = DrawKey::entity(42, 9, Phase::KNOW_LOSS).rng(seed).at(0);
        let drift = DrawKey::entity(42, 9, Phase::DRIFT).rng(seed).at(0);
        assert_ne!(transmit, know_loss);
        assert_ne!(transmit, gossip);
        assert_ne!(transmit, drift);
        assert_ne!(know_loss, gossip);
        assert_ne!(know_loss, drift);
        assert_ne!(transmit, development);
        // The two phase values are the next two free after DEVELOPMENT (0x16), and distinct.
        assert_eq!(Phase::TRANSMIT, Phase(0x17));
        assert_eq!(Phase::KNOW_LOSS, Phase(0x18));
        assert_ne!(Phase::TRANSMIT, Phase::GOSSIP);
        assert_ne!(Phase::KNOW_LOSS, Phase::DRIFT);
        // The belief-lift dispersion phase (0x19) is the next free value after KNOW_LOSS (0x18)
        // and must not alias the transmission phases it neighbours nor the perception phase, on
        // counter zero, so a belief-lift dispersion, a transmission copy, a forgetting roll, and a
        // perception roll never collide.
        let belief_lift = DrawKey::entity(42, 9, Phase::BELIEF_LIFT).rng(seed).at(0);
        assert_eq!(Phase::BELIEF_LIFT, Phase(0x19));
        assert_ne!(belief_lift, transmit);
        assert_ne!(belief_lift, know_loss);
        assert_ne!(belief_lift, perception);
        assert_ne!(belief_lift, development);
        assert_ne!(Phase::BELIEF_LIFT, Phase::TRANSMIT);
        assert_ne!(Phase::BELIEF_LIFT, Phase::KNOW_LOSS);
        // The institution-crystallization phase (0x1A) is the next free value after BELIEF_LIFT
        // (0x19) and must not alias the belief-lift or perception phases it neighbours, on counter
        // zero, so a crystallization tie-break, a belief-lift dispersion, and a perception roll
        // never collide.
        let crystallize = DrawKey::entity(42, 9, Phase::CRYSTALLIZE).rng(seed).at(0);
        assert_eq!(Phase::CRYSTALLIZE, Phase(0x1A));
        assert_ne!(crystallize, belief_lift);
        assert_ne!(crystallize, perception);
        assert_ne!(crystallize, know_loss);
        assert_ne!(Phase::CRYSTALLIZE, Phase::BELIEF_LIFT);
        assert_ne!(Phase::CRYSTALLIZE, Phase::MORTALITY);
        // The morphogen developmental-growth phase (0x1B) is the next free value after CRYSTALLIZE
        // (0x1A); a growth branch draw must not alias the crystallization, genesis, or development
        // streams it neighbours on counter zero, so growth stays its own reproducible stream.
        let morphogen = DrawKey::entity(42, 9, Phase::MORPHOGEN).rng(seed).at(0);
        assert_eq!(Phase::MORPHOGEN, Phase(0x1B));
        assert_ne!(morphogen, crystallize);
        assert_ne!(morphogen, development);
        assert_ne!(Phase::MORPHOGEN, Phase::CRYSTALLIZE);
        assert_ne!(Phase::MORPHOGEN, Phase::GENESIS);
    }

    #[test]
    fn distinct_slots_do_not_collide_on_counter_zero() {
        // Two distinct rolls in one phase, separated only by slot, must not alias.
        let seed = 0x5EED;
        let s0 = DrawKey::entity(1, 1, Phase::INNOVATE)
            .slot(0)
            .rng(seed)
            .at(0);
        let s1 = DrawKey::entity(1, 1, Phase::INNOVATE)
            .slot(1)
            .rng(seed)
            .at(0);
        let s2 = DrawKey::entity(1, 1, Phase::INNOVATE)
            .slot(2)
            .rng(seed)
            .at(0);
        assert_ne!(s0, s1);
        assert_ne!(s1, s2);
        assert_ne!(s0, s2);
    }

    #[test]
    fn the_tick_separates_streams() {
        // The coordinate the old (entity, phase) key omitted: two ticks differ.
        let seed = 1;
        let t0 = DrawKey::entity(3, 0, Phase::DRIFT).rng(seed).at(0);
        let t1 = DrawKey::entity(3, 1, Phase::DRIFT).rng(seed).at(0);
        assert_ne!(t0, t1, "the tick must change the stream");
    }

    #[test]
    fn absent_is_distinct_from_a_present_zero() {
        // The degrade rule: an absent secondary locus must not alias secondary locus 0.
        let seed = 99;
        let absent = DrawKey::entity(5, 2, Phase::COIN).rng(seed).at(0);
        let zero = DrawKey::pair(5, 0, 2, Phase::COIN).rng(seed).at(0);
        assert_ne!(absent, zero, "ABSENT must differ from a present zero");
    }

    #[test]
    fn region_and_locus_order_matters() {
        let seed = 4;
        let a = DrawKey::pair(1, 2, 3, Phase::PERCEPTION).rng(seed).at(0);
        let swapped = DrawKey::pair(2, 1, 3, Phase::PERCEPTION).rng(seed).at(0);
        assert_ne!(a, swapped, "the two loci are positional, not a set");
    }
}
