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
//! Everything here is integer and fixed-point with counter-keyed RNG, so a genome, a
//! phenotype, and a whole population's history are bit-identical across machines and
//! thread counts. This module now carries the genome representation and expression (25.1,
//! 25.3, 25.4), individual inheritance through the [`GeneticScheme`] (segregation,
//! recombination, and discrete-state mutation; 25.2, 25.4, 25.5), and the aggregate-tier
//! [`GenePool`] with Wright-Fisher drift, directional selection, genetic distance,
//! declared speciation, and the two-tier promotion and demotion crossing (25.7, 25.8).
//! Speciation runs on the owner's chosen rule: a frequency-distance threshold or a count of
//! active Dobzhansky-Muller incompatibilities, the latter drawn from the data
//! [`IncompatibilityTable`], so a discrete genetic firewall can isolate two pools that are
//! still close in frequency (25.7).
//!
//! The quantitative breeding-value tier is now built on the owner's stamped decisions (25.10):
//! the [`GenePool`] carries a per-locus average allele-substitution effect alpha_i alongside its
//! frequencies, so a promoted individual carries a continuous additive spine whose cohort variance
//! reconstructs the pool's additive genetic variance ([`GenePool::additive_variance`]);
//! narrow-sense heritability graduates from an authored constant to the derived read `V_A / (V_A +
//! V_E)` ([`GenePool::narrow_sense_heritability`]); and the continuous additive mutation step
//! perturbs the spine through the stamped integer-Gaussian approximation (`SumOfUniforms { k: 12 }`,
//! [`civsim_core::GaussApprox`]), the sole lever that grows additive variance. The effect vector is
//! per-race genome data (dev fixtures), never a manifest scalar; the stamp is a world-identity
//! value folded into [`GenePool::hash_into`].
//!
//! What remains deferred, and why: the bounded epistasis lookup (25.4); the infinitesimal
//! segregation noise on gamete formation; multi-allele loci (the pool is biallelic for now); and
//! the large-Ne Wright-Fisher approximation, the genetic-distance measure (a fixation index versus
//! a Nei distance), and the speciation and selection calibrations, which are owner-reserved
//! choices.

use civsim_core::{gaussian_unit, DrawKey, Fixed, GaussApprox, Phase, StateHasher};

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
/// expresses into: a Part 20 trait setpoint keyed by a data trait id, one of the
/// cognition, build, imbued, or life-history channels, or the sex-determination coordinate.
/// Anatomy is intentionally absent (25.1). New phenotype interfaces are an engine extension,
/// never world data; what genes reach these channels, and with what weight, is the data.
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
    /// A heritable tissue-composition coordinate, by composition-axis id (R-BIOSPHERE, fork
    /// F10). The heritable payload is a stick-breaking coordinate of the composition simplex
    /// rather than an absolute mass fraction, so additivity stays exact under drift and
    /// selection and defensive toxins (and their loss under domestication) fall out of
    /// selection. The variant is the fixed engine interface; which composition axes exist,
    /// and which genes reach them, is data.
    Composition(CompositionAxisId),
    /// A heritable behaviour-controller weight, by controller-parameter id (R-BEHAVIOR-EVOLVE,
    /// design Part 8; the evolved-behaviour work whose pass is `docs/emergent_behavior_design.md`).
    /// The controller is the mapping from a being's homeostatic state and percept to which
    /// morphological affordance it issues; its parameters are the heritable data expressed here,
    /// one weight per controller-parameter id, so behaviour is a lineage's inheritance the way
    /// its size and acuity are, evolving under the pre-dawn epoch's selection rather than being
    /// authored (Principle 9). The variant is the fixed engine interface; the controller's
    /// topology and how many parameters it has are data ([`crate::controller`]).
    Controller(ControllerParamId),
    /// A sex (mating-type) determination coordinate: the value read off a race's designated
    /// sex-determination locus (R-REPRO, design Part 25). A data-driven
    /// [`crate::breeding::BreedingSystem`] maps this expressed value to a
    /// [`crate::breeding::SexClass`], so the number of sex classes a race carries and how a
    /// genotype assigns to one are data, and the population sex ratio emerges from Fisherian
    /// selection on the locus rather than a drawn ratio (Principle 8). This variant is the fixed
    /// engine interface; which gene feeds it, and with what weight, is data, exactly like every
    /// other channel. Sex is therefore read through [`GeneSet::express`] like any other
    /// phenotype, with no bespoke sex-determination phase and no reserved ratio.
    SexDetermination,
    /// A heritable toxin-tolerance coordinate, by tolerance-axis id (base-level liveliness step 4,
    /// R-WOUND). The expressed value is a being's per-toxin-class tolerance the dose-response
    /// [`civsim_physics::laws::harm_class`] reads (a higher tolerance suffers less harm from a given
    /// dose), so a lineage adapts to an environmental gradient (a salt flat, a dust haze) by selection
    /// on this channel rather than being excluded at a fixed dose (Principle 8: a graded dose, never a
    /// gate). The variant is the fixed engine interface; which toxin classes exist and which genes reach
    /// them is data (a [`ToleranceRegistry`](crate::edibility::ToleranceRegistry) sibling to the
    /// controller and composition registries, keyed off the floor toxin-class id, never a `RaceId`).
    Tolerance(ToleranceAxisId),
}

/// A tolerance-axis id, an index into a world's toxin-tolerance registry (the floor toxin classes a
/// being's physiology carries a heritable tolerance for). A numeric id keeps [`Channel`] `Copy` and
/// `Ord`; the class-name-to-id mapping is data in the [`ToleranceRegistry`](crate::edibility::ToleranceRegistry),
/// sibling to the composition-axis and controller-parameter registries (base-level liveliness step 4).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ToleranceAxisId(pub u16);

/// A controller-parameter id, an index into a being's behaviour controller's flat weight vector
/// ([`crate::controller`]). A numeric id keeps [`Channel`] `Copy` and `Ord`; the parameter count
/// and the topology it indexes are data, sibling to the composition-axis registry. It is a `u32`
/// (not a `u16`), so a large controller (a wide recurrent network over a rich registry) cannot
/// silently collide two weights on one channel by truncation; `Channel` is already `u32`-sized
/// through [`TraitId`], so the width costs nothing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ControllerParamId(pub u32);

