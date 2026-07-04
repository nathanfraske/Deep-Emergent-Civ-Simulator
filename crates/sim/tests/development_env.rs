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

//! The per-being developmental-environment offset: the environmental-variance (V_E) half of
//! narrow-sense heritability (design Part 25.6). Today every member of a race rides one shared
//! `race.environment`, so the environmental variance across a cohort is identically zero. The
//! offset gives each being a mean-zero symmetric developmental deviation keyed on its id under
//! `Phase::DEVELOPMENT`, so `express()` varies per individual and V_E is positive, while the
//! symmetry keeps it from shifting any cohort mean (Principle 9). These tests prove: replay
//! determinism at a positive spread; that genetic divergence between races survives V_E without
//! the spread erasing the between-race difference (the non-steering divergence check); that the
//! variance decomposition becomes non-trivial (V_P == V_A at spread zero, V_P > V_A at spread
//! above zero); that the mean is preserved (mean-zero adds variance, not direction) and the
//! offset is an odd reflection; that a zero spread reproduces the pre-offset dawn state hash bit
//! for bit; and that two siblings recombined identically draw distinct offsets.

use std::collections::BTreeMap;

use civsim_core::{DrawKey, Fixed, Phase, StableId};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, BandSpec, Channel, CognitionChannel, Curve, DominanceKind,
    DominanceMode, EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet,
    GeneticScheme, InferenceParams, IntrinsicBeliefs, Race, RaceId, ReproductionMode,
    RingCapacityLaw, SchemeId, SourceModeId, ValueAxisId, ValueProfile, World,
};

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

/// A labelled test ring-capacity law (not owner data), the same shape the dawn tests use.
fn dev_ring_law() -> RingCapacityLaw {
    RingCapacityLaw {
        curve: Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::from_int(8), Fixed::from_int(16)),
        ]),
        hard_cap: 32,
    }
}

/// A race whose ReasoningAcuity gene carries a heterozygote dominance deviation `acuity_d` over a
/// polymorphic locus, so a pool-promoted cohort spreads across expressed acuity (a member
/// heterozygous at the acuity locus expresses the deviation, a homozygous one expresses only the
/// environment baseline): that is the genetic variance V_A the decomposition needs. The
/// environment baseline is 2 for every member, and `env_var` is the reserved V_E half-width the
/// developmental offset uses. Everything else is held identical, so acuity differences trace to
/// the acuity gene's dominance and the developmental offset alone (labelled fixtures, not owner
/// data).
fn race(id: u32, acuity_d: Fixed, env_var: Fixed) -> Race {
    let genes = GeneSet {
        genes: vec![
            GeneDef {
                id: GeneId(0),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::ReasoningAcuity),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode {
                    a: Fixed::ZERO,
                    d: acuity_d,
                    kind: DominanceKind::Over,
                },
            },
            GeneDef {
                id: GeneId(1),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::MemoryCapacity),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            },
        ],
    };
    let pool = GenePool::new(
        SchemeId(0),
        20,
        vec![Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2)],
    );
    let intrinsic = IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 3)]),
        axioms: vec![Axiom {
            axis: AxiomAxisId(0),
            stance: Fixed::from_ratio(1, 2),
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 5,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: Fixed::from_ratio(1, 2),
            evidence: EvidenceRing::new(4),
        }],
        epistemic: EpistemicStance::new(
            [(SourceModeId(1), Fixed::ONE)],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        ),
    };
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: Fixed::ZERO,
        additive_mutation_step: Fixed::ZERO,
        gauss: civsim_core::GaussApprox::default(),
    };
    Race::new(
        RaceId(id),
        genes,
        pool,
        scheme,
        intrinsic,
        Fixed::from_int(2),
        env_var,
        80,
        18,
    )
}

