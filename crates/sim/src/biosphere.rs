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

//! Biosphere generation, the generate-and-validate seeder (design Part 25.11, R-BIOSPHERE).
//!
//! A world's species are generated, not hand-authored: the generative space is authored (the
//! niche and food-web constraints and the reserved ranges) and a species is a sampled point
//! over it, drawn through counter-RNG keyed on the world seed, so the biosphere is part of
//! the world's reproducible identity. The generator emits a closed food web rather than a
//! random bag of organisms: a candidate is validated in a fixed order (fixed-point
//! representability, biome-fit by a Liebig minimum of piecewise-linear tent responses,
//! least-fixed-point food-web closure, and niche openness), resampled at the next ordinal up
//! to a hard bound, then a closure-preserving fallback. Closure is the hybrid (fork F1):
//! topological at seed time here, with quantitative feasibility left to the Part 15 stock
//! dynamics ([`crate::stocks`]) which cull under-supplied pools over the early ticks.
//!
//! Every value the generator needs (the biome-fit cutoff, the resample bound, the niche
//! breadth range, the limiting-similarity floor, the per-region founder counts) is reserved
//! with its basis in [`GeneratorParams`] and defaulted only by a labelled development
//! fixture; the mechanism is fixed Rust and the numbers are the world's (Principle 11). The
//! per-species pool the pre-dawn `epoch` module radiates rides on each accepted
//! species. The source references are the matter-eating interim (fork F5): a draw resolves
//! against an abiotic source or an already-grounded species pool rather than a closed food
//! source enum, so the later source-vector lift (R-SOURCE-VECTOR) is a data change.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{DrawKey, Fixed, Phase};

use crate::anatomy::{sample_body_plan, BodyPlan, BodyPlanRegistry, WorldProfile};
use crate::genome::{GenePool, SchemeId};
use crate::lineage::{Lineage, SpeciesId};

/// A region's environmental profile: the value of each environmental field in `[0, ONE]`,
/// indexed by environmental-axis ordinal (the terrain elevation, moisture, and temperature
/// of Part 12 and the soil-fertility stock occupancy of Part 15, in the dev fixture). The
/// membership is data; the biome-fit law reads a candidate's niche against these fields.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EnvProfile {
    pub fields: Vec<Fixed>,
}

impl EnvProfile {
    /// A profile from its field values (each clamped into `[0, ONE]`).
    pub fn new(fields: Vec<Fixed>) -> EnvProfile {
        EnvProfile {
            fields: fields
                .into_iter()
                .map(|v| v.clamp(Fixed::ZERO, Fixed::ONE))
                .collect(),
        }
    }
}

/// A region the biosphere is seeded into: its environmental profile and the abiotic source
/// ids present there (light, water, soil nutrient, and the like), the ground producers close
/// on. The abiotic set is data; a producer's draw resolves against it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Region {
    pub env: EnvProfile,
    pub abiotic: BTreeSet<u16>,
}

/// A reference to what a species draws on: an abiotic source id, or an already-present
/// species' pool. This two-arm reference is the matter-eating interim (fork F5); the
/// source-vector lift (R-SOURCE-VECTOR) replaces it with a draw over the physics-substrate
/// axis registry so a field-and-gradient feeder is first-class, a data change rather than a
/// rewrite of the closure walk, which already resolves an edge against a source or a pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SourceRef {
    /// An abiotic source present in the region (a producer's ground).
    Abiotic(u16),
    /// Another species in the same region (a consumer's prey or host).
    Species(SpeciesId),
}

/// A species' fundamental niche: per environmental axis, an optimum and a tolerance breadth.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Niche {
    pub optimum: Vec<Fixed>,
    pub breadth: Vec<Fixed>,
}