/// A composition-axis id, an index into the biosphere composition-axis registry (the floor
/// axes an organism's tissue varies over). A numeric id keeps [`Channel`] `Copy` and `Ord`;
/// the axis-name-to-id mapping is data in the biosphere registry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CompositionAxisId(pub u16);

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

/// Append a full founding controller gene block to a gene set and its parallel pool spine (base-level
/// liveliness, step 1). This adds one unit-effect additive gene for EVERY one of the controller's
/// `weight_count` heritable weights (so a founder carries the whole controller substrate and mutation
/// can later turn any weight on, matching `crate::evolve::controller_gene_set`), each feeding its
/// `Channel::Controller(ControllerParamId(k))` at a fresh locus. The pool spine is seeded from the
/// `seeds` the caller derived from a taxis pattern (`crate::controller::taxis_move_weights`): a seeded
/// weight's locus gets frequency `ONE` (the founder is homozygous, so the locus carries no additive
/// variance and the dawn expression is deterministic) and additive effect `target / ploidy` (a
/// `ploidy`-fold founder expresses `ploidy * (target / ploidy) = target`); an unseeded weight gets a
/// balanced frequency and zero effect, so it expresses to zero until mutation moves it. The existing
/// genes and their spine are untouched, so the cognition and sex loci keep their expression, and the
/// `genes`, `freqs`, and `effects` vectors stay parallel and index-aligned (locus = gene index), the
/// invariant [`GeneSet::express`] and [`GenePool`] read them by.
///
/// The seed magnitudes are the caller's reserved values (Principle 11); the `ONE` frequency, the
/// balanced default, and the `target / ploidy` effect are mechanism (the deterministic-homozygote and
/// dosage-normalising conventions), not fabricated content. Reads no race id (Principle 9).
pub fn append_controller_block(
    genes: &mut Vec<GeneDef>,
    freqs: &mut Vec<Fixed>,
    effects: &mut Vec<Fixed>,
    ploidy: usize,
    weight_count: usize,
    seeds: &[(ControllerParamId, Fixed)],
) {
    let ploidy_fx = Fixed::from_int(ploidy.max(1) as i32);
    let seed_of: std::collections::BTreeMap<u32, Fixed> =
        seeds.iter().map(|&(p, t)| (p.0, t)).collect();
    for k in 0..weight_count {
        genes.push(GeneDef {
            id: GeneId(genes.len() as u32),
            effects: vec![GeneEffect {
                channel: Channel::Controller(ControllerParamId(k as u32)),
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode::additive(),
        });
        match seed_of.get(&(k as u32)) {
            Some(&target) => {
                freqs.push(Fixed::ONE);
                effects.push(target.checked_div(ploidy_fx).unwrap_or(Fixed::ZERO));
            }
            None => {
                freqs.push(Fixed::from_ratio(1, 2));
                effects.push(Fixed::ZERO);
            }
        }
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
    /// The per-gene continuous additive-mutation step-size standard deviation (a reserved owner
    /// value, `genome.additive_mutation_step`, supplied as data). When a point mutation fires it
    /// also perturbs the allele's quantitative additive value by a mean-zero Gaussian step of this
    /// standard deviation (design 25.5, 25.10), the sole lever that grows the additive spine's
    /// variance. A zero step freezes the spine, reproducing the discrete-only mutation.
    pub additive_mutation_step: Fixed,
    /// The stamped integer-Gaussian approximation the additive step draws through
    /// (`genome.gauss_approx`, a world-identity value; design 25.10). Only consulted when the
    /// additive step is non-zero, so a scheme with a frozen spine keeps the loud-fail sentinel
    /// harmlessly.
    pub gauss: GaussApprox,
}

// Draw-site slots within the REPRODUCE phase, so the strand, crossover, and the two mutation
// rolls (the discrete state flip and the continuous additive step) of one reproduction cannot
// collide on counter zero (the R-RNG-COORD slot rule).
const SLOT_STRAND: u32 = 0;
const SLOT_CROSSOVER: u32 = 1;
const SLOT_MUTATE: u32 = 2;
const SLOT_MUTATE_STEP: u32 = 3;

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
                // A point mutation flips the discrete allele state to a fresh variant.
                allele.state = AlleleState(allele.state.0.wrapping_add(1));
                // It also perturbs the continuous quantitative spine by a mean-zero Gaussian
                // additive step of the reserved per-gene step-size standard deviation (25.5,
                // 25.10), drawn on its own slot so it never disturbs the state-flip roll. This is
                // the sole lever that grows the additive variance; a zero step std freezes the
                // spine, and then the stamped approximation is not consulted at all.
                if self.additive_mutation_step != Fixed::ZERO {
                    let step_rng = DrawKey::pair(parent_id, l as u64, generation, Phase::REPRODUCE)
                        .slot(SLOT_MUTATE_STEP)
                        .rng(seed);
                    allele.additive += self
                        .additive_mutation_step
                        .mul(gaussian_unit(&step_rng, 0, self.gauss));
                }
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

// --- The aggregate tier: allele-frequency pools and deep-time evolution (design 25.7,
// 25.8) ---

/// An aggregate-tier population: per-locus biallelic allele-state frequencies advanced
/// statistically over deep time. The masses live here as frequency vectors rather than
/// modelled individuals, which is what makes a deep-time radiation cheap; a promoted being
/// carries an explicit [`Genome`] sampled from the pool, and a demoted being folds back
/// into it. Biallelic per locus is the starting case; multi-allele loci and the
/// quantitative breeding-value tier (the breeder's-equation channel means) are follow-ons.
/// Every operation is integer and fixed-point with counter-keyed RNG, so a population's
/// whole history is part of the world's reproducible identity.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GenePool {
    /// The genetic scheme this population runs.
    pub scheme: SchemeId,
    /// The effective population size Ne (a reserved owner value supplied as data); it sets
    /// the strength of drift, since a smaller Ne drifts harder.
    pub effective_size: u32,
    /// Per locus, the frequency of allele state 1 in `[0,1]`; state 0's frequency is the
    /// complement.
    freqs: Vec<Fixed>,
    /// Per locus, the average allele-substitution effect alpha_i: the per-race genome datum that
    /// makes the pool a quantitative breeding-value tier and not a bare frequency tier. It doubles
    /// as the pool's per-locus additive mean, the target [`GenePool::promote`] centres a promoted
    /// individual's additive spine on and [`GenePool::demote`] folds a demoted one back into. A
    /// flat pool (all zero) has no additive spine and promotes with zero additive, reproducing the
    /// prior behaviour. Parallel to `freqs`; per-race data (dev fixtures), never a manifest scalar.
    effects: Vec<Fixed>,
    /// The stamped integer-Gaussian approximation the additive spine draws through
    /// (`genome.gauss_approx`, a world-identity value; design 25.10). Consulted by `promote` only
    /// at loci with a non-zero effect, so a flat pool keeps the loud-fail sentinel harmlessly.
    gauss: GaussApprox,
}

// Draw-site slot within the PROMOTE phase for the additive breeding-value deviation, distinct
// from the default slot 0 the discrete state draw uses, so the two never collide (R-RNG-COORD).
const SLOT_ADDITIVE: u32 = 1;

/// The environmental variance V_E of the uniform developmental-environment offset (design 25.6): the
/// offset is drawn on `[-a, +a]` (`crate::world` `env_offset`), where `a` is the race's reserved
/// half-width `crate::race::Race::environment_variance`. A uniform deviate on `[-a, a]` has variance
/// `a^2 / 3`, so a caller holding the HALF-WIDTH converts through this before feeding V_E to
/// [`GenePool::narrow_sense_heritability`] (the half-width and the variance are NOT interchangeable;
/// passing the raw half-width overstates V_E). Pure fixed-point.
pub fn uniform_offset_variance(half_width: Fixed) -> Fixed {
    half_width.mul(half_width).div(Fixed::from_int(3))
}

impl GenePool {
    /// A pool over the given per-locus state-1 frequencies, with a flat additive spine (every
    /// effect zero) and the unset Gaussian sentinel. A pool built this way promotes with zero
    /// additive, exactly as before the breeding-value tier landed; give it a spine with
    /// [`GenePool::with_additive`].
    pub fn new(scheme: SchemeId, effective_size: u32, freqs: Vec<Fixed>) -> Self {
        let effects = vec![Fixed::ZERO; freqs.len()];
        GenePool {
            scheme,
            effective_size,
            freqs,
            effects,
            gauss: GaussApprox::default(),
        }
    }

    /// Attach the quantitative breeding-value spine: the per-locus average allele-substitution
    /// effects alpha_i (per-race genome data) and the stamped Gaussian approximation the spine
    /// draws through. The effects vector is truncated or zero-padded to the locus count so it stays
    /// parallel to `freqs`. This is the constructor the dawn and the pre-dawn epoch use once a race
    /// carries its effect vector; a flat pool leaves it at the default.
    pub fn with_additive(mut self, effects: Vec<Fixed>, gauss: GaussApprox) -> Self {
        let mut effects = effects;
        effects.resize(self.freqs.len(), Fixed::ZERO);
        self.effects = effects;
        self.gauss = gauss;
        self
    }

    /// The number of loci tracked.
    pub fn loci(&self) -> usize {
        self.freqs.len()
    }

    /// The state-1 frequency at a locus, or `None` if out of range.
    pub fn freq(&self, locus: usize) -> Option<Fixed> {
        self.freqs.get(locus).copied()
    }

    /// The average allele-substitution effect alpha_i at a locus, the pool's per-locus additive
    /// mean, or `None` if out of range.
    pub fn effect(&self, locus: usize) -> Option<Fixed> {
        self.effects.get(locus).copied()
    }

    /// The stamped integer-Gaussian approximation the additive spine draws through.
    pub fn gauss_approx(&self) -> GaussApprox {
        self.gauss
    }

    /// Fold the pool's canonical state into a hash in a fixed byte order (design Part 3.5), so the
    /// breeding-value tier is visible in the world's reproducible identity: the scheme, the
    /// effective size, the per-locus frequencies, the per-locus effects (the locked-representation
    /// additive spine), and the stamped Gaussian approximation. The order is the contract; the
    /// caller feeds pools in a fixed canonical order.
    pub fn hash_into(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.scheme.0);
        hasher.write_u32(self.effective_size);
        hasher.write_u32(self.freqs.len() as u32);
        for &p in &self.freqs {
            hasher.write_fixed(p);
        }
        hasher.write_u32(self.effects.len() as u32);
        for &alpha in &self.effects {
            hasher.write_fixed(alpha);
        }
        self.gauss.hash_into(hasher);
    }

    /// The additive genetic variance V_A the pool implies: the canonically-ordered sum over loci
    /// of `2 * p_i * (1 - p_i) * alpha_i^2`, the standing additive variance of a set of biallelic
    /// loci under Hardy-Weinberg and linkage equilibrium (Falconer and Mackay; Lynch and Walsh).
    /// The accumulation is order-independent in 128-bit space (like [`Fixed::sum_bits`]), so the
    /// value is the same for any partition of the loci across threads. A flat pool returns zero.
    pub fn additive_variance(&self) -> Fixed {
        let terms = self
            .freqs
            .iter()
            .zip(self.effects.iter())
            .map(|(&p, &alpha)| {
                // 2 * p * (1 - p) * alpha^2, all non-negative.
                let two_pq = Fixed::from_int(2).mul(p).mul(Fixed::ONE - p);
                two_pq.mul(alpha).mul(alpha)
            });
        Fixed::saturating_sum(terms)
    }

    /// Narrow-sense heritability, the derived read `V_A / (V_A + V_E)` (design 25.6, 25.10): the
    /// fraction of phenotypic variance that is additive-genetic and so transmitted to offspring
    /// (Falconer's offspring-on-midparent regression). `env_var` is the environmental variance V_E
    /// (a VARIANCE, not a half-width). The developmental-environment offset is a uniform deviate on
    /// `[-a, +a]` whose half-width `a` is `crate::race::Race::environment_variance`; its variance is
    /// `a^2 / 3`, NOT `a`, so a caller holding the half-width must convert through
    /// [`uniform_offset_variance`] before passing it here (the two are not interchangeable). This
    /// graduates the former authored `genome.narrow_sense_heritability` constant into a population
    /// statistic read from the pool's own effects and frequencies (Principle 11). A pool with no
    /// additive and no environmental variance returns zero (there is no heritable spread to speak
    /// of), rather than dividing by zero.
    pub fn narrow_sense_heritability(&self, env_var: Fixed) -> Fixed {
        let va = self.additive_variance();
        let denom = va + env_var;
        if denom == Fixed::ZERO {
            return Fixed::ZERO;
        }
        va.div(denom)
    }

    /// One generation of Wright-Fisher drift: each locus's frequency is resampled as the
    /// fraction of `2*Ne` gametes that carry state 1, each gamete drawn by counter-RNG
    /// against the current frequency. Exact (a sum of Bernoulli draws), integer and
    /// fixed-point with no float, so a population's drift is bit-identical. For very large
    /// Ne the exact sum is costly and the design reserves a Gaussian approximation; the
    /// method and its precision are an owner decision (25.10), so only the exact sampler is
    /// built here. A fixed locus (frequency 0 or 1) cannot drift.
    pub fn drift(&mut self, seed: u64, pool_id: u64, generation: u64) {
        let two_ne = self.effective_size.saturating_mul(2);
        if two_ne == 0 {
            return;
        }
        for (locus, p) in self.freqs.iter_mut().enumerate() {
            let rng = DrawKey::pair(pool_id, locus as u64, generation, Phase::EVOLVE).rng(seed);
            let mut count: u32 = 0;
            for k in 0..two_ne {
                if rng.unit_fixed(k as u64) < *p {
                    count += 1;
                }
            }
            *p = Fixed::from_ratio(count as i64, two_ne as i64);
        }
    }

    /// Directional selection by a per-locus selection coefficient (state 1's relative
    /// fitness is `1 + s`): `p' = p(1+s) / (1 + p*s)`. The coefficients are reserved owner
    /// values supplied as data (the selection-differential scaling of 25.7). Deterministic
    /// fixed-point with no sampling; a coefficient of zero leaves a locus unchanged.
    pub fn select(&mut self, coefficients: &[Fixed]) {
        for (p, &s) in self.freqs.iter_mut().zip(coefficients.iter()) {
            let den = Fixed::ONE + p.mul(s);
            if den != Fixed::ZERO {
                *p = p
                    .mul(Fixed::ONE + s)
                    .div(den)
                    .clamp(Fixed::ZERO, Fixed::ONE);
            }
        }
    }

    /// Fork a founder pool off this one, the founder effect (R-BIOSPHERE): each locus's
    /// frequency is resampled as the fraction of `2 * founder_size` colonising gametes that
    /// carry state 1, so drift at the small founder size shifts the daughter off the parent,
    /// and the daughter then carries `recovery_size` as its effective size (the colonisers
    /// grow into a full population). Exact per-locus Bernoulli sampling keyed on the founder
    /// id, the locus, and the generation (`Phase::FOUND`), so a deterministic pre-dawn epoch
    /// forks reproducibly. A zero founder size copies the parent frequencies unchanged.
    pub fn found(
        &self,
        seed: u64,
        founder_id: u64,
        generation: u64,
        founder_size: u32,
        recovery_size: u32,
    ) -> GenePool {
        let two_ne = founder_size.saturating_mul(2);
        let freqs = if two_ne == 0 {
            self.freqs.clone()
        } else {
            self.freqs
                .iter()
                .enumerate()
                .map(|(locus, &p)| {
                    let rng =
                        DrawKey::pair(founder_id, locus as u64, generation, Phase::FOUND).rng(seed);
                    let mut count: u32 = 0;
                    for k in 0..two_ne {
                        if rng.unit_fixed(k as u64) < p {
                            count += 1;
                        }
                    }
                    Fixed::from_ratio(count as i64, two_ne as i64)
                })
                .collect()
        };
        // The daughter inherits the parent's breeding-value spine: the same per-locus effects and
        // the same stamped approximation travel with the founders, so a forked pool stays a
        // quantitative tier and keeps the world identity.
        GenePool::new(self.scheme, recovery_size, freqs)
            .with_additive(self.effects.clone(), self.gauss)
    }

    /// A fixed-point genetic distance to another pool: the mean over shared loci of the
    /// absolute frequency difference. This is the structural divergence the speciation test
    /// reads; the exact population-genetics measure (a fixation index versus a Nei
    /// distance) is a reserved owner choice (25.7), so this interim measure stands until it
    /// is set.
    pub fn distance(&self, other: &GenePool) -> Fixed {
        let n = self.freqs.len().min(other.freqs.len());
        if n == 0 {
            return Fixed::ZERO;
        }
        let mut acc = Fixed::ZERO;
        for i in 0..n {
            let d = self.freqs[i] - other.freqs[i];
            acc += if d < Fixed::ZERO { Fixed::ZERO - d } else { d };
        }
        acc.div(Fixed::from_int(n as i32))
    }

    /// Whether two pools have diverged past a reserved speciation distance threshold, the
    /// declared-rather-than-scripted speciation of design 25.7.
    pub fn speciated(&self, other: &GenePool, threshold: Fixed) -> bool {
        self.distance(other) >= threshold
    }

    /// Promote an explicit genome from the pool (design 25.8): per locus, sample `ploidy` discrete
    /// allele states from the frequencies (Hardy-Weinberg, each allele independent) for the
    /// Mendelian layer, and assign the continuous quantitative additive spine so a promoted cohort
    /// reconstructs the pool's additive genetic variance. Keyed on the new being's id so the
    /// individual is reproducible and statistically consistent with the pool.
    ///
    /// The additive spine, per locus, is the pool's per-locus additive mean (the effect alpha_i)
    /// plus a mean-zero within-locus deviation scaled to the locus additive standard deviation
    /// `sqrt(2 p (1 - p)) * |alpha|` (one square root per locus, never in a sub-draw loop), drawn
    /// through the stamped Gaussian approximation on a slot distinct from the state draw. The
    /// whole-locus deviation is split evenly across the ploidy alleles, so `express()` sums to it.
    /// Across promoted individuals the additive spine's variance reconstructs
    /// [`GenePool::additive_variance`] (the tier-consistency invariant), while its mean matches the
    /// pool, so a promote-then-demote round trip is unbiased. A zero-effect locus carries no
    /// additive and draws nothing, so a flat pool with an unset stamp promotes exactly as before.
    pub fn promote(&self, seed: u64, individual_id: u64, ploidy: usize) -> Genome {
        let ploidy_fx = Fixed::from_int(ploidy.max(1) as i32);
        // The per-allele additive share at each locus (identical across the haplotypes of one
        // locus): the mean effect alpha plus the split within-locus Gaussian deviation.
        let per_allele_additive: Vec<Fixed> = self
            .freqs
            .iter()
            .enumerate()
            .map(|(locus, &p)| {
                let alpha = self.effects.get(locus).copied().unwrap_or(Fixed::ZERO);
                if alpha == Fixed::ZERO {
                    return Fixed::ZERO;
                }
                let g = gaussian_unit(
                    &DrawKey::pair(individual_id, locus as u64, 0, Phase::PROMOTE)
                        .slot(SLOT_ADDITIVE)
                        .rng(seed),
                    0,
                    self.gauss,
                );
                // Locus additive standard deviation sqrt(2 p (1 - p)) * |alpha|.
                let two_pq = Fixed::from_int(2).mul(p).mul(Fixed::ONE - p);
                let sigma = two_pq.sqrt().mul(alpha.abs());
                alpha + sigma.mul(g).div(ploidy_fx)
            })
            .collect();
        let mut haps = Vec::with_capacity(ploidy);
        for h in 0..ploidy {
            let state_rng = DrawKey::pair(individual_id, h as u64, 0, Phase::PROMOTE).rng(seed);
            let alleles = self
                .freqs
                .iter()
                .enumerate()
                .map(|(locus, &p)| {
                    let s = if state_rng.unit_fixed(locus as u64) < p {
                        1u16
                    } else {
                        0
                    };
                    Allele {
                        additive: per_allele_additive[locus],
                        state: AlleleState(s),
                        origin: individual_id as u32,
                    }
                })
                .collect();
            haps.push(Haplotype { alleles });
        }
        Genome {
            scheme: self.scheme,
            haps,
        }
    }

    /// Fold a demoted individual's genotype back into the pool (design 25.8): each locus's
    /// frequency moves toward the individual's state-1 fraction, and the pool's per-locus additive
    /// mean (the effect alpha_i) moves toward the individual's per-locus additive sum, both by the
    /// same `2Ne`-weighted running-mean update, canonically ordered, with fixed rounding. Because
    /// [`GenePool::promote`] centres a promoted individual's per-locus additive sum on the pool
    /// mean, the additive fold is unbiased: a promote-then-demote round trip leaves the additive
    /// mean unchanged in expectation. Linkage disequilibrium and family structure are lost, the
    /// documented cost of demotion.
    pub fn demote(&mut self, genome: &Genome) {
        let two_ne = (self.effective_size.saturating_mul(2)).min(i32::MAX as u32) as i32;
        let two_ne_fx = Fixed::from_int(two_ne);
        for locus in 0..self.freqs.len() {
            let mut total: i32 = 0;
            let mut ones: i32 = 0;
            let mut additive_sum = Fixed::ZERO;
            for hap in &genome.haps {
                if let Some(a) = hap.alleles.get(locus) {
                    total += 1;
                    if a.state == AlleleState(1) {
                        ones += 1;
                    }
                    additive_sum += a.additive;
                }
            }
            if total == 0 {
                continue;
            }
            let denom = Fixed::from_int(two_ne + total);
            // Frequency fold (unchanged semantics): toward the individual's state-1 fraction.
            let p = self.freqs[locus];
            let numerator = p.mul(two_ne_fx) + Fixed::from_int(ones);
            self.freqs[locus] = numerator.div(denom).clamp(Fixed::ZERO, Fixed::ONE);
            // Additive-mean fold: toward the individual's per-locus additive sum. The individual
            // contributes `total` allele copies whose sum has expectation `total * alpha`, so the
            // `2Ne`-weighted mean leaves alpha unchanged in expectation.
            let alpha = self.effects[locus];
            self.effects[locus] = (alpha.mul(two_ne_fx) + additive_sum).div(denom);
        }
    }

    /// Whether the pool carries an allele state at a locus at or above a presence threshold:
    /// state 1 is carried when its frequency meets `presence`, state 0 when its complement
    /// does. The pool tracks the binary Mendelian view (state 0 versus state 1); any other
    /// state is never carried at the pool tier (multi-allele pools are deferred, 25.10), so
    /// such a query is `false` rather than an error.
    pub fn carries(&self, locus: usize, state: AlleleState, presence: Fixed) -> bool {
        match self.freqs.get(locus) {
            None => false,
            Some(&p) => match state.0 {
                1 => p >= presence,
                0 => (Fixed::ONE - p) >= presence,
                _ => false,
            },
        }
    }

    /// Whether this pool is reproductively isolated from another (the declared speciation of
    /// design 25.7, on the owner's chosen rule: distance and incompatibilities together). It
    /// is isolated when the frequency distance has diverged past `dist_threshold`, or when
    /// the count of Dobzhansky-Muller incompatibilities active across the two pools reaches
    /// `incompat_threshold`. The first captures gradual divergence, the second the discrete
    /// genetic firewall a single complementary allele pair can raise even between otherwise
    /// close pools. Both the distance threshold and the count are reserved owner values; the
    /// allele-presence threshold is the data fed to [`IncompatibilityTable::active_between`].
    pub fn reproductively_isolated(
        &self,
        other: &GenePool,
        dist_threshold: Fixed,
        table: &IncompatibilityTable,
        incompat_threshold: usize,
        presence: Fixed,
    ) -> bool {
        self.distance(other) >= dist_threshold
            || table.active_between(self, other, presence) >= incompat_threshold
    }
}

/// How a Dobzhansky-Muller incompatibility bites when its two alleles meet in one genome.
/// These are the fixed mechanism affordances (the ways an incompatibility can express), on
/// the same footing as [`DominanceKind`] and the force kinds: the kinds are fixed Rust, the
/// membership of the [`IncompatibilityTable`] is data and grows with the world (Principle
/// 11). A genome that carries both partner alleles of a sterilizing pair develops but cannot
/// breed; of a lethal pair, does not develop.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum IncompatibilityKind {
    /// The hybrid develops but is sterile.
    Sterilizing,
    /// The hybrid is inviable.
    Lethal,
}