/// Seed a single-band cohort of one race and return the world and the seeded ids.
fn seed_cohort(seed: u64, r: Race, place: u32, members: usize) -> (World, Vec<StableId>) {
    let mut races = BTreeMap::new();
    races.insert(r.id, r);
    let bands = [BandSpec {
        race: RaceId(races.keys().next().unwrap().0),
        place,
        members,
    }];
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(seed);
    let ids = w.seed_dawn_populations(&races, &bands, &dev_ring_law());
    (w, ids)
}

/// The expressed reasoning acuity (the full phenotype: genetics, environment baseline, and the
/// developmental offset) of every seeded member, in seeding order.
fn cohort_acuity(w: &World, ids: &[StableId]) -> Vec<Fixed> {
    ids.iter()
        .map(|&id| w.mind(id).expect("a mind was seeded").acuity)
        .collect()
}

/// The genotypic value of every member on the ReasoningAcuity channel, environment stripped
/// (expressed at a zero environment), so its variance is the additive-and-dominance genetic
/// variance V_A. `express` adds the environment at the start, so this is exactly the phenotype
/// minus the shared baseline: a constant shift that does not change the variance.
fn cohort_genotypic_acuity(w: &World, ids: &[StableId], r: &Race) -> Vec<Fixed> {
    ids.iter()
        .map(|&id| {
            let g = w.genome_of(id).expect("a genome was seeded");
            r.genes.express(
                g,
                Channel::Cognition(CognitionChannel::ReasoningAcuity),
                Fixed::ZERO,
            )
        })
        .collect()
}

fn mean(xs: &[Fixed]) -> Fixed {
    let sum = xs.iter().copied().fold(Fixed::ZERO, |a, b| a + b);
    sum.div(Fixed::from_int(xs.len() as i32))
}

fn variance(xs: &[Fixed]) -> Fixed {
    let m = mean(xs);
    let ss = xs
        .iter()
        .copied()
        .fold(Fixed::ZERO, |acc, x| acc + (x - m).mul(x - m));
    ss.div(Fixed::from_int(xs.len() as i32))
}

/// The developmental offset formula, replicated for the symmetry assertion: a mean-zero deviation
/// `(2 * unit - 1) * spread`, the same map `world::env_offset` applies.
fn offset(unit: Fixed, spread: Fixed) -> Fixed {
    (Fixed::from_int(2).mul(unit) - Fixed::ONE).mul(spread)
}

// (1) Replay determinism: two seedings on one seed at a positive spread produce bit-identical
// minds, and the spread is doing real work (the cohort is not all one acuity).
#[test]
fn a_positive_spread_replays_bit_for_bit() {
    let spread = Fixed::from_int(1);
    let build = || seed_cohort(0x5EED_25E6, race(0, Fixed::from_int(4), spread), 3, 64);
    let (wa, ida) = build();
    let (wb, idb) = build();
    assert_eq!(
        wa.state_hash(),
        wb.state_hash(),
        "the same seed and spread reproduce the same canonical world"
    );
    assert_eq!(ida, idb, "the same seeding order");
    let acu = cohort_acuity(&wa, &ida);
    assert!(
        acu.iter().any(|&a| a != acu[0]),
        "a positive spread makes the cohort's expressed acuity vary (V_E > 0)"
    );
}

