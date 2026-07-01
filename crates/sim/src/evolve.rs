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

//! Homeostatic-survival selection of the behaviour controller (design Part 8, Part 25, Part 54;
//! R-BEHAVIOR-EVOLVE Stage 3; Principles 3, 9, 10, 11).
//!
//! This is layer 5 of the evolved-behaviour architecture: selection. A controller's fitness is a
//! consequence of homeostatic survival, not an authored objective and, in particular, not a
//! resemblance to any expected behaviour. A controller is scored by whether it keeps a body's
//! reserves off their floors, which is measured by running the body under that controller through the
//! movement-and-metabolism physics (`crate::locomotion`) and counting how long it stays viable. A
//! lineage whose controller keeps its bodies alive fixes its adaptive controller alleles; one whose
//! controller lets its bodies starve is selected against. Nobody scores "did it seek water": the
//! being that seeks water survives, and survival is what is counted. This is what keeps the behaviour
//! emergent (design pass, `docs/emergent_behavior_design.md`).
//!
//! Two scorers sit on the significance-and-time gradient the engine already uses for detail
//! (Principle 1, Part 54). The [`episode_survival`] scorer runs a full behavioural episode through
//! the movement physics, the honest high-fidelity measure. The cheaper proxy for quiet deep-time
//! pools, and the full-episode tier at the dawn, are the same call at different lengths and
//! environments; which pools get which is the reserved significance allocation. Every scoring draw
//! keys on the seed, the individual, and the generation ([`civsim_core::Phase::CONTROLLER`]), never
//! on the camera, so which behaviours a world evolves is a function of the seed and the world alone
//! (Principles 3, 10).
//!
//! The selection itself is the aggregate-tier recurrence [`crate::genome::GenePool::select`] already
//! carries: it takes a per-locus selection coefficient and moves the frequencies, channel-blind, so
//! a controller locus is selected by the same mechanism as any other once its coefficient is a
//! consequence of homeostatic survival ([`homeostatic_coefficient`]). The individual-based loop
//! [`evolve`] proves the whole chain end to end: from random controllers, homeostatic-survival
//! selection with bounded mutation produces water-seeking behaviour, from a random start, without it
//! being authored.
//!
//! Honest limits, which the design pass names as the crux to prove rather than proven here. The
//! deep-time pool tier expresses a controller from allele frequencies, which needs the quantitative
//! breeding-value tier the genome still defers (25.10), so the pure-frequency deep-time controller
//! evolution couples to that tier and to the open temporal level of detail (Part 32); this module
//! builds and proves the individual and sampled-episode tier and wires the coefficient into the pool
//! recurrence, and scopes the pure-frequency tier as the reserved coupling. The proxy's honesty
//! (that surviving the scored episode predicts surviving in the world) is validated by cross-checking
//! against longer and richer episodes, not asserted.

use civsim_core::{DrawKey, Fixed, Phase, StableId};
use civsim_world::Coord3;

use crate::anatomy::{BodyPlan, Part, Temperament};
use crate::controller::{Controller, ControllerLayout};
use crate::genome::{
    Allele, AlleleState, Channel, ControllerParamId, DominanceMode, GeneDef, GeneEffect, GeneId,
    GenePool, GeneSet, Genome, Haplotype, SchemeId,
};
use crate::homeostasis::{
    AffordanceRegistry, Homeostasis, HomeostaticAxisDef, HomeostaticRegistry, WATER,
};
use crate::locomotion::{self, LocomotionParams, ResourceField, Terrain, Walker};

// Draw-site slots within the CONTROLLER phase, so the init and the two mutation rolls of one lineage
// do not collide on counter zero (the R-RNG-COORD slot rule).
const SLOT_INIT: u32 = 0;
const SLOT_MUT_HIT: u32 = 1;
const SLOT_MUT_STEP: u32 = 2;