/// The outcome of forming a hybrid genome against an [`IncompatibilityTable`]: viable and
/// fertile, viable but sterile, or inviable. Ordered by severity so the worst active
/// incompatibility governs the genome's outcome.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum HybridOutcome {
    /// No active incompatibility; the hybrid is viable and fertile.
    Viable,
    /// At least one sterilizing incompatibility is active; viable but cannot breed.
    Sterile,
    /// At least one lethal incompatibility is active; inviable.
    Inviable,
}

/// One Dobzhansky-Muller incompatibility: an ordered pair of allele states at two loci that
/// is benign apart but deleterious when both are present in one genome (design 25.7). The
/// canonical case is two lineages that each fix a different derived allele from a shared
/// ancestor; neither lineage suffers, but a hybrid inherits both and pays the cost.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Incompatibility {
    /// The first locus.
    pub locus_a: u32,
    /// The deleterious state at the first locus.
    pub state_a: AlleleState,
    /// The second locus.
    pub locus_b: u32,
    /// The deleterious state at the second locus.
    pub state_b: AlleleState,
    /// How the incompatibility bites when both states co-occur.
    pub kind: IncompatibilityKind,
}

/// The set of Dobzhansky-Muller incompatibilities in play (design 25.7). This is the data
/// registry, sibling to the gene set and the genetic scheme: the mechanism that reads it is
/// fixed Rust, the pairs are data and grow as lineages accumulate divergence (Principle 11).
/// Empty by default, so a world with no declared incompatibilities falls back to the pure
/// distance test for speciation.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct IncompatibilityTable {
    /// The incompatible allele pairs.
    pub pairs: Vec<Incompatibility>,
}

