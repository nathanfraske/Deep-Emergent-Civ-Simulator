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

//! The genome: representation and genotype-to-phenotype expression (design Part 25, the
//! resolved R-GENOME work, record 62.5).
//!
//! This is the foundation brick of the genetic model: what a genome is (25.1) and how a
//! genotype expresses a phenotype channel (25.3, 25.4). It is a multi-locus quantitative
//! spine, where many small-effect alleles sum to a heritable value, with an optional
//! Mendelian dominance layer per gene laid on where a discrete, hideable character is
//! wanted. The mechanism is fixed Rust; which genes exist, what each reaches, and every
//! effect size are data (Principle 11). It applies uniformly to sentient races, animals,
//! and plants, differing only in the genes they carry.
//!
//! The phenotype channels a gene may feed ([`Channel`]) are a fixed mechanism enum, the
//! interface the genotype expresses into, on the same footing as [`crate::tom::EvidenceOrder`]
//! and [`crate::dialogue::ForceKind`]: each is a phenotype the engine knows how to read
//! (a Part 20 trait setpoint keyed by a data trait id, a cognition channel, a build
//! channel, an imbued trait, a life-history channel). What is data is which genes exist,
//! which channels they feed and with what weight, and their dominance, all carried in the
//! [`GeneSet`]. Anatomy is intentionally absent (25.1): which body parts a body plan has
//! is its own reserved question.
//!
//! Everything here is integer and fixed-point and a pure function of the genome and the
//! gene set, so a phenotype is bit-identical across machines and thread counts. What this
//! brick does not yet build, and what follows it: inheritance (segregation and
//! recombination), mutation, the bounded epistasis lookup, the two-tier allele-frequency
//! pool with Hardy-Weinberg promotion, and deep-time drift and selection (25.5, 25.7,
//! 25.8). The per-race genetic scheme is represented here only by its id on a genome;
//! its reproduction and inheritance variants arrive with the inheritance brick.

use civsim_core::{DrawKey, Fixed, Phase};

/// A data-defined gene identifier (Part 40).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GeneId(pub u32);

/// A data-defined personality-trait-axis identifier (the axes of Part 20).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TraitId(pub u32);

/// A per-race genetic-scheme identifier (the reproduction and inheritance variants of
/// 25.2). Carried on a genome so a being knows which scheme governs it; the scheme
/// registry itself arrives with the inheritance brick.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SchemeId(pub u32);

/// The cognitive-capacity channels, kept distinct (25.6): reasoning acuity gates cognitive
/// events and perception quality, memory governs belief deterioration, belief plasticity
/// governs how readily beliefs update. They may share loci by pleiotropy but are not one
/// axis.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum CognitionChannel {
    /// Reasoning acuity (the perception and inference quality of R-EVIDENCE, Part 41).
    ReasoningAcuity,
    /// Memory capacity (the belief deterioration of Part 9).
    MemoryCapacity,
    /// Belief plasticity (how readily beliefs update, Part 20).
    BeliefPlasticity,
}

/// The physical-build channels a gene may feed. Which of these are primitive versus
/// physics-derived is the connected open item R-BUILD-PHYS; the channel set is the
/// expression interface, not that decision.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BuildChannel {
    /// Body size.
    Size,
    /// Muscular strength.
    Strength,
    /// Movement speed.
    Speed,
    /// Climate tolerance.
    ClimateTolerance,
    /// Locomotion mode capacity.
    Locomotion,
}

/// The imbued (innate, often magical or constitutional) channels a gene may feed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ImbuedChannel {
    /// Affinity for magic.
    MagicAffinity,
    /// Resistance to disease.
    DiseaseImmunity,
    /// Capacity to regenerate.
    Regeneration,
    /// Sight in darkness.
    Nightvision,
}

/// The life-history channels a gene may feed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum LifeHistoryChannel {
    /// Maximum lifespan.
    Lifespan,
    /// Reproductive rate.
    Reproduction,
}