impl Niche {
    /// The biome-fit suitability against an environment profile: the Liebig minimum of
    /// per-axis piecewise-linear tent responses. A response is `1 - d/w` inside the breadth
    /// (`d = |v - optimum| < w`), zero at or beyond the breadth, and a knife-edge (exact
    /// match full, otherwise zero) at zero breadth. The one divide has `d < w`, so the
    /// quotient is in `[0, 1)` and no product precedes the clamp, and the minimum-fold is
    /// order-independent, matching the floor's `net_nutrition` discipline.
    pub fn suitability(&self, env: &EnvProfile) -> Fixed {
        let n = self.optimum.len().min(self.breadth.len()).min(env.fields.len());
        let mut worst = Fixed::ONE;
        for a in 0..n {
            let d = (env.fields[a] - self.optimum[a]).abs();
            let w = self.breadth[a];
            let response = if w <= Fixed::ZERO {
                if d == Fixed::ZERO {
                    Fixed::ONE
                } else {
                    Fixed::ZERO
                }
            } else if d >= w {
                Fixed::ZERO
            } else {
                Fixed::ONE - d.checked_div(w).unwrap_or(Fixed::ONE)
            };
            if response < worst {
                worst = response;
            }
        }
        worst
    }

    /// The niche-space distance to another niche: the mean absolute difference of the
    /// optima over the shared axes, the trait-space limiting-similarity measure the generator
    /// checks at sample time (before any pool exists, so it does not use the genetic
    /// distance).
    pub fn distance(&self, other: &Niche) -> Fixed {
        let n = self.optimum.len().min(other.optimum.len());
        if n == 0 {
            return Fixed::ZERO;
        }
        let mut acc = Fixed::ZERO;
        for i in 0..n {
            acc += (self.optimum[i] - other.optimum[i]).abs();
        }
        acc.checked_div(Fixed::from_int(n as i32)).unwrap_or(Fixed::ZERO)
    }
}

/// A generated species: its trophic layer (0 producers, higher consumers), its fundamental
/// niche, its structured [`BodyPlan`] (typed parts and temperament, design 25.14), what it
/// draws on, and the allele-frequency pool the pre-dawn epoch radiates. The trophic layer is
/// the grounding depth the closure walk assigns; the emergent trophic label is derived by
/// [`trophic_label`] from what it eats rather than stored (fork F11).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Species {
    pub layer: u16,
    pub niche: Niche,
    pub body_plan: BodyPlan,
    pub draws_on: Vec<SourceRef>,
    pub pool: GenePool,
    /// Whether the lineage has gone extinct. Append-only: an extinct species stays in the
    /// lineage tree as history (the phylogeny is complete), so extinction is a state the
    /// payload carries rather than a deletion (design 25.12).
    pub extinct: bool,
}

/// The emergent trophic label, derived from what a species draws on rather than stored (fork
/// F11): a producer draws only on abiotic sources, a carnivore draws on at least one animal
/// (a non-producer species), a herbivore draws on species that are all producers. The lookup
/// resolves each drawn-on species' role in the same set, so "carnivore" is a reading of the
/// food web, never an authored type.
pub fn trophic_label(species: &std::collections::BTreeMap<SpeciesId, Species>, id: SpeciesId) -> &'static str {
    let sp = match species.get(&id) {
        Some(s) => s,
        None => return "unknown",
    };
    let mut eats_species = false;
    let mut eats_animal = false;
    for src in &sp.draws_on {
        if let SourceRef::Species(dep) = src {
            eats_species = true;
            // A prey that itself draws on a species is an animal; a prey that draws only on
            // abiotic sources is a plant (a producer).
            if let Some(prey) = species.get(dep) {
                if prey.draws_on.iter().any(|s| matches!(s, SourceRef::Species(_))) {
                    eats_animal = true;
                }
            }
        }
    }
    if !eats_species {
        "plant"
    } else if eats_animal {
        "carnivore"
    } else {
        "herbivore"
    }
}

