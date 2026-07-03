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

//! Emergent mate choice: the direction of assortment is a consequence of physics, never
//! authored (R-REPRO; design Parts 25, 21, 28, 8.4).
//!
//! A being choosing a mate may prefer a similar mate (homophily, positive assortative
//! mating), a dissimilar one (heterophily, negative or disassortative mating), or have no
//! preference. The owner's question was whether that DIRECTION can be emergent rather than an
//! authored per-race lever. The population-genetics literature answers cleanly: direction is
//! an output of selection, not a species constant. Whenever like-with-like offspring are
//! fitter (reproductive incompatibility, unfit hybrids), positive assortment is favoured;
//! whenever unlike-with-unlike offspring are fitter (heterozygote advantage, inbreeding
//! cost), disassortment is favoured; when offspring fitness is flat in parental similarity,
//! random mating is favoured (Otto, Servedio and Nuismer 2008; Maisonneuve et al 2021). This
//! is the R-BEHAVIOR-EVOLVE stance (record 62.19) applied to mate choice: the physics is an
//! authored input, the behaviour is a selected consequence.
//!
//! This module is the prototype-in-isolation proof of that claim. A [`MatePreference`] is a
//! reaction norm over a candidate feature (here the genetic distance between chooser and
//! candidate), whose weight sign is the direction: negative prefers the near candidate
//! (homophily), positive the far (heterophily), zero is indifferent. The mechanism
//! ([`genetic_distance`], [`MatePreference::preference`], [`choose`]) authors no direction:
//! it is symmetric in near versus far. What sets the direction is the authored offspring
//! physics, the map from parental genetic distance to offspring fitness, exactly the
//! genotype-to-viability category the physics floor and the Dobzhansky-Muller
//! [`crate::genome::IncompatibilityTable`] already author (Principle 9 permits authored
//! physics; it forbids an authored behavioural outcome).
//!
//! What the tests below establish, stated precisely. First, the mechanism is sign-agnostic:
//! the same byte-identical [`MatePreference`] and [`choose`] favour homophily under one
//! offspring physics and heterophily under its mirror, so the direction cannot live in the
//! mechanism (the swap-gradient test, the R-EVOLVE-STEER anti-steering invariant, a
//! non-circular refutation). Second, under an authored offspring physics that is monotone in
//! parental genetic distance (the inbreeding-depression and heterosis axis, a real biological
//! relationship), selection favours the sign the physics rewards. What the tests do NOT claim
//! to prove, because a distance-monotone proxy cannot: emergence through the engine's real
//! offspring-genotype physics. The Dobzhansky-Muller [`crate::genome::IncompatibilityTable`]
//! is a step function of WHICH complementary alleles a cross unites, not of the scalar
//! distance, so hybrid viability is non-monotone in distance (a near cross can be inviable and
//! a far cross viable, shown in `hybrid_viability_is_non_monotone_in_genetic_distance`). That
//! is a positive design finding rather than a hole: a distance-only preference handles the
//! inbreeding-heterosis axis but not the incompatibility axis, so the resolved feature set
//! must also carry a prospective-hybrid-viability cue (which the scorer computes from
//! [`crate::genome::IncompatibilityTable::hybrid_outcome`]).
//!
//! Third, the genome-derived offspring fitness is now built, closing the proxy gap: the
//! fitness is read off the offspring genotype the engine's own `GeneticScheme::reproduce`
//! forms, not authored as a function of distance. [`offspring_heterozygosity`] (the heterosis
//! axis) rises when a heterophilic chooser picks the farther mate, and [`hybrid_viability`]
//! (the incompatibility axis, via `hybrid_outcome`) shows the viability-aware
//! [`most_viable_mate`] avoiding a Dobzhansky-Muller cross a distance preference walks into.
//!
//! Fourth, the selection loop is now built: [`evolve_preference_weight`] runs truncation
//! selection on the genome-derived offspring fitness from RANDOM founder weights with no bias,
//! and produces a heterophilic weight under overdominance and a homophilic one when homozygous
//! offspring are fitter. So the direction is not merely favoured in a gradient, it is what
//! selection converges to, from no bias, on the engine's own genetics.
//!
//! Fifth, the feature-weighted preference over BOTH axes is now built: a [`FeaturedPreference`]
//! carries a distance weight (the heterosis and inbreeding axis) and a prospective-viability
//! weight (the incompatibility axis a distance cue cannot carry), [`choose_featured`] scores a
//! candidate by both, and [`evolve_featured_preference`] runs the same truncation selection over
//! both weights under a combined genome-derived fitness (heterozygosity times viability, so an
//! offspring must be both heterozygous AND viable to score). When the fittest mate by distance is
//! a Dobzhansky-Muller incompatible cross, selection from random founders discovers to weight the
//! prospective-viability feature, so the feature set the resolved mechanism needs is a selected
//! consequence rather than an authored one.
//!
//! Sixth, the `World::birth` choosing call site is now built ([`crate::world::World::choose_mate`]):
//! a per-being heritable [`MatePreference`] over the distance axis, seeded at the dawn with
//! unbiased variation (symmetric about indifference, so no direction is authored) and inherited
//! at birth by the midparent rule plus a bounded mutation, so which way a being assorts is a
//! heritable trait shaped by differential reproduction rather than a per-race lever. What remains
//! is folding the feature-weighted loop onto the shared `evolve_with` plus `Controller` substrate
//! (once its input registry carries a candidate-percept family) and giving the World choose site
//! a Dobzhansky-Muller table so it can weight the incompatibility axis, not distance alone.
//!
//! Compute: this is the cheap "matching rule / magic trait / one-allele" case (Felsenstein
//! 1981; Servedio et al 2011; Kopp et al 2018), where the mating cue is the fitness-relevant
//! genome itself, so no separate preference dimension must be coevolved and held in linkage
//! against recombination. A preference over an already-computed genome distance is order-1
//! over the existing evolution loop, so emergence is tractable and is the recommendation, not
//! the fallback. Honest follow-ons, each named: the selection loop over the genome-derived fitness
//! is built ([`evolve_preference_weight`], the distance axis); the feature-weighted version over
//! both axes is built as a standalone selection loop ([`evolve_featured_preference`]) and its
//! folding onto the shared `evolve_with` plus `Controller` substrate rides that substrate's input
//! registry carrying a candidate-percept family; the value and axiom distances are added as further
//! features; and the `World::birth` call site now does the choosing under a per-being heritable
//! preference ([`crate::world::World::choose_mate`]), with the incompatibility axis at that site
//! awaiting a Dobzhansky-Muller table on the choose path. The reserved values the resolved
//! mechanism surfaces (none fabricated) are the
//! cost of choosiness, the preference mutation variance, and, for the deep-time pure-frequency
//! fallback only, a per-race assortment-bias scalar.