impl IncompatibilityTable {
    /// An empty table.
    pub fn new() -> Self {
        IncompatibilityTable { pairs: Vec::new() }
    }

    /// A table over the given pairs.
    pub fn with(pairs: Vec<Incompatibility>) -> Self {
        IncompatibilityTable { pairs }
    }

    /// Register one incompatibility.
    pub fn add(&mut self, pair: Incompatibility) {
        self.pairs.push(pair);
    }

    /// The outcome of an explicit hybrid genome: the most severe incompatibility whose both
    /// partner alleles the genome carries on any haplotype governs the result. A genome that
    /// carries neither side, or only one side, of every pair is [`HybridOutcome::Viable`].
    pub fn hybrid_outcome(&self, genome: &Genome) -> HybridOutcome {
        let mut worst = HybridOutcome::Viable;
        for pair in &self.pairs {
            if genome_carries(genome, pair.locus_a, pair.state_a)
                && genome_carries(genome, pair.locus_b, pair.state_b)
            {
                let outcome = match pair.kind {
                    IncompatibilityKind::Sterilizing => HybridOutcome::Sterile,
                    IncompatibilityKind::Lethal => HybridOutcome::Inviable,
                };
                if outcome > worst {
                    worst = outcome;
                }
            }
        }
        worst
    }