/// A phenotype channel a gene's effect feeds. The fixed mechanism interface the genotype
/// expresses into: a Part 20 trait setpoint keyed by a data trait id, or one of the
/// cognition, build, imbued, or life-history channels. Anatomy is intentionally absent
/// (25.1). New phenotype interfaces are an engine extension, never world data; what genes
/// reach these channels, and with what weight, is the data.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Channel {
    /// An additive push on a Part 20 personality trait setpoint, by trait-axis id.
    TraitSetpoint(TraitId),
    /// A cognitive-capacity channel (25.6).
    Cognition(CognitionChannel),
    /// A physical-build channel.
    Build(BuildChannel),
    /// An imbued (innate) channel.
    Imbued(ImbuedChannel),
    /// A life-history channel.
    LifeHistory(LifeHistoryChannel),
}

/// One phenotypic effect of a gene: the channel it feeds and the additive weight with
/// which the locus's allele values push that channel. A gene may carry several effects
/// (pleiotropy).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GeneEffect {
    /// The phenotype channel this effect feeds.
    pub channel: Channel,
    /// The additive weight applied to the locus's allele values for this channel.
    pub weight: Fixed,
}

/// How an allele pair resolves under a diploid scheme (25.4). The degree of dominance,
/// after Falconer: `a` is half the difference between the two homozygotes, `d` the
/// heterozygote's deviation from their midpoint, and the kind names the regime. The
/// expression here applies `d` as the heterozygote deviation; `a` and `kind` carry the
/// homozygous half-difference and the regime label for the inheritance and distance work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DominanceMode {
    /// Half the difference between the two homozygotes (Falconer's a).
    pub a: Fixed,
    /// The heterozygote's deviation from the homozygote midpoint (Falconer's d).
    pub d: Fixed,
    /// The dominance regime.
    pub kind: DominanceKind,
}

impl DominanceMode {
    /// A purely additive gene: no dominance deviation. The genotype-to-phenotype map then
    /// collapses to the additive sum, the limit that reconciles with the Part 20
    /// personality inheritance rule (25.3).
    pub fn additive() -> Self {
        DominanceMode {
            a: Fixed::ZERO,
            d: Fixed::ZERO,
            kind: DominanceKind::Additive,
        }
    }
}

/// The dominance regime of a gene (25.4).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DominanceKind {
    /// No dominance: the heterozygote sits at the homozygote midpoint.
    Additive,
    /// Partial dominance: the heterozygote leans toward one homozygote.
    Incomplete,
    /// Full dominance: the heterozygote matches one homozygote.
    Complete,
    /// Overdominance: the heterozygote exceeds both homozygotes.
    Over,
    /// Co-dominance: both alleles express.
    Co,
}

/// The discrete state of an allele (the Mendelian, hideable view). State 0 is the default
/// quantitative-only allele; distinct non-zero states make a locus heterozygous and so
/// expose the dominance deviation. Used for distance and incompatibility in later bricks.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct AlleleState(pub u16);

/// One allele: a small-effect additive value (the quantitative view), a discrete state
/// (the Mendelian view), and an origin tag used for distance and incompatibility (25.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Allele {
    /// The allele's small-effect additive contribution.
    pub additive: Fixed,
    /// The discrete allele state (0 is the quantitative-only default).
    pub state: AlleleState,
    /// An origin tag (which lineage the allele descends from).
    pub origin: u32,
}

impl Allele {
    /// A purely additive allele in the default state.
    pub fn additive(value: Fixed) -> Self {
        Allele {
            additive: value,
            state: AlleleState(0),
            origin: 0,
        }
    }
}

/// One haplotype: the alleles a being carries, indexed by the gene order of its
/// [`GeneSet`].
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Haplotype {
    /// The alleles, one per gene in carried order.
    pub alleles: Vec<Allele>,
}

/// A being's genome: which scheme governs it and its haplotypes, two for a diploid being,
/// one for a haploid or clonal one (25.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Genome {
    /// The per-race genetic scheme governing this genome.
    pub scheme: SchemeId,
    /// The haplotypes: two diploid, one haploid or clonal.
    pub haps: Vec<Haplotype>,
}

impl Genome {
    /// The ploidy: how many haplotypes the genome carries.
    pub fn ploidy(&self) -> usize {
        self.haps.len()
    }
}