use crate::genome::{GeneticScheme, Genome, HybridOutcome, IncompatibilityTable};
use civsim_core::{DrawKey, Fixed, Phase};

/// The genetic distance between two genomes over their discrete allele states, in `[0, 1]`:
/// the mean over loci of the allele-sharing distance, `(ploidy - shared) / ploidy`, where
/// `shared` is the size of the multiset intersection of the two genotypes' states at that
/// locus. Zero for identical genotypes, one for genotypes that share no allele at any locus.
/// The individual-genome analogue of [`crate::genome::GenePool::distance`] (the pool-tier mean
/// absolute frequency difference), a pure function of two genomes with no authored physics,
/// genotype-based and blind to haplotype phase. It reads only the discrete allele state (the
/// Mendelian view the [`crate::genome::IncompatibilityTable`] and speciation also key off), not
/// the quantitative `additive` value; a state-and-additive distance is a follow-on. Correct for
/// any allele multiplicity: a shared allele in a non-aligned sorted position is not miscounted
/// as a difference. Panics on a ragged or mismatched genome (both must share a
/// [`crate::genome::GeneSet`], so every haplotype carries the same loci).
pub fn genetic_distance(a: &Genome, b: &Genome) -> Fixed {
    let ploidy = a.haps.len();
    assert_eq!(ploidy, b.haps.len(), "genetic distance needs equal ploidy");
    assert!(ploidy >= 1, "a genome carries at least one haplotype");
    let loci = a.haps[0].alleles.len();
    // Guard every haplotype, not just the first, so a ragged genome is a loud error rather
    // than an out-of-bounds index below.
    for h in 0..ploidy {
        assert_eq!(a.haps[h].alleles.len(), loci, "genome a is ragged");
        assert_eq!(
            b.haps[h].alleles.len(),
            loci,
            "genome b must match a's loci"
        );
    }
    if loci == 0 {
        return Fixed::ZERO;
    }
    let mut diff: i64 = 0;
    let mut sa: Vec<u16> = Vec::with_capacity(ploidy);
    let mut sb: Vec<u16> = Vec::with_capacity(ploidy);
    for locus in 0..loci {
        sa.clear();
        sb.clear();
        for h in 0..ploidy {
            sa.push(a.haps[h].alleles[locus].state.0);
            sb.push(b.haps[h].alleles[locus].state.0);
        }
        sa.sort_unstable();
        sb.sort_unstable();
        // Size of the multiset intersection by a two-pointer merge over the sorted states.
        let (mut i, mut j, mut shared) = (0usize, 0usize, 0usize);
        while i < ploidy && j < ploidy {
            match sa[i].cmp(&sb[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    shared += 1;
                    i += 1;
                    j += 1;
                }
            }
        }
        diff += (ploidy - shared) as i64;
    }
    Fixed::from_ratio(diff, (loci * ploidy) as i64)
}

/// A heritable mate preference as a reaction norm over the candidate genetic distance. The
/// weight sign is the direction of assortment: a negative weight scores the near candidate
/// higher (homophily), a positive weight the far candidate (heterophily), zero is
/// indifferent. This is the minimal one-feature form; the general form is a weight per
/// candidate feature (genetic, value, and axiom distances), the direction still emerging as
/// the sign selection settles on. Nothing here authors the sign; selection over the offspring
/// physics does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MatePreference {
    /// The weight on the candidate genetic distance. Its sign is the emergent direction.
    pub distance_weight: Fixed,
}

impl MatePreference {
    /// A preference with the given distance weight.
    pub fn new(distance_weight: Fixed) -> Self {
        MatePreference { distance_weight }
    }

    /// A homophilic preference (weight -1): the near candidate scores higher.
    pub fn homophilic() -> Self {
        MatePreference::new(Fixed::from_int(-1))
    }

    /// A heterophilic preference (weight +1): the far candidate scores higher.
    pub fn heterophilic() -> Self {
        MatePreference::new(Fixed::from_int(1))
    }

    /// An indifferent preference (weight 0): every candidate scores the same.
    pub fn indifferent() -> Self {
        MatePreference::new(Fixed::ZERO)
    }

    /// The preference this chooser assigns a candidate: the weight times the genetic distance.
    /// Higher is more preferred. A pure, symmetric function: it privileges neither near nor
    /// far except through the sign of the weight, which is itself the selected quantity.
    pub fn preference(&self, chooser: &Genome, candidate: &Genome) -> Fixed {
        self.distance_weight
            .mul(genetic_distance(chooser, candidate))
    }
}

/// Choose the index of the most-preferred candidate for `chooser` under `pref`, ties broken by
/// the lower index so the choice is deterministic and observer-independent (design Part 3).
/// Returns `None` for an empty candidate set. The choice rule is the same for every direction;
/// only the preference weight differs, so the mechanism carries no built-in bias.
pub fn choose(pref: &MatePreference, chooser: &Genome, candidates: &[Genome]) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (pref.preference(chooser, c), i))
        // Highest preference wins; on a tie the lower index is treated as greater so it is the
        // one `max_by` keeps.
        .max_by(|x, y| x.0.cmp(&y.0).then(y.1.cmp(&x.1)))
        .map(|(_, i)| i)
}

/// The realised offspring fitness of a chooser under a preference, a candidate set, and an
/// authored offspring-physics map `fitness_of_distance` (the P9-allowed genotype-to-viability
/// input): the chooser picks a candidate by [`choose`], and its fitness is the physics
/// evaluated at the chosen pairing's genetic distance. Returns [`Fixed::ZERO`] for an empty
/// candidate set. The direction the chooser should carry is not read from here; it is whatever
/// this physics rewards.
pub fn realised_fitness(
    pref: &MatePreference,
    chooser: &Genome,
    candidates: &[Genome],
    fitness_of_distance: fn(Fixed) -> Fixed,
) -> Fixed {
    match choose(pref, chooser, candidates) {
        None => Fixed::ZERO,
        Some(i) => fitness_of_distance(genetic_distance(chooser, &candidates[i])),
    }
}