/// The reserved parameters of controller evolution. The mechanism is fixed; these numbers are the
/// owner's to set, surfaced with a basis, never fabricated (Principle 11). The development fixture
/// below lets the loop run and be tested now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EvolveParams {
    /// The number of controller lineages under selection each generation. RESERVED. Basis: the
    /// sample size that resolves the fitness ranking against the per-generation evaluation budget, a
    /// performance bound on the significance-and-time gradient (Part 54).
    pub pop_size: usize,
    /// The number of generations to run. RESERVED. Basis: the deep-time depth over which behaviour
    /// is allowed to adapt before the dawn, tied to the pre-dawn radiation depth (`EpochParams`).
    pub generations: u32,
    /// The length in ticks of a scoring episode. RESERVED. Basis: long enough that a viable
    /// controller's survival separates from an unviable one under the metabolic timescale, against
    /// the evaluation budget (a proxy-versus-episode length on the Part 54 gradient).
    pub episode_ticks: u32,
    /// The half-range of the initial random controller weights. RESERVED. Basis: the weight scale at
    /// which the activation's clamp is neither always saturated nor always near zero, so variation is
    /// expressible from the start (a representational bound).
    pub init_spread: Fixed,
    /// The per-controller-weight probability of a mutation on inheritance. RESERVED. Basis: the
    /// mutation scale the epoch uses for the other channels (`EpochParams`), adjusted for the larger
    /// controller parameter space (design pass).
    pub mutation_rate: Fixed,
    /// The bounded magnitude of a controller-weight mutation. RESERVED. Basis: a perturbation small
    /// enough that a small weight change is a small behaviour change (smooth evolution) yet large
    /// enough to explore the weight space, a stress-test tunable.
    pub mutation_step: Fixed,
}

impl EvolveParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so the loop runs and can be tested now.
    pub fn dev_default() -> EvolveParams {
        EvolveParams {
            pop_size: 32,
            generations: 16,
            episode_ticks: 200,
            init_spread: Fixed::from_int(2),
            mutation_rate: Fixed::from_ratio(1, 6),
            mutation_step: Fixed::from_ratio(2, 5),
        }
    }
}

