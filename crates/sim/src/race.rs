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

//! What a race starts with at the dawn of sentience, and the band placements that seed it
//! onto the world (design Part 28, Part 20).
//!
//! A race begins with exactly two things, both data (Part 40): its intrinsic capabilities,
//! carried here as the genetic substrate (a [`GeneSet`] and the aggregate allele-frequency
//! [`GenePool`] a member is sampled from), and its intrinsic beliefs (an [`IntrinsicBeliefs`]
//! seed: the innate value profile, axioms, and epistemic stance, Part 28). A [`Race`] bundles
//! these so the dawn seeding can draw a member's genome from the pool, express its mind from
//! the genes, and seed its innate disposition, all from one per-race record. Everything else
//! a race becomes (language, technique, society, belief) is developed from there by the
//! simulation, not given.
//!
//! This is the convergence point of the deep being model: the map (where bands land), the
//! genome (what a member inherits and expresses), the value substrate (Part 21), and the
//! axiom kernel (Part 28) first run together when [`crate::world::World::seed_dawn_populations`]
//! reads these records.

use civsim_core::Fixed;

use crate::axiom::IntrinsicBeliefs;
use crate::genome::{GenePool, GeneSet};
use crate::value::RaceId;
use crate::world::PlaceId;

/// A sentient race as it stands at the dawn: its genetic substrate and its innate belief
/// disposition, all per-race data (Principle 11). The mechanism that seeds and expresses a
/// member is fixed; a race differs only in this record.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Race {
    /// The race's identifier.
    pub id: RaceId,
    /// The genes the race carries and what each reaches (the expression substrate, Part 25).
    pub genes: GeneSet,
    /// The aggregate allele-frequency pool a dawn member is sampled from (Hardy-Weinberg
    /// promotion, design 25.8), so members of a band differ genetically from one draw rule.
    pub pool: GenePool,
    /// The ploidy a member is promoted at (two for a sexual diploid race, one for a haploid or
    /// clonal one), matching the race's reproduction mode.
    pub ploidy: usize,
    /// The innate belief disposition seeded into every dawn member of the race (Part 28). All
    /// members of a race share this seed at the dawn; per-member divergence is the later
    /// inheritance and enculturation work.
    pub intrinsic: IntrinsicBeliefs,
    /// The non-genetic offset added when a member's cognition is expressed from its genes (the
    /// nurture baseline). At the dawn this is the race's environmental cognition floor; richer
    /// per-context environment is a follow-on.
    pub environment: Fixed,
}

impl Race {
    /// A race record.
    pub fn new(
        id: RaceId,
        genes: GeneSet,
        pool: GenePool,
        ploidy: usize,
        intrinsic: IntrinsicBeliefs,
        environment: Fixed,
    ) -> Self {
        Race {
            id,
            genes,
            pool,
            ploidy,
            intrinsic,
            environment,
        }
    }
}

/// A dawn band placement: which race, where on the map, and how many members. The dawn
/// replaces the abstract civilization-placement step of the old worldgen pass (design Part
/// 28): worldgen builds the natural world and supplies the habitable places, and a band spec
/// seeds a proto-population of a race at sentience onto one of them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BandSpec {
    /// The race this band belongs to.
    pub race: RaceId,
    /// The place the band is seeded onto.
    pub place: PlaceId,
    /// How many members the band starts with.
    pub members: usize,
}
