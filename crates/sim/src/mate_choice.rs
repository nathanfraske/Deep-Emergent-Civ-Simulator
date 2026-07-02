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
//! [`crate::genome::IncompatibilityTable::hybrid_outcome`]). Showing the sign emerge under the
//! full genome-derived fitness through selection (`reproduce` plus `hybrid_outcome` plus
//! offspring heterozygosity, run on `evolve_with`) is the named follow-on below.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{
        Allele, AlleleState, Genome, Haplotype, HybridOutcome, Incompatibility,
        IncompatibilityKind, IncompatibilityTable, SchemeId,
    };

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
        // A homozygous diploid genome with state 1 at the given loci and 0 elsewhere.
        let g = |loci: usize, ones: &[usize]| {
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
        };
        // A hybrid carrying one strand from each parent (homozygous parents transmit their
        // identical strand).
        let cross = |p1: &Genome, p2: &Genome| Genome {
            scheme: SchemeId(0),
            haps: vec![p1.haps[0].clone(), p2.haps[0].clone()],
        };

        // Near cross: one parent carries the DM allele at locus 0, the other at locus 1, so the
        // hybrid unites the incompatible pair. They differ at two loci.
        let near = cross(&g(10, &[0]), &g(10, &[1]));
        // Far cross: one parent carries state 1 at four loci that never include locus 1, the
        // other is all zero, so the hybrid never unites the pair. They differ at four loci.
        let far = cross(&g(10, &[0, 2, 3, 4]), &g(10, &[]));

        assert!(
            genetic_distance(&g(10, &[0]), &g(10, &[1]))
                < genetic_distance(&g(10, &[0, 2, 3, 4]), &g(10, &[])),
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
}