    /// The count of incompatibilities active across two pools: a pair is active when the two
    /// deleterious alleles are partitioned across the pools, one pool carrying `state_a` at
    /// `locus_a` while the other carries `state_b` at `locus_b` (or the mirror). This is the
    /// Dobzhansky-Muller signature: each pool is internally consistent, but a cross would
    /// unite the two alleles in one hybrid. The count is what [`GenePool::reproductively_
    /// isolated`] compares against the reserved incompatibility threshold.
    ///
    /// The activation test is the joint Hardy-Weinberg cross probability, not a marginal-AND of two
    /// presence cutoffs: a pair is active when the product of the two pools' allele frequencies (the
    /// probability under linkage equilibrium that a hybrid draws `state_a` from one parent and
    /// `state_b` from the other) reaches `presence`. So the threshold now gates the joint chance a
    /// cross actually unites the pair rather than each allele being common on its own, and two pools
    /// that each merely carry one deleterious allele are isolated only to the extent a hybrid is
    /// likely to inherit both. A pure, deterministic fixed-point function of the frequencies.
    pub fn active_between(&self, a: &GenePool, b: &GenePool, presence: Fixed) -> usize {
        self.pairs
            .iter()
            .filter(|pair| {
                // The joint Hardy-Weinberg cross probability of a partition: the chance a hybrid
                // draws state_a from the first pool and state_b from the second, the product of the
                // two allele frequencies under linkage equilibrium.
                let cross = |first: &GenePool, second: &GenePool| {
                    let pa = state_freq(first, pair.locus_a, pair.state_a);
                    let pb = state_freq(second, pair.locus_b, pair.state_b);
                    pa.mul(pb)
                };
                // Active when either partition (forward or its mirror) clears the presence gate.
                cross(a, b) >= presence || cross(b, a) >= presence
            })
            .count()
    }
}