// --- Genome-derived offspring fitness (the follow-on that closes the proxy gap) ---
//
// The functions above score a pairing through an authored fitness-of-distance map. The
// functions below score the actual offspring GENOTYPE, formed by the engine's own
// `GeneticScheme::reproduce`, so the fitness is real physics on the real genome rather than a
// reduced-form proxy. The two axes the literature names are covered: offspring heterozygosity
// (the heterosis and overdominance axis) and hybrid viability (the Dobzhansky-Muller
// incompatibility axis, which the distance feature cannot carry because it is non-monotone in
// distance).

/// The heterozygosity of a diploid offspring: the fraction of loci where its two haplotypes
/// carry different allele states (the same locus-heterozygous predicate
/// [`crate::genome::GeneSet::express`] uses to apply the dominance deviation). A real genome
/// property, read off a formed offspring, so under overdominance a more heterozygous offspring
/// is a fitter one. Zero for a haploid or empty genome.
pub fn offspring_heterozygosity(offspring: &Genome) -> Fixed {
    if offspring.haps.len() < 2 {
        return Fixed::ZERO;
    }
    let loci = offspring.haps[0].alleles.len();
    if loci == 0 {
        return Fixed::ZERO;
    }
    let mut het: i64 = 0;
    for locus in 0..loci {
        if offspring.haps[0].alleles[locus].state != offspring.haps[1].alleles[locus].state {
            het += 1;
        }
    }
    Fixed::from_ratio(het, loci as i64)
}

/// The viability of a formed offspring against a Dobzhansky-Muller incompatibility table, as a
/// fitness scalar in `[0, 1]`: viable is one, sterile one half, inviable zero
/// ([`crate::genome::IncompatibilityTable::hybrid_outcome`]). Genome-derived offspring fitness
/// on the incompatibility axis, a step function of which complementary alleles the cross
/// unites rather than of the parents' distance.
pub fn hybrid_viability(offspring: &Genome, table: &IncompatibilityTable) -> Fixed {
    match table.hybrid_outcome(offspring) {
        HybridOutcome::Viable => Fixed::ONE,
        HybridOutcome::Sterile => Fixed::from_ratio(1, 2),
        HybridOutcome::Inviable => Fixed::ZERO,
    }
}

/// Choose the candidate whose offspring with `chooser` is most viable against `table`, ties to
/// the lower index; `None` for an empty set. This is the viability-aware choice the
/// incompatibility axis needs: it reads prospective hybrid viability by forming each candidate
/// offspring through `scheme`, the cue a distance-only preference lacks. It is deterministic
/// and observer-independent (the offspring is a pure function of the seed and the parents).
pub fn most_viable_mate(
    chooser: &Genome,
    candidates: &[Genome],
    scheme: &GeneticScheme,
    gene_count: usize,
    table: &IncompatibilityTable,
    seed: u64,
) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let child = scheme.reproduce(chooser, 0, c, i as u64 + 1, gene_count, seed, 0);
            (hybrid_viability(&child, table), i)
        })
        .max_by(|x, y| x.0.cmp(&y.0).then(y.1.cmp(&x.1)))
        .map(|(_, i)| i)
}

/// Evolve the distance weight of a mate preference under a genome-derived offspring fitness,
/// from RANDOM founders, and return the fittest evolved weight (whose sign is the emergent
/// direction). This is the capstone of the emergence claim: the earlier proofs show the
/// mechanism authors no direction and that the physics-favoured direction is fitter; this shows
/// selection, starting from a population with no preference bias, PRODUCES it. Each generation
/// scores every preference by the fitness of the offspring its chooser forms with its picked
/// mate (read off the real offspring genotype via `scheme.reproduce`, so the selection pressure
/// is the engine's own genetics, not an authored fitness-of-distance), keeps the fitter half
/// (truncation, ties to the lower index), and refills with bounded mutants. Deterministic and
/// observer-independent: founders and mutations are counter-keyed on the seed, the lineage, and
/// the generation under [`Phase::MATE_CHOICE`], mirroring the R-BEHAVIOR-EVOLVE selection loop.
///
/// This evolves the one distance weight, so it demonstrates the heterosis and inbreeding axis
/// that distance carries; the incompatibility axis needs the prospective-hybrid-viability cue
/// [`most_viable_mate`] reads, and evolving a feature-weighted preference over both axes on the
/// shared `evolve_with` substrate is the named follow-on.
#[allow(clippy::too_many_arguments)]
pub fn evolve_preference_weight(
    chooser: &Genome,
    candidates: &[Genome],
    scheme: &GeneticScheme,
    gene_count: usize,
    fitness_of_offspring: fn(&Genome) -> Fixed,
    seed: u64,
    pop_size: usize,
    generations: u64,
) -> Fixed {
    assert!(pop_size >= 2, "selection needs at least two lineages");
    // Map a unit draw in [0, 1) to a signed weight in [-1, 1].
    let signed = |u: Fixed| Fixed::from_int(2).mul(u) - Fixed::ONE;
    // The fitness of a preference: its chooser picks a mate, forms an offspring, and the
    // offspring's genome-derived fitness is the score. An empty candidate set scores zero.
    let score = |weight: Fixed| -> Fixed {
        match choose(&MatePreference::new(weight), chooser, candidates) {
            None => Fixed::ZERO,
            Some(i) => {
                let child = scheme.reproduce(
                    chooser,
                    0,
                    &candidates[i],
                    (i as u64) + 1,
                    gene_count,
                    seed,
                    0,
                );
                fitness_of_offspring(&child)
            }
        }
    };

    // Founders: one random weight per lineage.
    let mut pop: Vec<Fixed> = (0..pop_size as u64)
        .map(|lineage| {
            signed(
                DrawKey::entity(lineage, 0, Phase::MATE_CHOICE)
                    .rng(seed)
                    .unit_fixed(0),
            )
        })
        .collect();
    let mut next_lineage = pop_size as u64;

    for g in 0..generations {
        let mut scored: Vec<(Fixed, usize)> = pop
            .iter()
            .enumerate()
            .map(|(i, &w)| (score(w), i))
            .collect();
        // Fitter first, ties to the lower index (deterministic).
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let keep = (pop.len() / 2).max(1);
        let survivors: Vec<Fixed> = scored[..keep].iter().map(|&(_, i)| pop[i]).collect();
        // Next generation: the survivors, then bounded mutants of them until refilled.
        let mut next = survivors.clone();
        let mut s = 0usize;
        while next.len() < pop.len() {
            let parent = survivors[s % survivors.len()];
            let delta = signed(
                DrawKey::entity(next_lineage, g, Phase::MATE_CHOICE)
                    .rng(seed)
                    .unit_fixed(0),
            )
            .mul(Fixed::from_ratio(1, 4));
            next.push((parent + delta).clamp(Fixed::from_int(-1), Fixed::from_int(1)));
            next_lineage += 1;
            s += 1;
        }
        pop = next;
    }

    // The fittest evolved weight, ties to the lower weight for determinism.
    let mut ranked: Vec<(Fixed, Fixed)> = pop.iter().map(|&w| (score(w), w)).collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    ranked[0].1
}