/// A gene definition (25.1), reduced to what genotype-to-phenotype expression needs: its
/// id, the channels it feeds, and its dominance. The linkage site, mutation regime, and
/// incompatibility partners arrive with the inheritance brick.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GeneDef {
    /// The gene's stable identifier.
    pub id: GeneId,
    /// The phenotype channels this gene feeds (pleiotropy allowed).
    pub effects: Vec<GeneEffect>,
    /// How an allele pair at this gene resolves under a diploid scheme.
    pub dominance: DominanceMode,
}

/// The set of genes a race or species carries, in the canonical order its haplotypes
/// index by (Part 40). Data: which genes exist and what each reaches. The mechanism that
/// reads it ([`GeneSet::express`]) is fixed.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GeneSet {
    /// The genes, in carried order (the order a haplotype's alleles align to).
    pub genes: Vec<GeneDef>,
}

impl GeneSet {
    /// An empty gene set.
    pub fn new() -> Self {
        GeneSet::default()
    }

    /// Express a phenotype channel from a genome (25.3, 25.4). The value is the sum over
    /// the genes feeding the channel of each locus's genotypic contribution, plus the
    /// environmental offset, all in fixed point:
    ///
    /// - the locus's additive part is the sum of its alleles' additive values (the
    ///   quantitative spine), times the gene's weight for the channel;
    /// - the dominance deviation `d` is added when the locus is diploid and heterozygous
    ///   (its two alleles differ in state), times the weight (the Mendelian overlay).
    ///
    /// With every feeding gene additive (`d` zero), this collapses to the weighted allele
    /// sum, the limit that reconciles with the Part 20 personality inheritance rule. It is
    /// a pure function of the genome and the gene set, with no RNG and no float, so the
    /// phenotype is bit-identical on replay. Alleles past a haplotype's length contribute
    /// nothing, so a genome carrying fewer alleles than the gene set degrades cleanly.
    pub fn express(&self, genome: &Genome, channel: Channel, environment: Fixed) -> Fixed {
        let mut total = environment;
        for (locus, gene) in self.genes.iter().enumerate() {
            let weight = match channel_weight(gene, channel) {
                Some(w) => w,
                None => continue,
            };
            let mut additive = Fixed::ZERO;
            let mut states: Vec<AlleleState> = Vec::with_capacity(genome.haps.len());
            for hap in &genome.haps {
                if let Some(allele) = hap.alleles.get(locus) {
                    additive += allele.additive;
                    states.push(allele.state);
                }
            }
            let mut genotypic = additive;
            if states.len() == 2 && states[0] != states[1] {
                genotypic += gene.dominance.d;
            }
            total += genotypic.mul(weight);
        }
        total
    }
}

/// The weight with which a gene feeds a channel, summing across its effects so a gene that
/// feeds one channel through several effects accumulates them. `None` if the gene does not
/// feed the channel at all.
fn channel_weight(gene: &GeneDef, channel: Channel) -> Option<Fixed> {
    let mut weight = Fixed::ZERO;
    let mut found = false;
    for e in &gene.effects {
        if e.channel == channel {
            weight += e.weight;
            found = true;
        }
    }
    found.then_some(weight)
}

// --- Inheritance: per-race scheme, gamete formation, and reproduction (design 25.2, 25.4,
// 25.5) ---

/// How a race or species reproduces and inherits (design 25.2). Three mechanism variants
/// are implemented here, defaulting to the sexual diploid model ordinary creatures share.
/// The two escape-hatch modes the design names (eusocial caste inheritance and a
/// magically-determined non-allelic rule) are deferred; they dispatch to bespoke audited
/// functions when built, and are not in this enum yet.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ReproductionMode {
    /// The common default: Mendelian segregation with recombination, two parents, a
    /// diploid offspring.
    SexualDiploid,
    /// A single haploid parent contributes its one strand (with mutation) to a haploid
    /// offspring.
    Haploid,
    /// The offspring is the single parent's genome copied, plus mutation (no recombination,
    /// no second parent).
    Clonal,
}