/// The Hardy-Weinberg frequency of an allele state at a locus in a pool: the state-1 frequency for
/// state 1, its complement for state 0, and zero for any other state (multi-allele pools are
/// deferred, 25.10) or an out-of-range locus. The frequency the joint cross-probability product in
/// [`IncompatibilityTable::active_between`] multiplies.
fn state_freq(pool: &GenePool, locus: u32, state: AlleleState) -> Fixed {
    match pool.freq(locus as usize) {
        None => Fixed::ZERO,
        Some(p) => match state.0 {
            1 => p,
            0 => Fixed::ONE - p,
            _ => Fixed::ZERO,
        },
    }
}

/// Whether a genome carries an allele state at a locus on any haplotype.
fn genome_carries(genome: &Genome, locus: u32, state: AlleleState) -> bool {
    genome.haps.iter().any(|h| {
        h.alleles
            .get(locus as usize)
            .is_some_and(|a| a.state == state)
    })
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
            additive_mutation_step: Fixed::ZERO,
            gauss: GaussApprox::default(),
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

    // --- The aggregate tier: pools and deep-time evolution (design 25.7, 25.8) ---

    #[test]
    fn drift_leaves_a_fixed_locus_and_stays_in_range_and_replays() {
        // A locus fixed at 0 or 1 cannot drift; a polymorphic locus drifts within [0,1] and
        // replays bit for bit from the seed.
        let mut p = GenePool::new(
            SCHEME,
            8,
            vec![Fixed::ZERO, Fixed::ONE, Fixed::from_ratio(1, 2)],
        );
        let before = p.clone();
        p.drift(0xD1F7, 1, 0);
        assert_eq!(
            p.freq(0),
            Some(Fixed::ZERO),
            "a locus fixed at 0 stays fixed"
        );
        assert_eq!(
            p.freq(1),
            Some(Fixed::ONE),
            "a locus fixed at 1 stays fixed"
        );
        let mid = p.freq(2).unwrap();
        assert!(
            mid >= Fixed::ZERO && mid <= Fixed::ONE,
            "drift stays in range"
        );
        // Replay: the same pool drifts identically.
        let mut q = before;
        q.drift(0xD1F7, 1, 0);
        assert_eq!(p, q, "drift replays bit for bit");
    }

    #[test]
    fn selection_pushes_frequency_and_zero_leaves_it() {
        let mut p = GenePool::new(
            SCHEME,
            100,
            vec![Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2)],
        );
        // A positive coefficient on locus 0, none on locus 1.
        p.select(&[Fixed::from_ratio(1, 2), Fixed::ZERO]);
        assert!(
            p.freq(0).unwrap() > Fixed::from_ratio(1, 2),
            "selection raised state 1"
        );
        assert_eq!(
            p.freq(1),
            Some(Fixed::from_ratio(1, 2)),
            "no coefficient, no change"
        );
    }

    #[test]
    fn founder_fork_drifts_off_the_parent_and_recovers_size() {
        let parent = GenePool::new(SCHEME, 500, vec![Fixed::from_ratio(1, 2); 8]);
        // A small founder size (5) drifts the daughter off the 0.5 parent frequencies; the
        // daughter carries the recovery size.
        let d = parent.found(0xF00D, 42, 3, 5, 200);
        assert_eq!(d.loci(), parent.loci());
        assert_eq!(
            d.effective_size, 200,
            "the daughter recovers to the recovery size"
        );
        let moved = (0..8).any(|i| d.freq(i) != parent.freq(i));
        assert!(moved, "founder drift at a small size shifts some frequency");
        // Deterministic: the same fork replays identically.
        let d2 = parent.found(0xF00D, 42, 3, 5, 200);
        assert_eq!(d, d2, "a founder-fork is a pure function of the key");
        // A fixed locus stays fixed through the fork.
        let fixed_parent = GenePool::new(SCHEME, 500, vec![Fixed::ONE, Fixed::ZERO]);
        let fd = fixed_parent.found(0xF00D, 1, 0, 5, 200);
        assert_eq!(fd.freq(0), Some(Fixed::ONE));
        assert_eq!(fd.freq(1), Some(Fixed::ZERO));
    }

    #[test]
    fn distance_is_zero_for_identical_and_one_for_opposite() {
        let a = GenePool::new(SCHEME, 10, vec![Fixed::ONE, Fixed::ZERO]);
        let same = a.clone();
        let opposite = GenePool::new(SCHEME, 10, vec![Fixed::ZERO, Fixed::ONE]);
        assert_eq!(a.distance(&same), Fixed::ZERO);
        assert_eq!(a.distance(&opposite), Fixed::ONE);
        assert!(a.speciated(&opposite, Fixed::from_ratio(1, 2)));
        assert!(!a.speciated(&same, Fixed::from_ratio(1, 2)));
    }

    #[test]
    fn promotion_samples_a_genome_and_replays() {
        // A pool fixed at all-state-1 promotes a diploid genome of all state 1.
        let pool = GenePool::new(SCHEME, 50, vec![Fixed::ONE, Fixed::ONE, Fixed::ONE]);
        let g = pool.promote(0xBEEF, 7, 2);
        assert_eq!(g.ploidy(), 2);
        assert!(g
            .haps
            .iter()
            .all(|h| h.alleles.iter().all(|a| a.state == AlleleState(1))));
        // Same id and seed reproduce the same individual.
        let again = pool.promote(0xBEEF, 7, 2);
        assert_eq!(g, again, "promotion replays bit for bit");
    }

    #[test]
    fn demotion_folds_a_genotype_back_toward_its_states() {
        // A pool at frequency 1/2 that absorbs an all-state-1 individual moves upward.
        let mut pool = GenePool::new(SCHEME, 4, vec![Fixed::from_ratio(1, 2)]);
        let individual = Genome {
            scheme: SCHEME,
            haps: vec![
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::ZERO,
                        state: AlleleState(1),
                        origin: 0,
                    }],
                },
                Haplotype {
                    alleles: vec![Allele {
                        additive: Fixed::ZERO,
                        state: AlleleState(1),
                        origin: 0,
                    }],
                },
            ],
        };
        pool.demote(&individual);
        // (1/2 * 8 + 2) / (8 + 2) = 6/10 = 3/5 > 1/2.
        assert_eq!(pool.freq(0), Some(Fixed::from_ratio(3, 5)));
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

    fn dm_table() -> IncompatibilityTable {
        IncompatibilityTable::with(vec![Incompatibility {
            locus_a: 0,
            state_a: AlleleState(1),
            locus_b: 1,
            state_b: AlleleState(1),
            kind: IncompatibilityKind::Lethal,
        }])
    }

    #[test]
    fn an_incompatibility_is_active_only_across_the_partition() {
        // Lineage A fixes the derived allele at locus 0, lineage B at locus 1; neither pool
        // carries both, so the pair is the Dobzhansky-Muller signature: active across the
        // cross, dormant within either pool.
        let presence = Fixed::from_ratio(9, 10);
        let a = GenePool::new(SchemeId(1), 10, vec![Fixed::ONE, Fixed::ZERO]);
        let b = GenePool::new(SchemeId(1), 10, vec![Fixed::ZERO, Fixed::ONE]);
        let table = dm_table();
        assert_eq!(
            table.active_between(&a, &b, presence),
            1,
            "active across the cross"
        );
        assert_eq!(
            table.active_between(&a, &a, presence),
            0,
            "dormant within a pool"
        );
        assert_eq!(
            table.active_between(&b, &b, presence),
            0,
            "dormant within a pool"
        );
    }

    #[test]
    fn isolation_fires_on_incompatibility_even_when_distance_is_short() {
        // The discrete firewall: two pools below the distance threshold are still isolated if
        // the incompatibility count reaches its threshold. The pools share eight loci and
        // diverge only at the two incompatible loci, so their mean distance (0.2) is short
        // while the partitioned allele pair is fixed past the presence threshold.
        let presence = Fixed::from_ratio(9, 10);
        let far = Fixed::from_ratio(1, 2); // a distance threshold the pools do not reach
        let half = Fixed::from_ratio(1, 2);
        let mut a_freqs = vec![Fixed::ONE, Fixed::ZERO];
        a_freqs.extend(std::iter::repeat_n(half, 8));
        let mut b_freqs = vec![Fixed::ZERO, Fixed::ONE];
        b_freqs.extend(std::iter::repeat_n(half, 8));
        let a = GenePool::new(SchemeId(1), 10, a_freqs);
        let b = GenePool::new(SchemeId(1), 10, b_freqs);
        let table = dm_table();
        assert!(
            a.distance(&b) < far,
            "the pools are within the distance threshold"
        );
        assert!(
            a.reproductively_isolated(&b, far, &table, 1, presence),
            "one active incompatibility isolates the pools"
        );
        assert!(
            !a.reproductively_isolated(&b, far, &IncompatibilityTable::new(), 1, presence),
            "with no incompatibilities and short distance the pools are not isolated"
        );
    }

    #[test]
    fn a_dm_pair_counts_by_the_joint_cross_probability_not_the_marginals() {
        // The joint Hardy-Weinberg replacement for the marginal-AND: two pools that each merely
        // carry the deleterious allele at an intermediate frequency are active only to the extent a
        // hybrid is likely to inherit both, the product of the two frequencies gated by presence.
        let table = dm_table(); // locus 0 state 1 crossed with locus 1 state 1
        let presence = Fixed::from_ratio(1, 2);
        // Each pool carries its allele at 0.6. The marginal-AND would count this active (0.6 and
        // 0.6 both clear 0.5), but the joint cross probability is 0.36 < 0.5, so it is dormant.
        let a = GenePool::new(SchemeId(1), 10, vec![Fixed::from_ratio(3, 5), Fixed::ZERO]);
        let b = GenePool::new(SchemeId(1), 10, vec![Fixed::ZERO, Fixed::from_ratio(3, 5)]);
        assert_eq!(
            table.active_between(&a, &b, presence),
            0,
            "a joint cross probability of 0.36 is below the 0.5 presence gate"
        );
        // Raise both to 0.8: the product 0.64 clears the gate, so the pair is active.
        let a2 = GenePool::new(SchemeId(1), 10, vec![Fixed::from_ratio(4, 5), Fixed::ZERO]);
        let b2 = GenePool::new(SchemeId(1), 10, vec![Fixed::ZERO, Fixed::from_ratio(4, 5)]);
        assert_eq!(
            table.active_between(&a2, &b2, presence),
            1,
            "0.64 clears the gate"
        );
        // A fully-fixed partition (the classic DM signature) is a product of 1.0, always active.
        let fa = GenePool::new(SchemeId(1), 10, vec![Fixed::ONE, Fixed::ZERO]);
        let fb = GenePool::new(SchemeId(1), 10, vec![Fixed::ZERO, Fixed::ONE]);
        assert_eq!(table.active_between(&fa, &fb, presence), 1);
        // The mirror partition fires too: b carries locus 0, a carries locus 1.
        let ma = GenePool::new(SchemeId(1), 10, vec![Fixed::ZERO, Fixed::from_ratio(4, 5)]);
        let mb = GenePool::new(SchemeId(1), 10, vec![Fixed::from_ratio(4, 5), Fixed::ZERO]);
        assert_eq!(
            table.active_between(&ma, &mb, presence),
            1,
            "the mirror partition clears the gate"
        );
        // Determinism: the joint-product count replays bit for bit.
        assert_eq!(
            table.active_between(&a2, &b2, presence),
            table.active_between(&a2, &b2, presence)
        );
    }

    #[test]
    fn a_hybrid_genome_takes_the_worst_active_outcome() {
        // A hybrid carrying both partner alleles is governed by the most severe pair.
        let mut table = IncompatibilityTable::with(vec![Incompatibility {
            locus_a: 0,
            state_a: AlleleState(1),
            locus_b: 1,
            state_b: AlleleState(1),
            kind: IncompatibilityKind::Sterilizing,
        }]);
        let one = Allele {
            additive: Fixed::ZERO,
            state: AlleleState(1),
            origin: 0,
        };
        let zero = Allele {
            additive: Fixed::ZERO,
            state: AlleleState(0),
            origin: 0,
        };
        let hybrid = Genome {
            scheme: SchemeId(1),
            haps: vec![Haplotype {
                alleles: vec![one, one],
            }],
        };
        let carrier_one_side = Genome {
            scheme: SchemeId(1),
            haps: vec![Haplotype {
                alleles: vec![one, zero],
            }],
        };
        assert_eq!(table.hybrid_outcome(&hybrid), HybridOutcome::Sterile);
        assert_eq!(
            table.hybrid_outcome(&carrier_one_side),
            HybridOutcome::Viable,
            "one side alone is benign"
        );
        table.add(Incompatibility {
            locus_a: 0,
            state_a: AlleleState(1),
            locus_b: 1,
            state_b: AlleleState(1),
            kind: IncompatibilityKind::Lethal,
        });
        assert_eq!(
            table.hybrid_outcome(&hybrid),
            HybridOutcome::Inviable,
            "the lethal pair dominates the sterilizing one"
        );
    }

    #[test]
    fn pool_carries_reads_both_states_against_presence() {
        let presence = Fixed::from_ratio(9, 10);
        let pool = GenePool::new(SchemeId(1), 10, vec![Fixed::ONE, Fixed::ZERO]);
        assert!(pool.carries(0, AlleleState(1), presence), "state 1 fixed");
        assert!(!pool.carries(0, AlleleState(0), presence), "state 0 absent");
        assert!(pool.carries(1, AlleleState(0), presence), "state 0 fixed");
        assert!(!pool.carries(1, AlleleState(1), presence), "state 1 absent");
        assert!(
            !pool.carries(0, AlleleState(2), presence),
            "multi-allele not at pool tier"
        );
        assert!(
            !pool.carries(9, AlleleState(1), presence),
            "out of range is false"
        );
    }
}
