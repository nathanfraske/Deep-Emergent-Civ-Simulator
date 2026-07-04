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
use crate::breeding::BreedingSystemId;
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
    /// per-context environment is a follow-on. This is the cohort-shared centre; the per-member
    /// spread around it is `environment_variance`.
    pub environment: Fixed,
    /// The half-width of the per-being developmental-environment deviation (the V_E spread, design
    /// Part 25.6): the environmental-variance half of narrow-sense heritability. At expression each
    /// member draws a mean-zero symmetric offset in `[-environment_variance, +environment_variance)`
    /// (see [`crate::world`]'s development draw under [`civsim_core::Phase::DEVELOPMENT`]) and adds it
    /// to `environment`, so two members of one cohort express different minds from one genome-and-
    /// environment rule and V_E is positive rather than identically zero. The offset is symmetric,
    /// so it authors variance without shifting the cohort mean (Principle 9). A per-race owner datum,
    /// reserved (`genome.environment_variance`): the interim [`Fixed::ZERO`] reproduces the current
    /// homogeneous world bit for bit. The mechanism is fixed Rust; this half-width is data
    /// (Principle 11).
    pub environment_variance: Fixed,
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
    /// The race's breeding system, by id into the world's [`crate::breeding::BreedingSystemRegistry`]
    /// (design Part 25, R-REPRO). It names how many sex classes the race carries and how a genotype
    /// assigns to one, so a race's sex is a gene-fed phenotype read off its sex-determination locus,
    /// and the number of mating types is per-race data rather than a closed binary enum (Principle
    /// 8, Principle 11). Defaults to [`BreedingSystemId`] zero (the conventional first-registered
    /// system) in [`Race::new`]; set another with [`Race::with_breeding`]. An id the registry does
    /// not hold falls back to a single class, so a world with no registered system authors no ratio.
    pub breeding: BreedingSystemId,
    /// The race's articulation and hearing parameters (design Part 33.3, R-SENSORIUM): the per-race
    /// data the phonetic pipeline reads to bend the shared base sound geometry to this race's own body
    /// ([`crate::langmod::articulated_geometry`]). `None` until declared, so a race with no
    /// articulation derives no phonetic form system (the fail-quiet-until-declared convention);
    /// [`Race::with_articulation`] sets it. Two races diverge in their phonetics from this data alone
    /// through one kernel, never a `RaceId` branch (Principle 9).
    pub articulation: Option<Articulation>,
}

/// A race's articulation and hearing parameters (design Part 33.3): the two per-race scalars the
/// phonetic pipeline reads. The base sound geometry (the resonator length of each candidate feature
/// value) is shared universal physics; these two scalars bend it to the race's own body, so two races
/// diverge in the sounds they produce and discriminate from this data alone through one kernel, never
/// a `RaceId` branch (Principle 9). The mechanism is fixed Rust; these values are data (Principle 11),
/// reserved fail-loud with basis, never fabricated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Articulation {
    /// The vocal-tract scale: a multiplier on the base resonator lengths, so a larger tract gives
    /// longer resonators and lower formants (the tube-resonance law, frequency proportional to the
    /// sound speed over the length). RESERVED with basis (the race's resonating-cavity size relative
    /// to the base geometry). AUTHORED NOW, DERIVABLE LATER: this should eventually derive from the
    /// race's body plan (the resonating-cavity size, the anatomy tier / R-ORGAN-FLUX), so it is an
    /// interim per-race lever rather than a permanent one.
    pub vocal_tract_scale: Fixed,
    /// The hearing resolution: the just-noticeable frequency difference the race discriminates voice
    /// at, the sensorium resolution the perceptual geometry reads (a SMALLER value is a sharper ear).
    /// RESERVED with basis (the race's auditory frequency-discrimination threshold).
    pub hearing_resolution: Fixed,
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
        environment_variance: Fixed,
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
            environment_variance,
            lifespan_years,
            maturity_years,
            breeding: BreedingSystemId(0),
            articulation: None,
        }
    }

    /// Set the race's articulation and hearing parameters (a builder over [`Race::new`]). Until set,
    /// the race derives no phonetic form system (the fail-quiet-until-declared convention). The
    /// mechanism is fixed; the vocal-tract scale and hearing resolution are per-race data (Principle
    /// 11), reserved fail-loud with basis.
    pub fn with_articulation(mut self, articulation: Articulation) -> Self {
        self.articulation = Some(articulation);
        self
    }

    /// Set the race's breeding system by id (a builder over [`Race::new`]). The mechanism is fixed;
    /// which system a race breeds under is data (Principle 11).
    pub fn with_breeding(mut self, breeding: BreedingSystemId) -> Self {
        self.breeding = breeding;
        self
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
