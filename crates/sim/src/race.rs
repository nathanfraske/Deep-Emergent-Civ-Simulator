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
use crate::genome::{GenePool, GeneSet, GeneticScheme, ReproductionMode};
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
    /// The genetic scheme the race reproduces under (design 25.2): the reproduction mode, the
    /// linkage map, and the mutation rate. Births read it; the ploidy a dawn member is promoted
    /// at follows from its reproduction mode (see [`Race::ploidy`]).
    pub scheme: GeneticScheme,
    /// The innate belief disposition seeded into every dawn member of the race (Part 28). All
    /// members of a race share this seed at the dawn; per-member divergence is the later
    /// inheritance and enculturation work.
    pub intrinsic: IntrinsicBeliefs,
    /// The non-genetic offset added when a member's cognition is expressed from its genes (the
    /// nurture baseline). At the dawn this is the race's environmental cognition floor; richer
    /// per-context environment is a follow-on.
    pub environment: Fixed,
    /// The race's natural lifespan in life-cadence steps (design Part 20, R-AGING), an owner-set
    /// per-race datum (design.md:1593). It normalizes a being's raw age into the life fraction a
    /// life-hazard curve is evaluated at (see [`Race::life_fraction`]), so a long-lived and a
    /// short-lived race face the same curve on their own scale from this one number, never a
    /// per-race code branch (Principle 9). A plain count with no formula: the owner sets it.
    pub lifespan_years: u32,
    /// The race's age of maturity in life-cadence steps, the same units as `lifespan_years`, an
    /// owner-set per-race datum (design.md:1594). It normalizes raw age into the maturation
    /// fraction (see [`Race::maturation_fraction`]) and gates [`Race::is_mature`], so when a being
    /// crosses into adulthood is per-race data, not a hardcoded threshold. A plain count, no
    /// formula: the owner sets it.
    pub maturity_years: u32,
}

impl Race {
    /// A race record.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: RaceId,
        genes: GeneSet,
        pool: GenePool,
        scheme: GeneticScheme,
        intrinsic: IntrinsicBeliefs,
        environment: Fixed,
        lifespan_years: u32,
        maturity_years: u32,
    ) -> Self {
        Race {
            id,
            genes,
            pool,
            scheme,
            intrinsic,
            environment,
            lifespan_years,
            maturity_years,
        }
    }

    /// The ploidy a member is promoted or born at, derived from the reproduction mode: two for
    /// a sexual diploid race, one for a haploid or clonal one.
    pub fn ploidy(&self) -> usize {
        match self.scheme.reproduction {
            ReproductionMode::SexualDiploid => 2,
            ReproductionMode::Haploid | ReproductionMode::Clonal => 1,
        }
    }

    /// The maturation fraction of a being of this race at `age` life-cadence steps: [`Fixed::ZERO`]
    /// at birth, rising linearly to [`Fixed::ONE`] at `maturity_years` and saturating there
    /// (design Part 20). A race whose `maturity_years` is zero (mature from birth) reads
    /// [`Fixed::ONE`] at any age, which also guards the ratio against a zero denominator, since
    /// [`Fixed::from_ratio`] panics on a zero divisor. Age is capped at `maturity_years` before the
    /// ratio, so the result is always in the unit interval and the division never overflows. The
    /// fraction is shaped only by the per-race `maturity_years` datum, so two races diverge here
    /// through the one function rather than a per-race branch (Principle 9).
    pub fn maturation_fraction(&self, age: u32) -> Fixed {
        if self.maturity_years == 0 {
            return Fixed::ONE;
        }
        let capped = age.min(self.maturity_years);
        Fixed::from_ratio(capped as i64, self.maturity_years as i64)
    }

    /// The life fraction of a being of this race at `age` life-cadence steps: [`Fixed::ZERO`] at
    /// birth, rising linearly to [`Fixed::ONE`] at `lifespan_years` and saturating there, the
    /// race-normalized age a life-hazard curve is evaluated at (design Part 20, R-AGING). A race
    /// whose `lifespan_years` is zero reads [`Fixed::ONE`] at any age, which also guards the ratio
    /// against a zero denominator. Age is capped at `lifespan_years` before the ratio, so the
    /// result is always in the unit interval and the division never overflows. The fraction is
    /// shaped only by the per-race `lifespan_years` datum, so a long-lived and a short-lived race
    /// map the same hazard curve onto their own scale through the one function (Principle 9).
    pub fn life_fraction(&self, age: u32) -> Fixed {
        if self.lifespan_years == 0 {
            return Fixed::ONE;
        }
        let capped = age.min(self.lifespan_years);
        Fixed::from_ratio(capped as i64, self.lifespan_years as i64)
    }

    /// Whether a being of this race is mature at `age`: at or past `maturity_years` (design Part
    /// 20). A race whose `maturity_years` is zero is mature at any age, including birth. The gate
    /// reads only the per-race `maturity_years` datum, never a hardcoded threshold or a per-race
    /// code branch (Principle 9).
    pub fn is_mature(&self, age: u32) -> bool {
        age >= self.maturity_years
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