// --- The resolved feature set: a preference over both the distance and the incompatibility
// axes, and selection over it (the feature-weighted follow-on) ---

/// A mate preference over two candidate features: the genetic distance (the heterosis and
/// inbreeding axis) and the prospective hybrid viability (the incompatibility axis a distance
/// feature cannot carry, because viability is non-monotone in distance). The general form the
/// resolved mechanism needs. As with [`MatePreference`], nothing here authors a direction: the
/// two weights are the selected quantities, and their signs and magnitudes are what selection
/// over the offspring physics settles on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FeaturedPreference {
    /// The weight on the candidate genetic distance (its sign is the assortment direction).
    pub distance_weight: Fixed,
    /// The weight on the prospective hybrid viability (a positive weight avoids incompatible
    /// mates).
    pub viability_weight: Fixed,
}

impl FeaturedPreference {
    /// A preference with the given feature weights.
    pub fn new(distance_weight: Fixed, viability_weight: Fixed) -> Self {
        FeaturedPreference {
            distance_weight,
            viability_weight,
        }
    }
}

/// The prospective hybrid viability of a chooser-candidate pairing: form the candidate offspring
/// through `scheme` and read its [`hybrid_viability`] against `table`. The incompatibility-axis
/// feature a featured preference reads before choosing, the cue [`genetic_distance`] cannot
/// supply.
pub fn prospective_viability(
    chooser: &Genome,
    candidate: &Genome,
    scheme: &GeneticScheme,
    gene_count: usize,
    table: &IncompatibilityTable,
    seed: u64,
) -> Fixed {
    let child = scheme.reproduce(chooser, 0, candidate, 1, gene_count, seed, 0);
    hybrid_viability(&child, table)
}

/// Choose the most-preferred candidate under a two-feature preference, ties to the lower index;
/// `None` for an empty set. The score is `distance_weight` times the genetic distance plus
/// `viability_weight` times the prospective viability, so a preference can weight the
/// incompatibility axis the distance feature cannot carry.
#[allow(clippy::too_many_arguments)]
pub fn choose_featured(
    pref: &FeaturedPreference,
    chooser: &Genome,
    candidates: &[Genome],
    scheme: &GeneticScheme,
    gene_count: usize,
    table: &IncompatibilityTable,
    seed: u64,
) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let d = genetic_distance(chooser, c);
            let v = prospective_viability(chooser, c, scheme, gene_count, table, seed);
            (
                pref.distance_weight.mul(d) + pref.viability_weight.mul(v),
                i,
            )
        })
        .max_by(|x, y| x.0.cmp(&y.0).then(y.1.cmp(&x.1)))
        .map(|(_, i)| i)
}