/// The generator's reserved parameters (fork F8 and the seeding parameters). DEVELOPMENT
/// FIXTURE values come from [`GeneratorParams::dev_default`]; the authoritative values are
/// the owner's to set on the bases recorded in the audit log, never fabricated here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GeneratorParams {
    /// The number of environmental axes a niche responds to.
    pub env_axes: usize,
    /// The number of loci each species pool tracks.
    pub loci: usize,
    /// The effective size a freshly seeded pool carries.
    pub pool_size: u32,
    /// The minimum and maximum niche breadth a candidate may draw.
    pub breadth_min: Fixed,
    pub breadth_max: Fixed,
    /// The biome-fit suitability cutoff `tau_fit`: a candidate below it does not fit.
    pub fit_cutoff: Fixed,
    /// The limiting-similarity floor `D_min`: a candidate closer than this to a same-layer
    /// resident is rejected as niche overlap.
    pub open_min: Fixed,
    /// The hard resample bound `K_max` per niche draw site (guarantees termination).
    pub resample_bound: u32,
    /// The number of trophic layers to seed (producers plus consumer layers).
    pub layers: u16,
    /// The number of niche draw sites to attempt per layer per region.
    pub niches_per_layer: u32,
}

impl GeneratorParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so the generator runs and can be
    /// tested now, the way the dev biome set and dev worldgen params let the map run.
    pub fn dev_default() -> GeneratorParams {
        GeneratorParams {
            env_axes: 4,
            loci: 8,
            pool_size: 200,
            breadth_min: Fixed::from_ratio(15, 100),
            breadth_max: Fixed::from_ratio(45, 100),
            fit_cutoff: Fixed::from_ratio(1, 10),
            open_min: Fixed::from_ratio(8, 100),
            resample_bound: 24,
            // Richer ecology (owner-tuned 2026-06-30): four trophic layers (producers,
            // herbivores, carnivores, apex) and nine niche draw sites per layer, so a region
            // grows a fuller food web.
            layers: 4,
            niches_per_layer: 9,
        }
    }
}