// (2) The non-steering divergence test: two races with divergent acuity genes, each seeded with a
// positive spread. Their cohort mean acuity still differs (genetic divergence survives V_E), and
// the separation between the means at a positive spread stays within tolerance of the separation
// at spread zero (V_E widens each cohort without erasing the between-race difference or steering
// it).
#[test]
fn genetic_divergence_survives_v_e_without_the_spread_erasing_it() {
    let spread = Fixed::from_int(1);
    let members = 256;
    // Divergent gene sets: race A's acuity gene has a larger heterozygote deviation than B's, so
    // the cohorts pull apart genetically. Same pool, seed, and band, so the two cohorts share the
    // same genomes and the separation is forced through the acuity gene alone.
    let (wa0, ida0) = seed_cohort(
        0xD1_7E00,
        race(0, Fixed::from_int(4), Fixed::ZERO),
        1,
        members,
    );
    let (wb0, idb0) = seed_cohort(
        0xD1_7E00,
        race(1, Fixed::from_int(1), Fixed::ZERO),
        1,
        members,
    );
    let (wa, ida) = seed_cohort(0xD1_7E00, race(0, Fixed::from_int(4), spread), 1, members);
    let (wb, idb) = seed_cohort(0xD1_7E00, race(1, Fixed::from_int(1), spread), 1, members);

    let mean_a = mean(&cohort_acuity(&wa, &ida));
    let mean_b = mean(&cohort_acuity(&wb, &idb));
    assert_ne!(
        mean_a, mean_b,
        "the between-race genetic difference survives the developmental variance"
    );

    let sep_spread = (mean_a - mean_b).abs();
    let sep_zero = (mean(&cohort_acuity(&wa0, &ida0)) - mean(&cohort_acuity(&wb0, &idb0))).abs();
    let drift = (sep_spread - sep_zero).abs();
    let tol = Fixed::from_ratio(1, 4);
    assert!(
        drift <= tol,
        "V_E must not steer the between-race separation: |{sep_spread:?} - {sep_zero:?}| = {drift:?} exceeds {tol:?}"
    );
    assert!(
        sep_zero > tol,
        "the genetic separation is real and larger than the tolerance it is checked against"
    );
}

// (3) The decomposition is trivial at spread zero and non-trivial above it: V_P is the variance of
// the expressed phenotype, V_A the variance of the genotypic value (environment stripped). At
// spread zero the phenotype is the genotypic value plus a shared constant, so V_P == V_A (h2 == 1);
// a positive spread adds environmental variance, so V_P > V_A (h2 < 1).
#[test]
fn the_spread_makes_the_variance_decomposition_non_trivial() {
    let members = 256;
    let r0 = race(0, Fixed::from_int(4), Fixed::ZERO);
    let (w0, id0) = seed_cohort(0x5E9E_0001, r0.clone(), 5, members);
    let v_p0 = variance(&cohort_acuity(&w0, &id0));
    let v_a0 = variance(&cohort_genotypic_acuity(&w0, &id0, &r0));
    assert_eq!(
        v_p0, v_a0,
        "at spread zero the phenotype variance equals the genotypic variance (heritability one)"
    );
    assert!(
        v_a0 > Fixed::ZERO,
        "the fixture carries real genetic variance"
    );

    let spread = Fixed::from_int(1);
    let rs = race(0, Fixed::from_int(4), spread);
    let (ws, ids) = seed_cohort(0x5E9E_0001, rs.clone(), 5, members);
    let v_ps = variance(&cohort_acuity(&ws, &ids));
    let v_as = variance(&cohort_genotypic_acuity(&ws, &ids, &rs));
    assert_eq!(
        v_as, v_a0,
        "the genotypic variance is unchanged: the offset is environmental, not genetic"
    );
    assert!(
        v_ps > v_as,
        "a positive spread adds environmental variance, so V_P ({v_ps:?}) exceeds V_A ({v_as:?})"
    );
}