/// Evolve a two-feature mate preference under a combined genome-derived offspring fitness
/// (heterozygosity times viability, so a fit offspring must be both heterozygous AND viable),
/// from random founders. This is the resolved-feature-set demonstration: when the fittest mate
/// by distance is a Dobzhansky-Muller incompatible one (an inviable cross), selection discovers
/// to weight the prospective-viability feature, not distance alone, so the feature set the
/// mechanism needs emerges rather than being authored. Same deterministic truncation-and-mutation
/// discipline as [`evolve_preference_weight`], keyed under [`Phase::MATE_CHOICE`]; returns the
/// fittest evolved preference.
#[allow(clippy::too_many_arguments)]
pub fn evolve_featured_preference(
    chooser: &Genome,
    candidates: &[Genome],
    scheme: &GeneticScheme,
    gene_count: usize,
    table: &IncompatibilityTable,
    seed: u64,
    pop_size: usize,
    generations: u64,
) -> FeaturedPreference {
    assert!(pop_size >= 2, "selection needs at least two lineages");
    let signed = |u: Fixed| Fixed::from_int(2).mul(u) - Fixed::ONE;
    // A preference's fitness: its chooser picks a mate, forms an offspring, and the score is the
    // offspring's heterozygosity times its viability, so an inviable cross scores zero however
    // heterozygous it would have been.
    let score = |pref: FeaturedPreference| -> Fixed {
        match choose_featured(&pref, chooser, candidates, scheme, gene_count, table, seed) {
            None => Fixed::ZERO,
            Some(i) => {
                let child = scheme.reproduce(
                    chooser,
                    0,
                    &candidates[i],
                    (i as u64) + 1,
                    gene_count,
                    seed,
                    0,
                );
                offspring_heterozygosity(&child).mul(hybrid_viability(&child, table))
            }
        }
    };
    // Founders: random (distance_weight, viability_weight) per lineage, two counters in the
    // lineage's stream.
    let mut pop: Vec<FeaturedPreference> = (0..pop_size as u64)
        .map(|lineage| {
            let rng = DrawKey::entity(lineage, 0, Phase::MATE_CHOICE).rng(seed);
            FeaturedPreference::new(signed(rng.unit_fixed(0)), signed(rng.unit_fixed(1)))
        })
        .collect();
    let mut next_lineage = pop_size as u64;

    for g in 0..generations {
        let mut scored: Vec<(Fixed, usize)> = pop
            .iter()
            .enumerate()
            .map(|(i, &p)| (score(p), i))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let keep = (pop.len() / 2).max(1);
        let survivors: Vec<FeaturedPreference> =
            scored[..keep].iter().map(|&(_, i)| pop[i]).collect();
        let mut next = survivors.clone();
        let mut s = 0usize;
        let bound = Fixed::from_ratio(1, 4);
        while next.len() < pop.len() {
            let parent = survivors[s % survivors.len()];
            let rng = DrawKey::entity(next_lineage, g, Phase::MATE_CHOICE).rng(seed);
            let clamp = |w: Fixed| w.clamp(Fixed::from_int(-1), Fixed::from_int(1));
            next.push(FeaturedPreference::new(
                clamp(parent.distance_weight + signed(rng.unit_fixed(0)).mul(bound)),
                clamp(parent.viability_weight + signed(rng.unit_fixed(1)).mul(bound)),
            ));
            next_lineage += 1;
            s += 1;
        }
        pop = next;
    }

    // The fittest evolved preference, ties broken by the weight pair for determinism.
    let mut ranked: Vec<(Fixed, FeaturedPreference)> = pop.iter().map(|&p| (score(p), p)).collect();
    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.distance_weight.cmp(&b.1.distance_weight))
            .then(a.1.viability_weight.cmp(&b.1.viability_weight))
    });
    ranked[0].1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{
        Allele, AlleleState, Haplotype, Incompatibility, IncompatibilityKind, LinkageGroup,
        ReproductionMode, SchemeId,
    };

    /// A sexual-diploid scheme over `loci` loci in one linkage group, at the given per-locus
    /// mutation rate (zero for clean segregation in the offspring-fitness tests).
    fn diploid_scheme(loci: usize, mutation: Fixed) -> GeneticScheme {
        GeneticScheme {
            id: SchemeId(0),
            reproduction: ReproductionMode::SexualDiploid,
            linkage_groups: vec![LinkageGroup {
                loci: (0..loci as u32).collect(),
                recombination: vec![Fixed::ZERO; loci.saturating_sub(1)],
            }],
            mutation_rate: mutation,
            additive_mutation_step: Fixed::ZERO,
            gauss: civsim_core::GaussApprox::default(),
        }
    }

    /// A homozygous diploid genome carrying allele state 1 at the given loci and 0 elsewhere.
    fn one_at(loci: usize, ones: &[usize]) -> Genome {
        let hap = || Haplotype {
            alleles: (0..loci)
                .map(|i| Allele {
                    additive: Fixed::ZERO,
                    state: AlleleState(if ones.contains(&i) { 1 } else { 0 }),
                    origin: 0,
                })
                .collect(),
        };
        Genome {
            scheme: SchemeId(0),
            haps: vec![hap(), hap()],
        }
    }

    /// A diploid genome over `loci` loci where the first `divergent` loci carry allele state 1
    /// on both haplotypes and the rest state 0. Its genetic distance from the all-zero genome
    /// of the same size is `divergent / loci`, so a panel of controlled distances is easy to
    /// build.
    fn genome(loci: usize, divergent: usize) -> Genome {
        let hap = || Haplotype {
            alleles: (0..loci)
                .map(|i| Allele {
                    additive: Fixed::ZERO,
                    state: AlleleState(if i < divergent { 1 } else { 0 }),
                    origin: 0,
                })
                .collect(),
        };
        Genome {
            scheme: SchemeId(0),
            haps: vec![hap(), hap()],
        }
    }

    // The three authored offspring-physics worlds (P9-allowed inputs): the map from parental
    // genetic distance to offspring fitness. Nothing about the mate-choice DIRECTION is
    // authored here; these are genotype-to-viability maps, the same category as a material
    // yield stress or the Dobzhansky-Muller incompatibility outcomes.

    /// Reproductive-incompatibility physics: a dissimilar cross yields a less-fit offspring
    /// (unfit hybrids), so fitness falls with distance.
    fn incompatibility_world(distance: Fixed) -> Fixed {
        Fixed::ONE - distance
    }

    /// Heterosis physics: a dissimilar cross yields a fitter, more heterozygous offspring
    /// (heterozygote advantage), so fitness rises with distance.
    fn heterosis_world(distance: Fixed) -> Fixed {
        distance
    }

    /// Neutral physics: offspring fitness is flat in parental similarity.
    fn neutral_world(_distance: Fixed) -> Fixed {
        Fixed::from_ratio(1, 2)
    }

    #[test]
    fn genetic_distance_is_symmetric_zero_for_identical_and_rises_with_divergence() {
        let base = genome(10, 0);
        assert_eq!(
            genetic_distance(&base, &base),
            Fixed::ZERO,
            "identical is zero"
        );
        let near = genome(10, 1);
        let far = genome(10, 9);
        assert_eq!(
            genetic_distance(&base, &near),
            genetic_distance(&near, &base),
            "distance is symmetric"
        );
        assert!(
            genetic_distance(&base, &near) < genetic_distance(&base, &far),
            "distance rises with divergence"
        );
        assert_eq!(
            genetic_distance(&base, &genome(10, 10)),
            Fixed::ONE,
            "all loci differing is distance one"
        );
    }

    #[test]
    fn genetic_distance_counts_shared_multiallelic_states_correctly() {
        // The multiset-intersection fix: at a single locus, genotype {1,2} versus {2,3} shares
        // allele 2, so the allele-sharing distance is (2 - 1)/2 = 1/2, not the 2/2 a naive
        // aligned-Hamming over the sorted states would give. Guards the multi-allelic regime the
        // real engine reaches by state mutation.
        let one = |s0: u16, s1: u16| Genome {
            scheme: SchemeId(0),
            haps: vec![
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::ZERO,
                        state: AlleleState(s0),
                        origin: 0,
                    }],
                },
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::ZERO,
                        state: AlleleState(s1),
                        origin: 0,
                    }],
                },
            ],
        };
        assert_eq!(
            genetic_distance(&one(1, 2), &one(2, 3)),
            Fixed::from_ratio(1, 2),
            "a shared allele in a non-aligned position is not counted as a difference"
        );
        assert_eq!(
            genetic_distance(&one(1, 2), &one(2, 1)),
            Fixed::ZERO,
            "the same genotype in swapped phase is distance zero"
        );
    }

    #[test]
    fn choose_is_deterministic_with_lower_index_ties() {
        let chooser = genome(10, 0);
        // Two candidates at the same distance: the tie must resolve to the lower index.
        let candidates = vec![genome(10, 3), genome(10, 3)];
        let picked = choose(&MatePreference::heterophilic(), &chooser, &candidates);
        assert_eq!(
            picked,
            Some(0),
            "an equal-preference tie breaks to the lower index"
        );
        assert_eq!(choose(&MatePreference::homophilic(), &chooser, &[]), None);
    }

    #[test]
    fn a_homophile_picks_the_near_and_a_heterophile_the_far_candidate() {
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        assert_eq!(
            choose(&MatePreference::homophilic(), &chooser, &candidates),
            Some(0),
            "the homophile picks the nearest"
        );
        assert_eq!(
            choose(&MatePreference::heterophilic(), &chooser, &candidates),
            Some(2),
            "the heterophile picks the farthest"
        );
    }

    #[test]
    fn the_physics_sets_which_direction_is_favoured_not_the_mechanism() {
        // The SAME preference machinery is scored under two authored offspring-physics worlds
        // that are monotone in parental distance (the inbreeding-heterosis axis). In the
        // incompatibility world the homophile out-reproduces the heterophile; in the heterosis
        // world the ordering flips. This shows the favoured sign follows the physics on that
        // axis; the non-circular refutation that the mechanism authors no sign is the
        // swap-gradient test below.
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        let homo = MatePreference::homophilic();
        let hetero = MatePreference::heterophilic();

        let homo_incompat = realised_fitness(&homo, &chooser, &candidates, incompatibility_world);
        let hetero_incompat =
            realised_fitness(&hetero, &chooser, &candidates, incompatibility_world);
        assert!(
            homo_incompat > hetero_incompat,
            "unfit hybrids favour homophily ({homo_incompat:?} vs {hetero_incompat:?})"
        );

        let homo_heterosis = realised_fitness(&homo, &chooser, &candidates, heterosis_world);
        let hetero_heterosis = realised_fitness(&hetero, &chooser, &candidates, heterosis_world);
        assert!(
            hetero_heterosis > homo_heterosis,
            "heterozygote advantage favours heterophily ({hetero_heterosis:?} vs {homo_heterosis:?})"
        );
    }

    #[test]
    fn the_mechanism_carries_no_near_or_far_bias() {
        // The anti-steering invariant with teeth. A flat-world equality is vacuous (it holds
        // for any choose), so instead assert that the homophile's advantage under
        // incompatibility exactly mirrors the heterophile's advantage under heterosis. The two
        // worlds are mirror gradients about the flat world, so if the mechanism carried a
        // near-or-far bias (a term added to the preference beyond the sign) the two advantages
        // would differ. Equality pins that the sign is the only asymmetry the mechanism has.
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        let homo = MatePreference::homophilic();
        let hetero = MatePreference::heterophilic();

        let homo_adv_incompat =
            realised_fitness(&homo, &chooser, &candidates, incompatibility_world)
                - realised_fitness(&hetero, &chooser, &candidates, incompatibility_world);
        let hetero_adv_heterosis =
            realised_fitness(&hetero, &chooser, &candidates, heterosis_world)
                - realised_fitness(&homo, &chooser, &candidates, heterosis_world);
        assert_eq!(
            homo_adv_incompat, hetero_adv_heterosis,
            "the two directions' advantages mirror exactly, so the mechanism adds no bias"
        );
        // And the flat world confirms the physics, not the mechanism, is the source of any
        // advantage: with neutral physics every direction realises the same fitness. (This
        // alone is vacuous, hence the mirror assertion above; kept as a sanity check.)
        let flat_homo = realised_fitness(&homo, &chooser, &candidates, neutral_world);
        let flat_hetero = realised_fitness(&hetero, &chooser, &candidates, neutral_world);
        assert_eq!(flat_homo, flat_hetero);
    }

    #[test]
    fn the_fittest_preference_sign_follows_the_physics() {
        // Which weight SIGN is fittest is set by the physics: the homophilic sign wins under
        // incompatibility, the heterophilic sign under heterosis. This pins the sign only, not a
        // gradient: realised fitness depends on the weight only through its sign (choose
        // argmaxes weight*distance), so magnitude is irrelevant, which is the honest bound on
        // the claim. The panel puts a MIDDLE-distance candidate at index 0, so the indifferent
        // (weight-0) chooser, which falls to index 0 by the tie-break, picks neither extreme and
        // coincides with neither optimum, keeping both legs probative.
        let chooser = genome(12, 0);
        let candidates = vec![
            genome(12, 4),  // index 0, a middle distance: what indifference picks
            genome(12, 11), // the farthest
            genome(12, 1),  // the nearest
            genome(12, 8),
        ];
        let homo = MatePreference::homophilic();
        let indiff = MatePreference::indifferent();
        let hetero = MatePreference::heterophilic();

        // Under incompatibility the nearest is fittest, so the homophilic sign strictly beats
        // both indifference and heterophily.
        let hi = realised_fitness(&homo, &chooser, &candidates, incompatibility_world);
        assert!(
            hi > realised_fitness(&indiff, &chooser, &candidates, incompatibility_world)
                && hi > realised_fitness(&hetero, &chooser, &candidates, incompatibility_world),
            "the homophilic sign is strictly fittest when hybrids are unfit"
        );
        // Under heterosis the farthest is fittest, so the heterophilic sign strictly beats both.
        let he = realised_fitness(&hetero, &chooser, &candidates, heterosis_world);
        assert!(
            he > realised_fitness(&indiff, &chooser, &candidates, heterosis_world)
                && he > realised_fitness(&homo, &chooser, &candidates, heterosis_world),
            "the heterophilic sign is strictly fittest under heterozygote advantage"
        );
    }

    #[test]
    fn swapping_the_physics_gradient_swaps_the_favoured_direction() {
        // The clean statement of the anti-steering invariant: negating the offspring-physics
        // gradient (incompatibility versus heterosis are mirror gradients about the flat world)
        // swaps which direction wins, with the mechanism untouched. If the mechanism authored a
        // direction, one side would always win regardless of the physics.
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 2), genome(10, 8)];
        let homo = MatePreference::homophilic();
        let hetero = MatePreference::heterophilic();

        let incompat_winner_is_homo =
            realised_fitness(&homo, &chooser, &candidates, incompatibility_world)
                > realised_fitness(&hetero, &chooser, &candidates, incompatibility_world);
        let heterosis_winner_is_homo =
            realised_fitness(&homo, &chooser, &candidates, heterosis_world)
                > realised_fitness(&hetero, &chooser, &candidates, heterosis_world);
        assert!(
            incompat_winner_is_homo != heterosis_winner_is_homo,
            "the winning direction flips with the physics gradient, so the mechanism authors none"
        );
    }

    #[test]
    fn hybrid_viability_is_non_monotone_in_genetic_distance() {
        // The red-team's key insight as a tested result over the real hybrid_outcome physics. A
        // Dobzhansky-Muller incompatibility fires on WHICH complementary alleles a cross unites,
        // not on the parents' scalar genetic distance, so hybrid viability is not monotone in
        // distance: a NEAR cross can be inviable and a FAR cross viable. A distance-only
        // preference therefore cannot handle the incompatibility axis; the resolved feature set
        // must also carry a prospective-hybrid-viability cue, which the scorer reads from
        // hybrid_outcome. This grounds the module in the engine's real genotype physics and
        // bounds what the distance proxy above can claim.
        let table = IncompatibilityTable::with(vec![Incompatibility {
            locus_a: 0,
            state_a: AlleleState(1),
            locus_b: 1,
            state_b: AlleleState(1),
            kind: IncompatibilityKind::Lethal,
        }]);
        // A hybrid carrying one strand from each parent (homozygous parents transmit their
        // identical strand).
        let cross = |p1: &Genome, p2: &Genome| Genome {
            scheme: SchemeId(0),
            haps: vec![p1.haps[0].clone(), p2.haps[0].clone()],
        };

        // Near cross: one parent carries the DM allele at locus 0, the other at locus 1, so the
        // hybrid unites the incompatible pair. They differ at two loci.
        let near = cross(&one_at(10, &[0]), &one_at(10, &[1]));
        // Far cross: one parent carries state 1 at four loci that never include locus 1, the
        // other is all zero, so the hybrid never unites the pair. They differ at four loci.
        let far = cross(&one_at(10, &[0, 2, 3, 4]), &one_at(10, &[]));

        assert!(
            genetic_distance(&one_at(10, &[0]), &one_at(10, &[1]))
                < genetic_distance(&one_at(10, &[0, 2, 3, 4]), &one_at(10, &[])),
            "the near cross has the smaller parental genetic distance"
        );
        assert_eq!(
            table.hybrid_outcome(&near),
            HybridOutcome::Inviable,
            "the near cross unites the incompatible pair and is inviable"
        );
        assert_eq!(
            table.hybrid_outcome(&far),
            HybridOutcome::Viable,
            "the far cross never unites the pair and is viable"
        );
    }

    #[test]
    fn indifference_collapses_to_index_order_not_random_mating() {
        // An honest limit for the full build: a weight-0 preference is indifferent in score, but
        // choose must break the all-way tie somehow, and it breaks to the lowest index. So in a
        // candidate list ordered by distance a weight-0 chooser deterministically picks index 0,
        // which reads as homophily (nearest) or heterophily (farthest) purely from the list
        // order. Behavioural neutrality is a property of candidate presentation, not of choose:
        // the resolved build must present candidates in an order decorrelated from distance (or
        // break the all-tie case with a counter-keyed draw).
        let chooser = genome(10, 0);
        let ascending = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        assert_eq!(
            choose(&MatePreference::indifferent(), &chooser, &ascending),
            Some(0),
            "indifference falls to index 0, the nearest in an ascending panel"
        );
        let descending = vec![genome(10, 9), genome(10, 5), genome(10, 1)];
        assert_eq!(
            choose(&MatePreference::indifferent(), &chooser, &descending),
            Some(0),
            "and the farthest in a descending panel: the order, not the preference, decides"
        );
    }

    #[test]
    fn heterophily_raises_offspring_heterozygosity_measured_on_the_formed_genome() {
        // The follow-on that closes the proxy gap on the heterosis axis: the fitness is read off
        // the offspring GENOTYPE formed by reproduce, not authored as a function of distance. A
        // heterophilic chooser (picks the farther mate) yields a more heterozygous offspring than
        // a homophilic one (picks the nearer), so under overdominance selection favours
        // heterophily, with the fitness grounded in the engine's own genetics.
        let scheme = diploid_scheme(10, Fixed::ZERO); // no mutation: clean segregation
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];

        let near_i = choose(&MatePreference::homophilic(), &chooser, &candidates).unwrap();
        let far_i = choose(&MatePreference::heterophilic(), &chooser, &candidates).unwrap();
        let homo_child = scheme.reproduce(&chooser, 1, &candidates[near_i], 2, 10, 0x5EED, 0);
        let hetero_child = scheme.reproduce(&chooser, 1, &candidates[far_i], 3, 10, 0x5EED, 0);

        assert!(
            offspring_heterozygosity(&hetero_child) > offspring_heterozygosity(&homo_child),
            "the heterophile's offspring is more heterozygous ({:?} vs {:?})",
            offspring_heterozygosity(&hetero_child),
            offspring_heterozygosity(&homo_child)
        );
    }

    #[test]
    fn a_distance_preference_cannot_avoid_an_incompatible_near_mate_but_viability_choice_can() {
        // The red-team's insight built out over the real reproduce and hybrid_outcome physics. On
        // the incompatibility axis a homophilic (distance) preference picks the NEAR mate, which
        // here is the incompatible one, so its offspring is inviable; the viability-aware choice
        // (which reads prospective hybrid viability, the cue the distance feature lacks) picks the
        // farther, compatible mate. This is why the resolved feature set must carry a
        // prospective-hybrid-viability cue, not distance alone.
        let scheme = diploid_scheme(10, Fixed::ZERO);
        let table = IncompatibilityTable::with(vec![Incompatibility {
            locus_a: 0,
            state_a: AlleleState(1),
            locus_b: 1,
            state_b: AlleleState(1),
            kind: IncompatibilityKind::Lethal,
        }]);
        let chooser = one_at(10, &[0]); // carries the DM allele at locus 0
        let near = one_at(10, &[0, 1]); // the complementary allele at locus 1: a near, incompatible mate
        let far = one_at(10, &[0, 2, 3, 4, 5]); // far, but never carries state 1 at locus 1: compatible
        let candidates = vec![near, far];
        assert!(
            genetic_distance(&chooser, &candidates[0]) < genetic_distance(&chooser, &candidates[1]),
            "the incompatible mate is the nearer one"
        );

        // The homophile picks the near mate and forms an inviable offspring.
        let homo_i = choose(&MatePreference::homophilic(), &chooser, &candidates).unwrap();
        let homo_child = scheme.reproduce(&chooser, 1, &candidates[homo_i], 2, 10, 0xD3, 0);
        assert_eq!(
            hybrid_viability(&homo_child, &table),
            Fixed::ZERO,
            "the distance preference cannot avoid the incompatible near mate"
        );

        // The viability-aware choice picks the compatible mate and forms a viable offspring.
        let via_i = most_viable_mate(&chooser, &candidates, &scheme, 10, &table, 0xD3).unwrap();
        let via_child = scheme.reproduce(&chooser, 1, &candidates[via_i], 2, 10, 0xD3, 0);
        assert_eq!(
            hybrid_viability(&via_child, &table),
            Fixed::ONE,
            "the viability-aware choice avoids the incompatibility"
        );
    }

    // Two genome-derived offspring-fitness worlds for the selection loop, each a fn pointer
    // over the formed offspring: overdominance rewards a heterozygous offspring, local
    // adaptation (homozygous advantage) rewards a homozygous one.
    fn overdominance_fitness(offspring: &Genome) -> Fixed {
        offspring_heterozygosity(offspring)
    }
    fn homozygous_advantage_fitness(offspring: &Genome) -> Fixed {
        Fixed::ONE - offspring_heterozygosity(offspring)
    }

    #[test]
    fn selection_produces_heterophily_under_overdominance_from_random_founders() {
        // The capstone: from a population of RANDOM preference weights with no bias, selection on
        // the genome-derived offspring fitness produces a heterophilic weight when a heterozygous
        // offspring is fitter (overdominance). The direction is not authored and not merely
        // favoured in a gradient, it is what selection converges to.
        let scheme = diploid_scheme(10, Fixed::ZERO);
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        let evolved = evolve_preference_weight(
            &chooser,
            &candidates,
            &scheme,
            10,
            overdominance_fitness,
            0xC0FFEE,
            16,
            8,
        );
        assert!(
            evolved > Fixed::ZERO,
            "selection produces a heterophilic weight under overdominance ({evolved:?})"
        );
    }

    #[test]
    fn selection_produces_homophily_when_homozygous_offspring_are_fitter() {
        // The mirror: reverse the offspring fitness (a homozygous offspring is fitter, the local-
        // adaptation regime) and selection over the same machinery produces a homophilic weight.
        // The direction tracks the physics, from random founders, with nothing authored.
        let scheme = diploid_scheme(10, Fixed::ZERO);
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        let evolved = evolve_preference_weight(
            &chooser,
            &candidates,
            &scheme,
            10,
            homozygous_advantage_fitness,
            0xC0FFEE,
            16,
            8,
        );
        assert!(
            evolved < Fixed::ZERO,
            "selection produces a homophilic weight when homozygous offspring are fitter ({evolved:?})"
        );
    }

    #[test]
    fn the_selection_loop_replays_bit_identically() {
        // Determinism (Principle 3): the whole selection run is a pure function of the seed.
        let scheme = diploid_scheme(10, Fixed::ZERO);
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        let a = evolve_preference_weight(
            &chooser,
            &candidates,
            &scheme,
            10,
            overdominance_fitness,
            0x1234,
            12,
            6,
        );
        let b = evolve_preference_weight(
            &chooser,
            &candidates,
            &scheme,
            10,
            overdominance_fitness,
            0x1234,
            12,
            6,
        );
        assert_eq!(a, b, "same seed, same evolved weight");
    }

    #[test]
    fn selection_weights_the_viability_feature_when_the_fittest_by_distance_mate_is_incompatible() {
        // The resolved feature set emerging: the fitness rewards a heterozygous AND viable
        // offspring (heterozygosity times viability), and the farthest mate (the one a distance
        // heterophile would pick) is Dobzhansky-Muller incompatible, so a distance-only
        // preference forms an inviable, zero-fitness offspring. From random founders over both
        // weights, selection discovers to weight the prospective-viability feature, and its
        // chosen offspring is viable. Nothing authored the feature weighting; selection found it.
        let scheme = diploid_scheme(10, Fixed::ZERO);
        let table = IncompatibilityTable::with(vec![Incompatibility {
            locus_a: 0,
            state_a: AlleleState(1),
            locus_b: 1,
            state_b: AlleleState(1),
            kind: IncompatibilityKind::Lethal,
        }]);
        let chooser = one_at(10, &[0]); // carries the DM allele at locus 0
                                        // The farthest candidate carries the complementary allele at locus 1: incompatible.
        let far_incompatible = one_at(10, &[1, 2, 3, 4, 5, 6, 7]);
        // A compatible mate at a moderate distance (more heterozygous than the near one).
        let mid_compatible = one_at(10, &[2, 3, 4]);
        let near_compatible = one_at(10, &[2]);
        let candidates = vec![far_incompatible, mid_compatible, near_compatible];

        let evolved =
            evolve_featured_preference(&chooser, &candidates, &scheme, 10, &table, 0xF00D, 24, 12);
        assert!(
            evolved.viability_weight > Fixed::ZERO,
            "selection weights the prospective-viability feature ({evolved:?})"
        );
        let pick =
            choose_featured(&evolved, &chooser, &candidates, &scheme, 10, &table, 0xF00D).unwrap();
        let child = scheme.reproduce(&chooser, 1, &candidates[pick], 2, 10, 0xF00D, 0);
        assert_eq!(
            hybrid_viability(&child, &table),
            Fixed::ONE,
            "the evolved preference avoids the incompatible mate"
        );
    }
}