/// The grounded set of a candidate web: the least fixed point of "a species is grounded when
/// every one of its draws resolves against a present abiotic source or an already-grounded
/// species." Producers ground on abiotic sources; consumers ground upward. A species outside
/// the returned set is an orphan (a carnivore with no prey, a specialist whose host is
/// absent). Pure integer set arithmetic, no RNG, reached in at most `species.len()` rounds.
pub fn grounded(
    abiotic: &BTreeSet<u16>,
    species: &BTreeMap<SpeciesId, Species>,
) -> BTreeSet<SpeciesId> {
    let mut set: BTreeSet<SpeciesId> = BTreeSet::new();
    loop {
        let mut added = false;
        for (&id, sp) in species {
            if set.contains(&id) {
                continue;
            }
            let resolves = !sp.draws_on.is_empty()
                && sp.draws_on.iter().all(|src| match src {
                    SourceRef::Abiotic(a) => abiotic.contains(a),
                    SourceRef::Species(dep) => set.contains(dep),
                });
            if resolves {
                set.insert(id);
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    set
}

/// A generated biosphere for a region: the accepted species keyed by id in a lineage tree
/// (founders now, daughters after the pre-dawn epoch), and the empty niche draw sites that
/// the generator could not fill (logged rather than hidden, fork F8).
#[derive(Clone, Debug)]
pub struct Biosphere {
    pub species: Lineage<Species>,
    pub empty_niches: u32,
}

impl Biosphere {
    /// The number of species (founders plus any later daughters).
    pub fn len(&self) -> usize {
        self.species.len()
    }

    /// Whether no species were seeded.
    pub fn is_empty(&self) -> bool {
        self.species.is_empty()
    }
}

/// Generate a region's biosphere deterministically from the world seed. Layers are seeded in
/// order: layer 0 producers draw on the region's abiotic sources, and each higher layer
/// draws on an accepted species one layer below, so the web closes by construction and the
/// least-fixed-point check confirms it. Each niche draw site samples a candidate, resamples
/// at the next ordinal up to the bound, and on exhaustion takes the best-scoring candidate
/// that keeps the web closed or logs an empty niche. Every draw is keyed by
/// `Phase::BIOSPHERE_SAMPLE` with the niche locus and a per-attempt slot, so the accepted set
/// is a pure function of the seed.
pub fn generate(
    seed: u64,
    region: &Region,
    region_id: u64,
    p: &GeneratorParams,
    reg: &BodyPlanRegistry,
    profile: WorldProfile,
) -> Biosphere {
    let mut lin: Lineage<Species> = Lineage::new();
    let mut empty_niches = 0u32;
    // Per-layer accepted ids, so a consumer draws on the layer below and openness is checked
    // within a layer.
    let mut by_layer: Vec<Vec<SpeciesId>> = vec![Vec::new(); p.layers as usize];

    for layer in 0..p.layers {
        for niche_ord in 0..p.niches_per_layer {
            let niche_locus = (region_id << 20) ^ ((layer as u64) << 12) ^ niche_ord as u64;
            let mut best: Option<(Fixed, Species)> = None;
            let mut accepted: Option<SpeciesId> = None;

            for attempt in 0..=p.resample_bound {
                let rng = DrawKey::entity(niche_locus, 0, Phase::BIOSPHERE_SAMPLE)
                    .in_region(region_id)
                    .slot(attempt)
                    .rng(seed);
                let cand = match sample_candidate(&rng, layer, region, &by_layer, p, reg, profile) {
                    Some(c) => c,
                    None => continue, // no lower-layer prey exists yet; a later layer stays thin
                };
                let fit = cand.niche.suitability(&region.env);
                let open = by_layer[layer as usize].iter().all(|&other| {
                    lin.get(other)
                        .map(|o| cand.niche.distance(&o.niche) >= p.open_min)
                        .unwrap_or(true)
                });
                // Track the best-scoring closing candidate for the fallback.
                if best.as_ref().map(|(s, _)| fit > *s).unwrap_or(true) {
                    best = Some((fit, cand.clone()));
                }
                if fit >= p.fit_cutoff && open {
                    accepted = Some(commit(&mut lin, &mut by_layer, layer, cand));
                    break;
                }
            }

            if accepted.is_none() {
                // Fallback: take the best-scoring candidate (it closes by construction) or
                // log an empty niche. Closure is never relaxed.
                match best {
                    Some((_, cand)) => {
                        commit(&mut lin, &mut by_layer, layer, cand);
                    }
                    None => empty_niches += 1,
                }
            }
        }
    }

    // Confirm closure over the whole accepted set (the least-fixed-point invariant).
    let all: BTreeMap<SpeciesId, Species> =
        lin.ids().map(|id| (id, lin.get(id).unwrap().clone())).collect();
    let live = grounded(&region.abiotic, &all);
    debug_assert_eq!(live.len(), all.len(), "the generated web is closed by construction");

    Biosphere {
        species: lin,
        empty_niches,
    }
}

/// Commit an accepted candidate into the lineage and its layer index, returning its id.
fn commit(
    lin: &mut Lineage<Species>,
    by_layer: &mut [Vec<SpeciesId>],
    layer: u16,
    cand: Species,
) -> SpeciesId {
    let id = lin.found(cand);
    by_layer[layer as usize].push(id);
    id
}

/// Sample one candidate species for a layer: its niche (optima and breadths over the
/// environmental axes), its draws (an abiotic source for a producer, an accepted lower-layer
/// species for a consumer), and a fresh pool. Returns `None` if a consumer layer has no
/// lower-layer prey to draw on yet.
#[allow(clippy::too_many_arguments)]
fn sample_candidate(
    rng: &civsim_core::Rng,
    layer: u16,
    region: &Region,
    by_layer: &[Vec<SpeciesId>],
    p: &GeneratorParams,
    reg: &BodyPlanRegistry,
    profile: WorldProfile,
) -> Option<Species> {
    let mut optimum = Vec::with_capacity(p.env_axes);
    let mut breadth = Vec::with_capacity(p.env_axes);
    for a in 0..p.env_axes {
        optimum.push(rng.unit_fixed(a as u64));
        let u = rng.unit_fixed((p.env_axes + a) as u64);
        // breadth_min + u*(breadth_max - breadth_min); u and the span are in [0,1], so the
        // product cannot overflow and precedes no divide.
        let span = p.breadth_max - p.breadth_min;
        breadth.push(p.breadth_min + u.checked_mul(span).unwrap_or(Fixed::ZERO));
    }

    let draws_on = if layer == 0 {
        if region.abiotic.is_empty() {
            return None;
        }
        // Pick one present abiotic source deterministically by the draw.
        let sources: Vec<u16> = region.abiotic.iter().copied().collect();
        let pick = rng.range_u32(100, sources.len() as u32) as usize;
        vec![SourceRef::Abiotic(sources[pick])]
    } else {
        let prey_layer = &by_layer[(layer - 1) as usize];
        if prey_layer.is_empty() {
            return None;
        }
        let pick = rng.range_u32(100, prey_layer.len() as u32) as usize;
        vec![SourceRef::Species(prey_layer[pick])]
    };

    // A fresh pool at half frequencies, the reserved loci count and effective size.
    let pool = GenePool::new(
        SchemeId(0),
        p.pool_size,
        vec![Fixed::from_ratio(1, 2); p.loci],
    );
    // The structured aggregate-tier anatomy: a body plan of typed parts and a temperament,
    // drawn on counters offset past the niche counters (design 25.14).
    let body_plan = sample_body_plan(rng, layer, reg, profile, 200);
    Some(Species {
        layer,
        niche: Niche { optimum, breadth },
        body_plan,
        draws_on,
        pool,
        extinct: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> BodyPlanRegistry {
        BodyPlanRegistry::dev_default()
    }

    fn region() -> Region {
        let mut abiotic = BTreeSet::new();
        abiotic.insert(0u16); // light
        abiotic.insert(1u16); // water
        Region {
            env: EnvProfile::new(vec![
                Fixed::from_ratio(5, 10),
                Fixed::from_ratio(6, 10),
                Fixed::from_ratio(4, 10),
                Fixed::from_ratio(7, 10),
            ]),
            abiotic,
        }
    }

    #[test]
    fn suitability_peaks_at_the_optimum_and_falls_to_zero_past_breadth() {
        let n = Niche {
            optimum: vec![Fixed::from_ratio(5, 10)],
            breadth: vec![Fixed::from_ratio(2, 10)],
        };
        let at = EnvProfile::new(vec![Fixed::from_ratio(5, 10)]);
        let near = EnvProfile::new(vec![Fixed::from_ratio(6, 10)]);
        let far = EnvProfile::new(vec![Fixed::from_ratio(9, 10)]);
        assert_eq!(n.suitability(&at), Fixed::ONE, "full at the optimum");
        assert!(n.suitability(&near) > Fixed::ZERO && n.suitability(&near) < Fixed::ONE);
        assert_eq!(n.suitability(&far), Fixed::ZERO, "zero beyond the breadth");
    }

    #[test]
    fn suitability_is_the_worst_axis() {
        // One perfect axis and one out-of-tolerance axis gives zero (the Liebig minimum).
        let n = Niche {
            optimum: vec![Fixed::from_ratio(5, 10), Fixed::from_ratio(5, 10)],
            breadth: vec![Fixed::from_ratio(2, 10), Fixed::from_ratio(1, 10)],
        };
        let env = EnvProfile::new(vec![Fixed::from_ratio(5, 10), Fixed::ONE]);
        assert_eq!(n.suitability(&env), Fixed::ZERO, "the worst axis governs");
    }

    #[test]
    fn a_generated_web_is_closed() {
        let p = GeneratorParams::dev_default();
        let b = generate(0xB105, &region(), 7, &p, &reg(), WorldProfile::grounded());
        assert!(!b.is_empty(), "the region is seeded");
        let all: BTreeMap<SpeciesId, Species> =
            b.species.ids().map(|id| (id, b.species.get(id).unwrap().clone())).collect();
        let live = grounded(&region().abiotic, &all);
        assert_eq!(live.len(), all.len(), "every generated species is grounded");
        // Producers draw on abiotic, consumers on a lower species.
        for sp in all.values() {
            for src in &sp.draws_on {
                match src {
                    SourceRef::Abiotic(a) => assert!(region().abiotic.contains(a)),
                    SourceRef::Species(dep) => assert!(all.contains_key(dep)),
                }
            }
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let p = GeneratorParams::dev_default();
        let hash = |b: &Biosphere| -> Vec<(u32, u16, i64)> {
            b.species
                .ids()
                .map(|id| {
                    let s = b.species.get(id).unwrap();
                    // Include a niche coordinate so the content, not only the shape, is compared.
                    (id.0, s.layer, s.niche.optimum[0].to_bits())
                })
                .collect()
        };
        let a = generate(0xB105, &region(), 7, &p, &reg(), WorldProfile::grounded());
        let b = generate(0xB105, &region(), 7, &p, &reg(), WorldProfile::grounded());
        assert_eq!(hash(&a), hash(&b), "same seed and region, same biosphere");
        let c = generate(0x1234, &region(), 7, &p, &reg(), WorldProfile::grounded());
        assert_ne!(hash(&a), hash(&c), "a different seed, a different biosphere");
    }

    #[test]
    fn generated_species_have_distinct_deterministic_anatomy() {
        let p = GeneratorParams::dev_default();
        let a = generate(0xB105, &region(), 7, &p, &reg(), WorldProfile::grounded());
        let b = generate(0xB105, &region(), 7, &p, &reg(), WorldProfile::grounded());
        // Deterministic: the same seed gives the same anatomy.
        for id in a.species.ids() {
            assert_eq!(
                a.species.get(id).unwrap().body_plan,
                b.species.get(id).unwrap().body_plan
            );
        }
        // Distinct: not every species shares one body mass (anatomy varies across the roster).
        let masses: std::collections::BTreeSet<i64> = a
            .species
            .ids()
            .map(|id| a.species.get(id).unwrap().body_plan.body_mass.to_bits())
            .collect();
        assert!(masses.len() > 1, "species differ in body mass");
    }

    #[test]
    fn an_orphan_is_not_grounded() {
        // A consumer whose only prey is absent is an orphan.
        let mut sp: BTreeMap<SpeciesId, Species> = BTreeMap::new();
        let pool = GenePool::new(SchemeId(0), 10, vec![Fixed::from_ratio(1, 2)]);
        // Build directly: species 0 producer on abiotic 0, species 1 consumer on species 99.
        let bp = sample_body_plan(
            &DrawKey::entity(1, 0, Phase::BIOSPHERE_SAMPLE).rng(0),
            1,
            &reg(),
            WorldProfile::grounded(),
            200,
        );
        sp.insert(
            SpeciesId(0),
            Species { layer: 0, niche: Niche { optimum: vec![], breadth: vec![] }, body_plan: bp.clone(), draws_on: vec![SourceRef::Abiotic(0)], pool: pool.clone(), extinct: false },
        );
        sp.insert(
            SpeciesId(1),
            Species { layer: 1, niche: Niche { optimum: vec![], breadth: vec![] }, body_plan: bp, draws_on: vec![SourceRef::Species(SpeciesId(99))], pool, extinct: false },
        );
        let mut abiotic = BTreeSet::new();
        abiotic.insert(0u16);
        let live = grounded(&abiotic, &sp);
        assert!(live.contains(&SpeciesId(0)), "the producer grounds on abiotic");
        assert!(!live.contains(&SpeciesId(1)), "the consumer with an absent prey is an orphan");

        // The trophic label is derived from diet, not stored: 0 is a plant, 1 eats a species.
        let full: BTreeMap<SpeciesId, Species> = sp.clone();
        assert_eq!(trophic_label(&full, SpeciesId(0)), "plant");
        assert_eq!(trophic_label(&full, SpeciesId(1)), "herbivore", "eats species 99, which is absent so treated as a producer-eater");
    }
}
