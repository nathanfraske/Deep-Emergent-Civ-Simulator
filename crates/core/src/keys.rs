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
//! - A registered [`Phase`] set, so a phase id is assigned here once rather than as a
//!   hand-written magic number at the call site. Two draw sites cannot silently share
//!   a phase number, which is the collision R-RNG-COORD names.
//! - A fixed canonical coordinate order ([`DrawKey`]), so every site folds the same
//!   fields in the same places (region, locus, secondary locus, tick, phase, slot).
//!   The tick is always present, which the older `(entity, phase)` coordinate omitted.
//! - Explicit draw-site namespacing via [`DrawKey::slot`], so two distinct rolls in one
//!   (locus, phase, tick) stream do not collide on counter zero.
//! - A degrade rule: a coordinate that does not apply folds as [`ABSENT`], distinct
//!   from a present zero, so "no secondary locus" never aliases "secondary locus 0".
//!
//! The phase registry is engine mechanics, the RNG-core exemption Principle 11 grants,
//! so it is Rust rather than data. The keying stays integer and counter-based: a draw
//! is still `key.rng(seed).at(counter)`, a pure function of its coordinate.

use crate::rng::Rng;

/// A registered simulation phase. Assigned here once; a new draw site adds a constant
/// rather than inventing a magic number at the call site (R-RNG-COORD). The numeric
/// values are arbitrary but must stay distinct and stable across releases, since they
/// are folded into canonical streams.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub struct Phase(pub u32);

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
    /// A worldgen lattice draw (terrain genesis). Genesis-time, so its draws carry no
    /// tick; the field being sampled is the draw-site slot and the octave the region.
    pub const WORLDGEN: Phase = Phase(0x10);
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

#[cfg(test)]
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