/// One linkage group (design 25.4): an ordered run of loci (gene indices into the
/// [`GeneSet`]) that travel together, with a per-interval recombination fraction. Genes in
/// different groups assort independently; within a group, a crossover between two adjacent
/// loci fires when a draw falls below that interval's fraction, so linkage disequilibrium
/// and hitchhiking emerge for free.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct LinkageGroup {
    /// The loci in this group, in map order, as gene indices into the gene set.
    pub loci: Vec<u32>,
    /// The crossover fraction for each adjacent interval, length `loci.len().saturating_sub(1)`.
    /// Reserved owner values (the genetic map), supplied as data, never fabricated.
    pub recombination: Vec<Fixed>,
}

/// A per-race genetic scheme (design 25.2): which reproduction variant a race runs and its
/// genetic map. The mechanism is fixed; the linkage map and the mutation rate are data.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GeneticScheme {
    /// The scheme's identifier (carried on a [`Genome`]).
    pub id: SchemeId,
    /// Which reproduction and inheritance variant this scheme runs.
    pub reproduction: ReproductionMode,
    /// The linkage groups partitioning the genes. Loci not covered by any group assort
    /// independently (each its own singleton).
    pub linkage_groups: Vec<LinkageGroup>,
    /// The per-locus point-mutation probability (a reserved owner value, supplied as data).
    /// A draw below this flips the locus's discrete allele state.
    pub mutation_rate: Fixed,
}

// Draw-site slots within the REPRODUCE phase, so the strand, crossover, and mutation rolls
// of one reproduction cannot collide on counter zero (the R-RNG-COORD slot rule).
const SLOT_STRAND: u32 = 0;
const SLOT_CROSSOVER: u32 = 1;
const SLOT_MUTATE: u32 = 2;

impl GeneticScheme {
    /// Form one gamete from a parent: a haplotype indexed by the gene set's order, each
    /// allele drawn from one of the parent's strands by walking the linkage groups with
    /// crossover (design 25.4), then point mutation applied per locus (25.5). For a haploid
    /// or clonal parent the single strand is copied (no recombination). Every draw is keyed
    /// through the canonical schema on the contributing parent and the locus, so a lineage
    /// is bit-identical across machines and thread counts. `gene_count` is the gene set's
    /// length; `parent_id` keys the draws; `generation` is the reproduction ordinal.
    pub fn gamete(
        &self,
        parent: &Genome,
        gene_count: usize,
        seed: u64,
        parent_id: u64,
        generation: u64,
    ) -> Haplotype {
        // The strand each locus is copied from. For a diploid parent, walk the linkage
        // groups; for a single-strand parent, every locus reads strand 0.
        let diploid = parent.haps.len() >= 2;
        let mut strand = vec![0usize; gene_count];
        if diploid {
            // Loci covered by a group follow its walk; uncovered loci assort independently.
            let mut covered = vec![false; gene_count];
            for (g, group) in self.linkage_groups.iter().enumerate() {
                // Independent assortment between groups: each group starts on its own draw.
                let mut s = (DrawKey::pair(parent_id, g as u64, generation, Phase::REPRODUCE)
                    .slot(SLOT_STRAND)
                    .rng(seed)
                    .at(0)
                    & 1) as usize;
                for (i, &locus) in group.loci.iter().enumerate() {
                    let l = locus as usize;
                    if l < gene_count {
                        strand[l] = s;
                        covered[l] = true;
                    }
                    // Crossover before the next locus in the group.
                    if i + 1 < group.loci.len() {
                        let frac = group.recombination.get(i).copied().unwrap_or(Fixed::ZERO);
                        let roll =
                            DrawKey::pair(parent_id, locus as u64, generation, Phase::REPRODUCE)
                                .slot(SLOT_CROSSOVER)
                                .rng(seed)
                                .unit_fixed(0);
                        if roll < frac {
                            s ^= 1;
                        }
                    }
                }
            }
            for (l, c) in covered.iter().enumerate() {
                if !*c {
                    strand[l] = (DrawKey::pair(parent_id, l as u64, generation, Phase::REPRODUCE)
                        .slot(SLOT_STRAND)
                        .rng(seed)
                        .at(0)
                        & 1) as usize;
                }
            }
        }

        // Copy each locus's allele from the chosen strand, then mutate. `strand` is zero
        // for a single-strand (haploid or clonal) parent, so the same walk serves both.
        let mut alleles = Vec::with_capacity(gene_count);
        for (l, &s) in strand.iter().enumerate() {
            let mut allele = parent
                .haps
                .get(s)
                .and_then(|h| h.alleles.get(l))
                .copied()
                .or_else(|| parent.haps.first().and_then(|h| h.alleles.get(l)).copied())
                .unwrap_or(Allele::additive(Fixed::ZERO));
            let roll = DrawKey::pair(parent_id, l as u64, generation, Phase::REPRODUCE)
                .slot(SLOT_MUTATE)
                .rng(seed)
                .unit_fixed(0);
            if roll < self.mutation_rate {
                // A point mutation flips the discrete allele state to a fresh variant. The
                // continuous additive-step mutation is deferred (it needs the reserved
                // integer-Gaussian approximation of 25.10), so the quantitative spine does
                // not yet mutate here.
                allele.state = AlleleState(allele.state.0.wrapping_add(1));
            }
            alleles.push(allele);
        }
        Haplotype { alleles }
    }

