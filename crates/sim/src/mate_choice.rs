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
//! physics; it forbids an authored behavioural outcome). The proof below shows that the same
//! preference machinery, scored under different offspring physics, favours different
//! directions, so the direction lives in the physics and not in the mechanism (the
//! anti-steering invariant, the R-EVOLVE-STEER analogue).
//!
//! Compute: this is the cheap "matching rule / magic trait / one-allele" case (Felsenstein
//! 1981; Servedio et al 2011; Kopp et al 2018), where the mating cue is the fitness-relevant
//! genome itself, so no separate preference dimension must be coevolved and held in linkage
//! against recombination. A preference over an already-computed genome distance is order-1
//! over the existing evolution loop, so emergence is tractable and is the recommendation, not
//! the fallback. Honest follow-ons, each named and not built here: the offspring fitness is a
//! supplied function of distance in this proof, to be replaced by the genome-derived fitness
//! (form the child with [`crate::genome::GeneticScheme::reproduce`], read hybrid viability
//! and offspring heterozygosity); the preference reused on the shared `evolve_with` plus
//! `Controller` substrate once its input registry carries a candidate-percept family; the
//! value and axiom distances added as further features; and the `World::birth` call site,
//! which today takes both parents pre-chosen. The reserved values the resolved mechanism
//! surfaces (none fabricated) are the cost of choosiness, the preference mutation variance,
//! and, for the deep-time pure-frequency fallback only, a per-race assortment-bias scalar.

use crate::genome::Genome;
use civsim_core::Fixed;

/// The genetic distance between two genomes over their discrete allele states, in `[0, 1]`:
/// the mean over loci of the fraction of the ploidy whose sorted states differ. Zero for
/// identical genomes, one for genomes that disagree at every allele. The individual-genome
/// analogue of [`crate::genome::GenePool::distance`] (which is the pool-tier mean absolute
/// frequency difference), a pure function of two genomes with no authored physics. The states
/// are compared sorted per locus, so the distance is genotype-based and blind to haplotype
/// phase. Panics if the genomes differ in ploidy or locus count (they must share a
/// [`crate::genome::GeneSet`]).
pub fn genetic_distance(a: &Genome, b: &Genome) -> Fixed {
    let ploidy = a.haps.len();
    assert_eq!(ploidy, b.haps.len(), "genetic distance needs equal ploidy");
    assert!(ploidy >= 1, "a genome carries at least one haplotype");
    let loci = a.haps[0].alleles.len();
    assert_eq!(
        loci,
        b.haps[0].alleles.len(),
        "genetic distance needs equal locus counts"
    );
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
        for k in 0..ploidy {
            if sa[k] != sb[k] {
                diff += 1;
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{Allele, AlleleState, Genome, Haplotype, SchemeId};

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
        // The load-bearing P8/P9 proof. The SAME preference machinery is scored under two
        // different authored offspring-physics worlds. In the incompatibility world the
        // homophile out-reproduces the heterophile; in the heterosis world the ordering flips.
        // The direction lives in the physics, not in the mechanism.
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
    fn a_neutral_world_favours_no_direction() {
        // The anti-steering invariant: with flat offspring physics the mechanism adds no
        // advantage to either direction. Homophily, heterophily, and indifference realise the
        // same fitness, so nothing but the physics can author a direction.
        let chooser = genome(10, 0);
        let candidates = vec![genome(10, 1), genome(10, 5), genome(10, 9)];
        let homo = realised_fitness(
            &MatePreference::homophilic(),
            &chooser,
            &candidates,
            neutral_world,
        );
        let hetero = realised_fitness(
            &MatePreference::heterophilic(),
            &chooser,
            &candidates,
            neutral_world,
        );
        let indiff = realised_fitness(
            &MatePreference::indifferent(),
            &chooser,
            &candidates,
            neutral_world,
        );
        assert_eq!(homo, hetero, "a flat world favours neither direction");
        assert_eq!(hetero, indiff, "and indifference does no worse");
    }

    #[test]
    fn the_fitness_optimal_preference_weight_follows_the_physics() {
        // The selection gradient itself: sweep the preference weight from strongly homophilic
        // to strongly heterophilic and find the weight that maximises realised fitness in each
        // world. Selection climbs to a negative weight (homophily) under incompatibility and a
        // positive weight (heterophily) under heterosis, from the same candidate panel and the
        // same mechanism. This is the emergent direction the recommendation names: the sign
        // selection settles on is set by the physics.
        let chooser = genome(12, 0);
        let candidates = vec![genome(12, 1), genome(12, 4), genome(12, 8), genome(12, 11)];
        let weights: Vec<Fixed> = (-3..=3).map(Fixed::from_int).collect();

        let argmax_weight = |world: fn(Fixed) -> Fixed| -> Fixed {
            weights
                .iter()
                .map(|&w| {
                    let f = realised_fitness(&MatePreference::new(w), &chooser, &candidates, world);
                    (f, w)
                })
                .max_by(|x, y| x.0.cmp(&y.0).then(y.1.cmp(&x.1)))
                .map(|(_, w)| w)
                .unwrap()
        };

        assert!(
            argmax_weight(incompatibility_world) < Fixed::ZERO,
            "selection climbs to a homophilic weight when hybrids are unfit"
        );
        assert!(
            argmax_weight(heterosis_world) > Fixed::ZERO,
            "selection climbs to a heterophilic weight under heterozygote advantage"
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
}