/// The gene set for a controller of a given layout: one gene per controller weight, each feeding its
/// own [`Channel::Controller`] parameter with unit weight, so the expressed weight at parameter `k`
/// is exactly the additive value of locus `k` (a haploid additive spine). This is the data half of
/// the controller (which genes reach which parameters); the mechanism that reads it is
/// [`Controller::express`].
pub fn controller_gene_set(layout: &ControllerLayout) -> GeneSet {
    let genes = (0..layout.weight_count())
        .map(|k| GeneDef {
            id: GeneId(k as u32),
            effects: vec![GeneEffect {
                channel: Channel::Controller(ControllerParamId(k as u32)),
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode::additive(),
        })
        .collect();
    GeneSet { genes }
}

/// A founder genome carrying random controller weights, each drawn uniformly in
/// `[-init_spread, init_spread]` from counter-based RNG keyed on the individual and the parameter
/// locus, so a founder lineage is reproducible from the seed (design Part 3.2).
pub fn random_controller_genome(
    layout: &ControllerLayout,
    params: &EvolveParams,
    seed: u64,
    id: u64,
) -> Genome {
    let rng = DrawKey::entity(id, 0, Phase::CONTROLLER).slot(SLOT_INIT).rng(seed);
    let spread = params.init_spread;
    let alleles = (0..layout.weight_count())
        .map(|k| {
            // unit in [0, ONE) -> [-spread, spread).
            let u = rng.unit_fixed(k as u64);
            let additive = u.mul(spread).mul(Fixed::from_int(2)) - spread;
            Allele {
                additive,
                state: AlleleState(0),
                origin: id as u32,
            }
        })
        .collect();
    Genome {
        scheme: SchemeId(0),
        haps: vec![Haplotype { alleles }],
    }
}

/// Mutate a genome's controller weights: each weight, with probability `mutation_rate`, gains a
/// bounded step drawn uniformly in `[-mutation_step, mutation_step]`, keyed on the child, the locus,
/// and the generation ([`Phase::CONTROLLER`]), so a lineage's mutations are a reproducible function
/// of the seed and its ancestry. This is the controller-allele mutation the design reserves; the
/// general continuous additive-mutation shape for every channel remains the deferred integer-Gaussian
/// of 25.10.
pub fn mutate(
    parent: &Genome,
    params: &EvolveParams,
    seed: u64,
    child_id: u64,
    generation: u64,
) -> Genome {
    let mut haps = parent.haps.clone();
    if let Some(hap) = haps.first_mut() {
        for (locus, allele) in hap.alleles.iter_mut().enumerate() {
            let hit = DrawKey::pair(child_id, locus as u64, generation, Phase::CONTROLLER)
                .slot(SLOT_MUT_HIT)
                .rng(seed)
                .unit_fixed(0);
            if hit < params.mutation_rate {
                let u = DrawKey::pair(child_id, locus as u64, generation, Phase::CONTROLLER)
                    .slot(SLOT_MUT_STEP)
                    .rng(seed)
                    .unit_fixed(0);
                // u in [0, ONE) -> [-step, step).
                let delta = u.mul(params.mutation_step).mul(Fixed::from_int(2)) - params.mutation_step;
                allele.additive += delta;
            }
        }
    }
    Genome {
        scheme: parent.scheme,
        haps,
    }
}

/// A fast-draining water-only physiology for scoring, so a controller that fails to reach and drink
/// water dies promptly and a competent one survives to the cap, giving selection a sharp gradient (a
/// labelled scoring fixture, not owner canon).
fn scoring_reg() -> HomeostaticRegistry {
    HomeostaticRegistry {
        axes: vec![HomeostaticAxisDef {
            id: WATER,
            name: "water".to_string(),
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::from_ratio(1, 60),
            exertion_drain: Fixed::from_ratio(1, 200),
            death_floor: Fixed::ZERO,
        }],
    }
}

/// An open, flat plane: the scoring environment where survival turns on the controller's foraging,
/// not on terrain.
struct OpenPlane;
impl Terrain for OpenPlane {
    fn passable(&self, _c: Coord3, _b: &BodyPlan) -> bool {
        true
    }
    fn cost(&self, _c: Coord3) -> Fixed {
        Fixed::ONE
    }
}

/// A plain mobile body for scoring: it can walk, its speed and metabolism are the physics, so only
/// its controller varies between the beings under selection.
fn scoring_body() -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(1, 2),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: vec![],
        covering: Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        },
        senses: vec![],
        locomotion: vec![1],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(3, 4),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

/// Score a controller by homeostatic survival: place a body carrying it at the origin, knowing of a
/// water region a short way east, and run it through the movement-and-metabolism physics for up to
/// `ticks`. The score is how many ticks it stays viable (capped at `ticks`), so a controller that
/// moves to the water and drinks when dry survives to the cap and one that idles or wanders starves.
/// The water is a region rather than a single tile, so partial competence (drifting east and drinking
/// in the region) yields partial survival, a climbable gradient, while survival stays the honest
/// fitness (no resemblance to an authored behaviour is scored). Fully deterministic and seed-keyed.
pub fn episode_survival(controller: &Controller, ticks: u32, seed: u64) -> u32 {
    let reg = scoring_reg();
    let afford = AffordanceRegistry::dev_default();
    let layout = ControllerLayout::new(&reg, &afford, controller.hidden());
    let homeo = Homeostasis::new(&reg, Fixed::ONE);
    let mut walker = Walker::new(StableId(1), Coord3::ground(0, 0), scoring_body(), homeo, controller.clone());
    let mut field = ResourceField::new();
    // A water region to the east; the being is shown it (the scorer tests foraging, not search).
    for y in -2..=2 {
        for x in 3..=7 {
            let c = Coord3::ground(x, y);
            field.add(WATER, c);
            walker.learn(WATER, c);
        }
    }
    let p = LocomotionParams::dev_default();
    let mut ws = vec![walker];
    let mut survived = 0u32;
    for t in 0..ticks {
        locomotion::step(&mut ws, &reg, &layout, &afford, &OpenPlane, &field, &p, seed, t as u64);
        if !ws[0].alive {
            break;
        }
        survived = t + 1;
    }
    survived
}

/// A gentler water-only physiology for the full-episode (dawn) tier, so a being has time to search
/// for water it does not yet know of before it starves (a labelled scoring fixture).
fn dawn_reg() -> HomeostaticRegistry {
    HomeostaticRegistry {
        axes: vec![HomeostaticAxisDef {
            id: WATER,
            name: "water".to_string(),
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::from_ratio(1, 150),
            exertion_drain: Fixed::from_ratio(1, 300),
            death_floor: Fixed::ZERO,
        }],
    }
}

/// Score a controller by a FULL behavioural episode, the high-fidelity tier the design pass runs at
/// the dawn and under significance (Part 54): unlike [`episode_survival`], the being does NOT know
/// where the water is, so it must explore to discover it (`crate::locomotion` exploration), then
/// forage. This exercises the whole loop (search, approach, drink) rather than only foraging from
/// known sources, so it validates that proxy-viability predicts world-viability. Deterministic and
/// seed-keyed; returns ticks survived.
pub fn full_episode_survival(controller: &Controller, ticks: u32, seed: u64) -> u32 {
    let reg = dawn_reg();
    let afford = AffordanceRegistry::dev_default();
    let layout = ControllerLayout::new(&reg, &afford, controller.hidden());
    let homeo = Homeostasis::new(&reg, Fixed::ONE);
    // Water sits east, near but outside the being's initial perception, so it must move to find it.
    let mut field = ResourceField::new();
    for y in -1..=1 {
        for x in 6..=9 {
            field.add(WATER, Coord3::ground(x, y));
        }
    }
    let walker = Walker::new(StableId(1), Coord3::ground(0, 0), scoring_body(), homeo, controller.clone());
    let p = LocomotionParams::dev_default();
    let mut ws = vec![walker];
    let mut survived = 0u32;
    for t in 0..ticks {
        locomotion::step(&mut ws, &reg, &layout, &afford, &OpenPlane, &field, &p, seed, t as u64);
        if !ws[0].alive {
            break;
        }
        survived = t + 1;
    }
    survived
}

/// The report of an evolutionary run: the mean and best homeostatic-survival fitness at each
/// generation (so a caller can see behaviour shift), and the final population of genomes.
#[derive(Clone, Debug)]
pub struct EvolveReport {
    /// The mean survival fitness at each generation.
    pub mean_fitness: Vec<Fixed>,
    /// The best survival fitness at each generation.
    pub best_fitness: Vec<u32>,
    /// The final population's genomes.
    pub final_genomes: Vec<Genome>,
}

/// Evolve a population of controllers under homeostatic-survival selection (design Part 8, Part 25;
/// R-BEHAVIOR-EVOLVE Stage 3). From random founders, each generation scores every controller by
/// [`episode_survival`], keeps the fitter half (truncation, ties broken by the lower id so the choice
/// is deterministic), and refills the population with bounded mutants of the survivors ([`mutate`]).
/// The whole run is a pure function of the seed. Returns the per-generation fitness so a caller can
/// see behaviour improve; the physics scores survival, and adaptive behaviour is what survives.
pub fn evolve(layout: &ControllerLayout, params: &EvolveParams, seed: u64) -> EvolveReport {
    // A degenerate empty population has nothing to select; return an empty report rather than
    // indexing an empty slice.
    if params.pop_size == 0 {
        return EvolveReport {
            mean_fitness: Vec::new(),
            best_fitness: Vec::new(),
            final_genomes: Vec::new(),
        };
    }
    let genes = controller_gene_set(layout);
    // Founders: random controllers, one per lineage, ids 0..pop_size.
    let mut pop: Vec<Genome> = (0..params.pop_size as u64)
        .map(|id| random_controller_genome(layout, params, seed, id))
        .collect();
    let mut next_id = params.pop_size as u64;
    let mut mean_fitness = Vec::with_capacity(params.generations as usize);
    let mut best_fitness = Vec::with_capacity(params.generations as usize);

    for g in 0..params.generations as u64 {
        // Score every genome by homeostatic survival. The scoring seed folds the generation so a
        // fixed lineage is re-scored in the same environment, keyed reproducibly.
        let mut scored: Vec<(u32, usize)> = pop
            .iter()
            .enumerate()
            .map(|(i, genome)| {
                let controller = Controller::express(&genes, genome, layout);
                let fit = episode_survival(&controller, params.episode_ticks, seed ^ 0xE0);
                (fit, i)
            })
            .collect();
        let sum: u64 = scored.iter().map(|(f, _)| *f as u64).sum();
        mean_fitness.push(Fixed::from_ratio(sum as i64, pop.len().max(1) as i64));
        best_fitness.push(scored.iter().map(|(f, _)| *f).max().unwrap_or(0));

        // Truncation selection: keep the fitter half. Sort by fitness descending, ties to the lower
        // index (deterministic).
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let keep = (pop.len() / 2).max(1);
        let survivors: Vec<Genome> = scored[..keep].iter().map(|&(_, i)| pop[i].clone()).collect();

        // Next generation: the survivors (elitism), then a bounded mutant of each until the
        // population is refilled.
        let mut next: Vec<Genome> = Vec::with_capacity(pop.len());
        next.extend(survivors.iter().cloned());
        let mut s = 0usize;
        while next.len() < pop.len() {
            let parent = &survivors[s % survivors.len()];
            let child = mutate(parent, params, seed, next_id, g);
            next.push(child);
            next_id += 1;
            s += 1;
        }
        pop = next;
    }

    EvolveReport {
        mean_fitness,
        best_fitness,
        final_genomes: pop,
    }
}

/// The homeostatic-survival selection coefficient of one controller allele over another, for the
/// aggregate-tier recurrence [`GenePool::select`] (design 25.7): the survival advantage of the
/// adaptive variant scaled by a selection strength, so a pool carrying the adaptive controller allele
/// at a locus fixes it under the same recurrence any other locus is selected by. This is how
/// homeostatic survival becomes a selection pressure on behaviour at the deep-time tier. `sel_strength`
/// is the reserved selection scale (the epoch's, shared for consistency). The pool-tier EXPRESSION of
/// a controller from allele frequencies is the reserved coupling to the deferred quantitative tier.
pub fn homeostatic_coefficient(
    adaptive: &Controller,
    baseline: &Controller,
    ticks: u32,
    sel_strength: Fixed,
    seed: u64,
) -> Fixed {
    let fit_a = episode_survival(adaptive, ticks, seed) as i64;
    let fit_b = episode_survival(baseline, ticks, seed) as i64;
    let advantage = Fixed::from_ratio(fit_a - fit_b, ticks.max(1) as i64);
    sel_strength.mul(advantage)
}

/// The per-locus selection gradient across a scored population: for each controller locus, the
/// (unnormalised) covariance of that locus's weight with fitness, so a positive value marks a locus
/// whose higher weight tracks higher survival (the breeder's-equation direction). A caller can feed a
/// scaled gradient to [`GenePool::select`] to move a pool toward the adaptive controller. Deterministic
/// and float-free (an i128 accumulation), a pure function of the scored population.
pub fn selection_gradient(
    scored: &[(Genome, u32)],
    layout: &ControllerLayout,
    genes: &GeneSet,
) -> Vec<Fixed> {
    let n = scored.len();
    let count = layout.weight_count();
    if n == 0 {
        return vec![Fixed::ZERO; count];
    }
    // Mean fitness and per-locus mean weight.
    let fit_sum: i64 = scored.iter().map(|(_, f)| *f as i64).sum();
    let mean_fit = Fixed::from_ratio(fit_sum, n as i64);
    let controllers: Vec<Controller> = scored
        .iter()
        .map(|(g, _)| Controller::express(genes, g, layout))
        .collect();
    let mut grad = vec![Fixed::ZERO; count];
    for (k, gk) in grad.iter_mut().enumerate() {
        let weights: Vec<Fixed> = controllers.iter().map(|c| c.weight(k)).collect();
        let wsum = Fixed::saturating_sum(weights.iter().copied());
        let mean_w = wsum.div(Fixed::from_int(n as i32));
        // Covariance = mean over individuals of (w - mean_w)*(fit - mean_fit).
        let terms = controllers.iter().enumerate().map(|(i, c)| {
            let dw = c.weight(k) - mean_w;
            let df = Fixed::from_int(scored[i].1 as i32) - mean_fit;
            dw.mul(df)
        });
        let cov = Fixed::saturating_sum(terms).div(Fixed::from_int(n as i32));
        *gk = cov;
    }
    grad
}

/// Build a gene pool over the controller loci for the aggregate-tier demonstration: a biallelic pool
/// whose state-1 frequency at every locus starts at `p0`, so [`GenePool::select`] with a positive
/// coefficient raises it (the frequency of the adaptive controller allele). The pool tracks the
/// discrete Mendelian view; the pool-to-controller expression is the reserved coupling.
pub fn controller_pool(layout: &ControllerLayout, effective_size: u32, p0: Fixed) -> GenePool {
    GenePool::new(SchemeId(0), effective_size, vec![p0; layout.weight_count()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::AffordanceRegistry;
    use std::collections::{BTreeMap, BTreeSet};

    fn scoring_layout(hidden: usize) -> ControllerLayout {
        ControllerLayout::new(&scoring_reg(), &AffordanceRegistry::dev_default(), hidden)
    }

    /// A hand-built competent forager over the water-only scoring layout: move toward known water,
    /// drink underfoot when dry. Its survival should be near the cap.
    fn competent(l: &ControllerLayout) -> Controller {
        let n_in = l.n_in();
        let bias = n_in - 1;
        let mut w = vec![Fixed::ZERO; l.weight_count()];
        w[bias] = Fixed::ONE; // move_act wants to move,
        w[1] = Fixed::from_int(-1); // but not off the water underfoot (here flag, index 1)
        w[n_in + 2] = Fixed::ONE; // move_dx follows water dir_x (index 2)
        w[2 * n_in + 3] = Fixed::ONE; // move_dy follows water dir_y (index 3)
        w[3 * n_in + 1] = Fixed::ONE; // ingest_act fires when water underfoot
        w[3 * n_in] = Fixed::from_int(-1); // and the reserve (index 0) is low
        Controller::from_weights(n_in, l.n_out(), l.hidden(), w)
    }

    #[test]
    fn a_competent_forager_outlives_a_blank_one() {
        let l = scoring_layout(0);
        let good = competent(&l);
        let blank = Controller::zeros(&l);
        let fit_good = episode_survival(&good, 200, 0xF00D);
        let fit_blank = episode_survival(&blank, 200, 0xF00D);
        assert!(fit_good > fit_blank, "the forager outlives the blank controller ({fit_good} vs {fit_blank})");
        assert!(fit_good >= 190, "the competent forager survives almost to the cap ({fit_good})");
    }

    #[test]
    fn episode_survival_is_deterministic() {
        let l = scoring_layout(0);
        let good = competent(&l);
        assert_eq!(
            episode_survival(&good, 120, 0xABCD),
            episode_survival(&good, 120, 0xABCD),
            "the same controller and seed replay the same survival"
        );
    }

    #[test]
    fn behaviour_evolves_under_homeostatic_selection() {
        // The proof: from random controllers, homeostatic-survival selection produces beings that
        // survive, from a random start, without water-seeking being authored anywhere.
        let l = scoring_layout(0);
        let params = EvolveParams::dev_default();
        let report = evolve(&l, &params, 0x5EED_1234);
        let first = report.mean_fitness.first().copied().unwrap();
        let last = report.mean_fitness.last().copied().unwrap();
        assert!(
            last > first,
            "mean survival rose under selection ({} -> {})",
            first.to_f64_lossy(),
            last.to_f64_lossy()
        );
        let best_last = *report.best_fitness.last().unwrap();
        assert!(best_last >= 190, "an evolved lineage survives almost to the cap ({best_last})");
    }

    #[test]
    fn the_evolved_best_seeks_water_when_dry_though_no_one_wrote_it() {
        // Take the fittest evolved genome and check the emergent behaviour: dry, knowing of water to
        // the east, it moves toward the water (or drinks it if underfoot). This behaviour was
        // selected, not authored.
        let l = scoring_layout(0);
        let params = EvolveParams::dev_default();
        let report = evolve(&l, &params, 0x5EED_1234);
        let genes = controller_gene_set(&l);
        // Re-score the final population to find the best.
        let scored: Vec<(Genome, u32)> = report
            .final_genomes
            .iter()
            .map(|g| {
                let c = Controller::express(&genes, g, &l);
                (g.clone(), episode_survival(&c, params.episode_ticks, 0x5EED_1234 ^ 0xE0))
            })
            .collect();
        let best = scored.iter().max_by_key(|(_, f)| *f).unwrap();
        let controller = Controller::express(&genes, &best.0, &l);
        // A dry being that knows of water to the east.
        let reg = scoring_reg();
        let mut homeo = Homeostasis::new(&reg, Fixed::ONE);
        for _ in 0..40 {
            homeo.metabolize(&reg, Fixed::ZERO);
        }
        let mut dirs = BTreeMap::new();
        dirs.insert(WATER, (Fixed::ONE, Fixed::ZERO));
        let input = l.build_input(&homeo, &BTreeSet::new(), &dirs);
        let (out, _) = controller.evaluate(&input, &[]);
        let d = l.decide(&out, &crate::homeostasis::AffordanceRegistry::dev_default().afforded(&scoring_body())).unwrap();
        // Emergent water-seeking: it moves toward the known water (a positive eastward heading).
        if let Some((hx, _)) = d.heading {
            assert!(
                d.affordance == crate::homeostasis::MOVE && hx > Fixed::ZERO,
                "the evolved being heads toward the known water it is dry for"
            );
        } else {
            panic!("the evolved being's top decision on a dry, water-east percept was not to move");
        }
    }

    #[test]
    fn homeostatic_survival_selects_the_adaptive_allele_in_a_pool() {
        // Wiring the coefficient into the aggregate-tier recurrence: the survival advantage of the
        // competent forever over the blank controller is a positive coefficient, and GenePool::select
        // raises the frequency of the adaptive controller allele (the epoch's existing mechanism).
        let l = scoring_layout(0);
        let good = competent(&l);
        let blank = Controller::zeros(&l);
        let sel_strength = Fixed::from_ratio(1, 5);
        let coeff = homeostatic_coefficient(&good, &blank, 200, sel_strength, 0xC0FFEE);
        assert!(coeff > Fixed::ZERO, "surviving longer is a positive selection coefficient");
        let mut pool = controller_pool(&l, 200, Fixed::from_ratio(1, 2));
        let before = pool.freq(0).unwrap();
        for _ in 0..20 {
            pool.select(&vec![coeff; pool.loci()]);
        }
        let after = pool.freq(0).unwrap();
        assert!(after > before, "the adaptive controller allele rises in the pool ({} -> {})", before.to_f64_lossy(), after.to_f64_lossy());
    }

    #[test]
    fn the_selection_gradient_is_deterministic_and_sized() {
        let l = scoring_layout(0);
        let genes = controller_gene_set(&l);
        let params = EvolveParams::dev_default();
        let report = evolve(&l, &params, 0x1111);
        let scored: Vec<(Genome, u32)> = report
            .final_genomes
            .iter()
            .map(|g| {
                let c = Controller::express(&genes, g, &l);
                (g.clone(), episode_survival(&c, params.episode_ticks, 0x1111 ^ 0xE0))
            })
            .collect();
        let grad_a = selection_gradient(&scored, &l, &genes);
        let grad_b = selection_gradient(&scored, &l, &genes);
        assert_eq!(grad_a, grad_b, "the gradient is a deterministic function of the population");
        assert_eq!(grad_a.len(), l.weight_count(), "one gradient entry per controller weight");
    }

    #[test]
    fn an_empty_population_evolves_to_an_empty_report_without_panicking() {
        // A degenerate config (no lineages) must not panic on an empty-slice index.
        let l = scoring_layout(0);
        let params = EvolveParams {
            pop_size: 0,
            generations: 3,
            ..EvolveParams::dev_default()
        };
        let report = evolve(&l, &params, 0x0);
        assert!(report.final_genomes.is_empty());
        assert!(report.mean_fitness.is_empty());
    }

    // --- Stage 4: the full-episode (dawn) tier and the recurrent-network graduation ---

    #[test]
    fn the_full_episode_tier_rewards_finding_water_by_search() {
        // The high-fidelity tier: the being is not shown the water, so it must explore to discover
        // it. The competent forager (which explores when it knows of no water) finds and drinks it
        // and outlives the blank one, which idles and dies of thirst. This validates that foraging
        // from known sources (the proxy) carries over to the full loop with search.
        let l = scoring_layout(0);
        let good = competent(&l);
        let blank = Controller::zeros(&l);
        let fit_good = full_episode_survival(&good, 400, 0xDA7);
        let fit_blank = full_episode_survival(&blank, 400, 0xDA7);
        assert!(
            fit_good > fit_blank + 100,
            "the forager finds water by search and far outlives the idle one ({fit_good} vs {fit_blank})"
        );
    }

    #[test]
    fn a_recurrent_controller_graduates_the_plumbing_and_evolves() {
        // The graduation: the same expression, selection, and mutation plumbing runs a small
        // recurrent network (a hidden state) rather than a reaction norm, its topology fixed Rust
        // and its weights the heritable data. Behaviour still evolves under homeostatic selection,
        // so moving to the network is a parameter change (the hidden width), not a rewrite.
        let l = scoring_layout(1); // hidden width 1: a recurrent controller
        assert_eq!(l.hidden(), 1);
        assert!(l.weight_count() > 0);
        let params = EvolveParams {
            generations: 24,
            ..EvolveParams::dev_default()
        };
        let report = evolve(&l, &params, 0x9E77);
        let first = report.mean_fitness.first().copied().unwrap();
        let last = report.mean_fitness.last().copied().unwrap();
        assert!(
            last > first,
            "the recurrent controller's behaviour evolves under selection too ({} -> {})",
            first.to_f64_lossy(),
            last.to_f64_lossy()
        );
    }
}