    /// Produce an offspring genome from one or two parents under this scheme (design 25.4,
    /// 25.5). Sexual diploid recombines a gamete from each parent; haploid takes a single
    /// strand from one parent; clonal copies the one parent and mutates. A pure function of
    /// the seed, the parents, their ids, and the generation ordinal, so a lineage replays
    /// bit for bit. `p2` is ignored for the single-parent modes.
    #[allow(clippy::too_many_arguments)]
    pub fn reproduce(
        &self,
        p1: &Genome,
        p1_id: u64,
        p2: &Genome,
        p2_id: u64,
        gene_count: usize,
        seed: u64,
        generation: u64,
    ) -> Genome {
        match self.reproduction {
            ReproductionMode::SexualDiploid => {
                let g1 = self.gamete(p1, gene_count, seed, p1_id, generation);
                let g2 = self.gamete(p2, gene_count, seed, p2_id, generation);
                Genome {
                    scheme: self.id,
                    haps: vec![g1, g2],
                }
            }
            ReproductionMode::Haploid => {
                let g = self.gamete(p1, gene_count, seed, p1_id, generation);
                Genome {
                    scheme: self.id,
                    haps: vec![g],
                }
            }
            ReproductionMode::Clonal => {
                // The offspring is the parent's strands copied, each mutated per locus.
                let g = self.gamete(p1, gene_count, seed, p1_id, generation);
                let haps = if p1.haps.len() >= 2 {
                    // A diploid clone keeps both strands; the gamete already mutated strand
                    // choices, so mutate the second strand on its own draw stream.
                    let g2 = self.gamete(p1, gene_count, seed, p1_id ^ 1, generation);
                    vec![g, g2]
                } else {
                    vec![g]
                };
                Genome {
                    scheme: self.id,
                    haps,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEME: SchemeId = SchemeId(0);
    const ACUITY: Channel = Channel::Cognition(CognitionChannel::ReasoningAcuity);

    fn additive_gene(id: u32, channel: Channel, weight: i32) -> GeneDef {
        GeneDef {
            id: GeneId(id),
            effects: vec![GeneEffect {
                channel,
                weight: Fixed::from_int(weight),
            }],
            dominance: DominanceMode::additive(),
        }
    }

    fn diploid(a: [Allele; 2], b: [Allele; 2]) -> Genome {
        // Two loci, two haplotypes: hap0 = [a0, b0], hap1 = [a1, b1].
        Genome {
            scheme: SCHEME,
            haps: vec![
                Haplotype {
                    alleles: vec![a[0], b[0]],
                },
                Haplotype {
                    alleles: vec![a[1], b[1]],
                },
            ],
        }
    }

    #[test]
    fn an_additive_diploid_genome_sums_its_allele_values() {
        // Two loci both feeding acuity, weights 1 and 2; the phenotype is the weighted
        // sum of the summed allele values per locus.
        let genes = GeneSet {
            genes: vec![additive_gene(1, ACUITY, 1), additive_gene(2, ACUITY, 2)],
        };
        let g = diploid(
            [
                Allele::additive(Fixed::from_int(3)),
                Allele::additive(Fixed::from_int(1)),
            ],
            [
                Allele::additive(Fixed::from_int(2)),
                Allele::additive(Fixed::from_int(2)),
            ],
        );
        // locus 1: (3+1)*1 = 4; locus 2: (2+2)*2 = 8; total 12.
        assert_eq!(genes.express(&g, ACUITY, Fixed::ZERO), Fixed::from_int(12));
    }

    #[test]
    fn the_environment_offset_adds() {
        let genes = GeneSet {
            genes: vec![additive_gene(1, ACUITY, 1)],
        };
        let g = Genome {
            scheme: SCHEME,
            haps: vec![Haplotype {
                alleles: vec![Allele::additive(Fixed::from_int(2))],
            }],
        };
        // haploid: locus contributes 2*1 = 2, plus environment 5 = 7.
        assert_eq!(
            genes.express(&g, ACUITY, Fixed::from_int(5)),
            Fixed::from_int(7)
        );
    }

    #[test]
    fn a_channel_no_gene_feeds_is_just_the_environment() {
        let genes = GeneSet {
            genes: vec![additive_gene(1, ACUITY, 1)],
        };
        let g = Genome {
            scheme: SCHEME,
            haps: vec![Haplotype {
                alleles: vec![Allele::additive(Fixed::from_int(9))],
            }],
        };
        let unfed = Channel::Build(BuildChannel::Size);
        assert_eq!(
            genes.express(&g, unfed, Fixed::from_int(4)),
            Fixed::from_int(4)
        );
    }

    #[test]
    fn dominance_deviation_applies_only_when_heterozygous() {
        // One locus feeding acuity, complete dominance with d = 2.
        let gene = GeneDef {
            id: GeneId(1),
            effects: vec![GeneEffect {
                channel: ACUITY,
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode {
                a: Fixed::from_int(3),
                d: Fixed::from_int(2),
                kind: DominanceKind::Complete,
            },
        };
        let genes = GeneSet { genes: vec![gene] };

        // Heterozygous: states differ, so the deviation d applies. additive 3+3=6, +d 2 = 8.
        let het = Genome {
            scheme: SCHEME,
            haps: vec![
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::from_int(3),
                        state: AlleleState(1),
                        origin: 0,
                    }],
                },
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::from_int(3),
                        state: AlleleState(2),
                        origin: 0,
                    }],
                },
            ],
        };
        assert_eq!(genes.express(&het, ACUITY, Fixed::ZERO), Fixed::from_int(8));

        // Homozygous: same state, no deviation. additive 3+3=6.
        let hom = Genome {
            scheme: SCHEME,
            haps: vec![
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::from_int(3),
                        state: AlleleState(1),
                        origin: 0,
                    }],
                },
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::from_int(3),
                        state: AlleleState(1),
                        origin: 0,
                    }],
                },
            ],
        };
        assert_eq!(genes.express(&hom, ACUITY, Fixed::ZERO), Fixed::from_int(6));
    }

    #[test]
    fn a_pleiotropic_gene_feeds_several_channels() {
        let size = Channel::Build(BuildChannel::Size);
        let gene = GeneDef {
            id: GeneId(1),
            effects: vec![
                GeneEffect {
                    channel: ACUITY,
                    weight: Fixed::from_int(1),
                },
                GeneEffect {
                    channel: size,
                    weight: Fixed::from_int(3),
                },
            ],
            dominance: DominanceMode::additive(),
        };
        let genes = GeneSet { genes: vec![gene] };
        let g = Genome {
            scheme: SCHEME,
            haps: vec![Haplotype {
                alleles: vec![Allele::additive(Fixed::from_int(2))],
            }],
        };
        assert_eq!(genes.express(&g, ACUITY, Fixed::ZERO), Fixed::from_int(2));
        assert_eq!(genes.express(&g, size, Fixed::ZERO), Fixed::from_int(6));
    }

    #[test]
    fn trait_setpoints_key_by_axis() {
        // Two trait axes fed by two genes; each axis expresses only its own gene.
        let axis0 = Channel::TraitSetpoint(TraitId(0));
        let axis1 = Channel::TraitSetpoint(TraitId(1));
        let genes = GeneSet {
            genes: vec![additive_gene(1, axis0, 1), additive_gene(2, axis1, 1)],
        };
        let g = diploid(
            [
                Allele::additive(Fixed::from_int(5)),
                Allele::additive(Fixed::from_int(5)),
            ],
            [
                Allele::additive(Fixed::from_int(1)),
                Allele::additive(Fixed::from_int(1)),
            ],
        );
        assert_eq!(genes.express(&g, axis0, Fixed::ZERO), Fixed::from_int(10));
        assert_eq!(genes.express(&g, axis1, Fixed::ZERO), Fixed::from_int(2));
    }

    #[test]
    fn expression_is_a_pure_deterministic_function() {
        let genes = GeneSet {
            genes: vec![additive_gene(1, ACUITY, 2)],
        };
        let g = Genome {
            scheme: SCHEME,
            haps: vec![
                Haplotype {
                    alleles: vec![Allele::additive(Fixed::from_ratio(1, 2))],
                },
                Haplotype {
                    alleles: vec![Allele::additive(Fixed::from_ratio(1, 4))],
                },
            ],
        };
        let a = genes.express(&g, ACUITY, Fixed::from_int(1));
        let b = genes.express(&g, ACUITY, Fixed::from_int(1));
        assert_eq!(a, b, "expression reproduces exactly");
        // (1/2 + 1/4) * 2 + 1 = 3/2 + 1 = 5/2.
        assert_eq!(a, Fixed::from_ratio(5, 2));
    }

    #[test]
    fn ploidy_reports_haplotype_count() {
        let g = Genome {
            scheme: SCHEME,
            haps: vec![Haplotype::default(), Haplotype::default()],
        };
        assert_eq!(g.ploidy(), 2);
    }

    // --- Inheritance (design 25.2, 25.4, 25.5) ---

    // A diploid parent over `n` loci whose two strands are tagged by origin, so a gamete's
    // provenance is visible: strand 0 alleles carry `o0`, strand 1 alleles carry `o1`.
    fn tagged_parent(n: usize, o0: u32, o1: u32) -> Genome {
        let strand = |o: u32| Haplotype {
            alleles: (0..n)
                .map(|_| Allele {
                    additive: Fixed::from_int(1),
                    state: AlleleState(0),
                    origin: o,
                })
                .collect(),
        };
        Genome {
            scheme: SCHEME,
            haps: vec![strand(o0), strand(o1)],
        }
    }

    fn scheme(mode: ReproductionMode, n: usize, recomb: i32, mutation: Fixed) -> GeneticScheme {
        // One linkage group covering all loci, a uniform per-interval crossover fraction.
        GeneticScheme {
            id: SCHEME,
            reproduction: mode,
            linkage_groups: vec![LinkageGroup {
                loci: (0..n as u32).collect(),
                recombination: vec![Fixed::from_int(recomb); n.saturating_sub(1)],
            }],
            mutation_rate: mutation,
        }
    }

    #[test]
    fn clonal_reproduction_copies_the_parent_and_replays() {
        let n = 4;
        let parent = tagged_parent(n, 10, 11);
        let sc = scheme(ReproductionMode::Clonal, n, 0, Fixed::ZERO);
        let child = sc.reproduce(&parent, 1, &parent, 2, n, 0xABC, 0);
        // No mutation, no recombination: the clone's first strand reproduces a parent strand.
        assert_eq!(child.haps.len(), 2);
        assert!(child.haps[0]
            .alleles
            .iter()
            .all(|a| a.origin == 10 || a.origin == 11));
        // Bit-identical replay.
        let again = sc.reproduce(&parent, 1, &parent, 2, n, 0xABC, 0);
        assert_eq!(child, again, "a clonal lineage replays bit for bit");
    }

    #[test]
    fn a_sexual_child_draws_each_strand_from_the_matching_parent() {
        let n = 4;
        let p1 = tagged_parent(n, 10, 11);
        let p2 = tagged_parent(n, 20, 21);
        let sc = scheme(ReproductionMode::SexualDiploid, n, 0, Fixed::ZERO);
        let child = sc.reproduce(&p1, 1, &p2, 2, n, 0x5EED, 0);
        assert_eq!(child.haps.len(), 2);
        // Hap 0 is p1's gamete (origins from p1), hap 1 is p2's gamete (origins from p2).
        assert!(
            child.haps[0]
                .alleles
                .iter()
                .all(|a| a.origin == 10 || a.origin == 11),
            "the first strand came from parent one"
        );
        assert!(
            child.haps[1]
                .alleles
                .iter()
                .all(|a| a.origin == 20 || a.origin == 21),
            "the second strand came from parent two"
        );
        // Replay is bit-identical.
        let again = sc.reproduce(&p1, 1, &p2, 2, n, 0x5EED, 0);
        assert_eq!(child, again, "a sexual lineage replays bit for bit");
    }

    #[test]
    fn no_crossover_keeps_a_strand_intact_and_full_crossover_recombines() {
        let n = 4;
        let p1 = tagged_parent(n, 10, 11);
        let p2 = tagged_parent(n, 20, 21);
        // With recombination 0, a gamete is one parent strand wholesale: every locus shares
        // the one origin.
        let clean = scheme(ReproductionMode::SexualDiploid, n, 0, Fixed::ZERO);
        let child = clean.reproduce(&p1, 1, &p2, 2, n, 7, 0);
        let o0 = child.haps[0].alleles[0].origin;
        assert!(
            child.haps[0].alleles.iter().all(|a| a.origin == o0),
            "no crossover leaves the strand intact"
        );
        // With recombination 1, the strand flips at every interval, so a multi-locus gamete
        // carries both of its parent's origins (it is recombined).
        let crossed = scheme(ReproductionMode::SexualDiploid, n, 1, Fixed::ZERO);
        let child2 = crossed.reproduce(&p1, 1, &p2, 2, n, 7, 0);
        let origins: std::collections::BTreeSet<u32> =
            child2.haps[0].alleles.iter().map(|a| a.origin).collect();
        assert_eq!(
            origins,
            [10, 11].into_iter().collect(),
            "full crossover recombines both strands of parent one"
        );
    }

    #[test]
    fn mutation_at_full_rate_flips_every_state_and_off_flips_none() {
        let n = 3;
        let p1 = tagged_parent(n, 10, 11);
        let p2 = tagged_parent(n, 20, 21);
        // Parent strands start at state 0.
        let always = scheme(ReproductionMode::SexualDiploid, n, 0, Fixed::ONE);
        let mutated = always.reproduce(&p1, 1, &p2, 2, n, 1, 0);
        assert!(
            mutated.haps[0]
                .alleles
                .iter()
                .all(|a| a.state != AlleleState(0)),
            "a full mutation rate flips every locus's discrete state"
        );
        let never = scheme(ReproductionMode::SexualDiploid, n, 0, Fixed::ZERO);
        let clean = never.reproduce(&p1, 1, &p2, 2, n, 1, 0);
        assert!(
            clean.haps[0]
                .alleles
                .iter()
                .all(|a| a.state == AlleleState(0)),
            "a zero mutation rate leaves every state untouched"
        );
    }

    #[test]
    fn a_child_genotype_still_expresses_a_phenotype() {
        // The inherited genome plugs straight back into the genotype-to-phenotype map.
        let n = 2;
        let p1 = tagged_parent(n, 10, 11);
        let p2 = tagged_parent(n, 20, 21);
        let sc = scheme(ReproductionMode::SexualDiploid, n, 0, Fixed::ZERO);
        let child = sc.reproduce(&p1, 1, &p2, 2, n, 99, 0);
        let genes = GeneSet {
            genes: vec![additive_gene(1, ACUITY, 1), additive_gene(2, ACUITY, 1)],
        };
        // Each locus carries additive 1 on both strands, weight 1, two loci: 2*(1+1)*1 = 4.
        assert_eq!(
            genes.express(&child, ACUITY, Fixed::ZERO),
            Fixed::from_int(4)
        );
    }
}