// (4) Mean preservation and reflection symmetry. Over a large cohort the mean expressed phenotype
// at a positive spread stays within tolerance of the mean at spread zero (the genomes are
// identical across the two runs, so the whole mean difference is the sample mean of a mean-zero
// draw). And the offset is an odd reflection about the unit midpoint: reflecting the unit draw to
// its complement negates the offset exactly (at unit spread the map is loss-free), so it authors
// variance without a direction.
#[test]
fn the_offset_preserves_the_mean_and_is_an_odd_reflection() {
    let members = 1024;
    let spread = Fixed::from_int(1);
    let (w0, id0) = seed_cohort(
        0x0FF5_E700,
        race(0, Fixed::from_int(4), Fixed::ZERO),
        2,
        members,
    );
    let (ws, ids) = seed_cohort(0x0FF5_E700, race(0, Fixed::from_int(4), spread), 2, members);
    let mean0 = mean(&cohort_acuity(&w0, &id0));
    let means = mean(&cohort_acuity(&ws, &ids));
    let shift = (means - mean0).abs();
    let tol = Fixed::from_ratio(1, 8);
    assert!(
        shift <= tol,
        "mean-zero adds variance, not direction: the mean shifted by {shift:?}, over {tol:?}"
    );

    // Reflection symmetry, tied to an actual DEVELOPMENT draw. At unit spread the map is exact, so
    // negating the (reflected) unit draw negates the offset with no rounding slack.
    for id in [1u64, 7, 100, 65_535] {
        let unit = DrawKey::entity(id, 0, Phase::DEVELOPMENT)
            .rng(0x0FF5_E700)
            .unit_fixed(0);
        let reflected = Fixed::ONE - unit;
        assert_eq!(
            offset(unit, Fixed::ONE),
            -offset(reflected, Fixed::ONE),
            "the offset is odd about the unit midpoint (mean-zero symmetry)"
        );
    }
}

// (5) Backward compatibility: a zero environment_variance reproduces the pre-offset dawn state
// hash exactly. The golden constant is captured with this same fixture, seed, and band; a zero
// spread makes the offset exactly zero, so the seeded world must fold to the identical canonical
// hash. The golden is refreshed whenever World::state_hash folds more canonical state: first the
// life-cadence period and mortality-hazard curve (an earlier defect), then each being's age and
// personality trait trajectory and each lineage's race maturity and lifespan (blind-audit defect 7),
// and now the aggregate belief pools (the belief-diffusion beat, world-wiring increment 4), which
// fold an empty (length-zero) pool set here. The zero-variance backward-compat property (a zero
// spread reproduces the no-offset dawn) is unchanged; only the folded field set grew.
#[test]
fn zero_variance_reproduces_the_pre_offset_state_hash() {
    let (w, ids) = seed_cohort(0xDE0D_0007, race(0, Fixed::from_int(4), Fixed::ZERO), 7, 12);
    assert_eq!(ids.len(), 12);
    assert_eq!(
        w.state_hash(),
        0x81014dee8e948eb5d032135f243231a3u128,
        "a zero developmental variance reproduces the pre-offset dawn bit for bit"
    );
}

// (6) Two siblings recombined identically from one pair of parents at one generation share a
// genome but draw distinct developmental offsets from their distinct ids, so express() differs
// between them even though the genetic input is byte-identical.
#[test]
fn siblings_with_one_genome_draw_distinct_offsets() {
    let spread = Fixed::from_int(1);
    let r = race(0, Fixed::from_int(4), spread);
    let (mut w, parents) = seed_cohort(0x51B_11465, r.clone(), 1, 2);
    let (pa, pb) = (parents[0], parents[1]);
    let band = [pa, pb];
    let birth = |w: &mut World| {
        w.birth(
            &r,
            pa,
            pb,
            &band,
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 20),
            1,
            &dev_ring_law(),
        )
        .expect("a birth")
    };
    let c1 = birth(&mut w);
    let c2 = birth(&mut w);
    assert_ne!(c1, c2, "each birth mints a fresh id");
    // Same parents, same generation: the recombination draws key on the parents and the
    // generation, not the child, so the two children share a byte-identical genome.
    assert_eq!(
        w.genome_of(c1),
        w.genome_of(c2),
        "the two siblings are recombined identically"
    );
    // Yet their expressed minds differ, and only through the developmental offset keyed on the
    // child id (the genome is the same, so this is pure V_E).
    let a1 = w.mind(c1).unwrap().acuity;
    let a2 = w.mind(c2).unwrap().acuity;
    assert_ne!(
        a1, a2,
        "distinct per-being offsets make express() differ at an identical genome"
    );
}
