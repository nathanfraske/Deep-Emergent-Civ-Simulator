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

//! The pre-dawn radiation epoch (design Part 25.12, R-BIOSPHERE).
//!
//! The generator ([`crate::biosphere`]) seeds a region's founders; this module radiates them
//! over deep time as a bounded pre-dawn epoch, before the dawn, so the people arrive into a
//! mature, self-made ecology. Each generation drifts every live pool (the exact Wright-Fisher
//! step of Part 25), applies selection through a piecewise-linear environment-to-coefficient
//! kernel (a species that fits its region better is pushed toward fixing its adaptive
//! alleles, the coefficient clamped to a divide-safe interval), forks founders on a cadence
//! (the founder effect, [`crate::genome::GenePool::found`], each daughter a declared species
//! in the parent-pointer lineage tree with an Orr-snowball incompatibility accumulation on
//! `Phase::SPECIATE`), and drives to extinction any pool whose region suitability leaves it
//! below the carrying-capacity floor (the collapse marked as an append-only payload state,
//! never a deletion).
//!
//! The whole epoch is a pure function of the world seed: every draw keys through the
//! canonical schema with a registered phase and the generation in the tick coordinate, the
//! generation count and the species cap bound the loops, and no float enters canonical state,
//! so a world's biosphere history reproduces bit for bit. Every value the epoch needs is
//! reserved with its basis in [`EpochParams`] and defaulted only by a labelled development
//! fixture.

use civsim_core::{DrawKey, Fixed, Phase};

use crate::biosphere::{Biosphere, Region};
use crate::lineage::SpeciesId;
use crate::stocks::Stock;

/// The epoch's reserved parameters (the selection, speciation, founder-fork, and extinction
/// scales). DEVELOPMENT FIXTURE values come from [`EpochParams::dev_default`]; the
/// authoritative values are the owner's to set on the bases recorded in the audit log, never
/// fabricated here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EpochParams {
    /// The number of pre-dawn generations to run (`G_predawn`), the loop bound.
    pub generations: u64,
    /// The selection-strength scale mapping a suitability shortfall to a coefficient
    /// (`k_sel`).
    pub sel_strength: Fixed,
    /// The selection-coefficient clamp interval, kept divide-safe for the built
    /// `GenePool::select` (`s_min` above `-ONE`, `s_max` bounding the numerator).
    pub s_min: Fixed,
    pub s_max: Fixed,
    /// Fork a founder off each live species every this many generations (the speciation
    /// cadence); zero disables forking.
    pub speciation_cadence: u64,
    /// The founder and recovery effective sizes for a founder-fork.
    pub founder_size: u32,
    pub recovery_size: u32,
    /// The hard cap on total species (living plus extinct), bounding the radiation.
    pub max_species: usize,
    /// The population carrying-capacity scale (a stock capacity is this times suitability).
    pub pop_capacity: Fixed,
    /// The population regeneration rate per generation.
    pub pop_regen: Fixed,
    /// The suitability floor below which a species' carrying capacity is treated as collapse
    /// and the species goes extinct.
    pub extinction_floor: Fixed,
}

impl EpochParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so the epoch runs and can be tested
    /// now.
    pub fn dev_default() -> EpochParams {
        EpochParams {
            generations: 40,
            sel_strength: Fixed::from_ratio(2, 10),
            s_min: Fixed::from_ratio(-4, 10),
            s_max: Fixed::from_ratio(4, 10),
            speciation_cadence: 10,
            founder_size: 6,
            recovery_size: 200,
            max_species: 64,
            pop_capacity: Fixed::ONE,
            pop_regen: Fixed::from_ratio(3, 10),
            extinction_floor: Fixed::from_ratio(12, 100),
        }
    }
}

/// A summary of what the epoch did, for the caller and the proof.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct EpochReport {
    /// Generations run.
    pub generations: u64,
    /// Daughter species forked (speciation events).
    pub daughters: u32,
    /// Species driven extinct.
    pub extinctions: u32,
    /// Species alive at the end.
    pub alive: u32,
    /// Dobzhansky-Muller incompatibilities accumulated (the Orr snowball).
    pub incompatibilities: u32,
}

/// The per-locus selection coefficients for a species in a region: a uniform coefficient
/// driven by how well the species fits its region, mapped from suitability in `[0, ONE]` to
/// `sel_strength * (2*suitability - 1)` (well-fit pushes toward fixing adaptive alleles,
/// poorly-fit against them), clamped to the divide-safe interval. Closed-form fixed-point,
/// no product before the clamp that could wrap the built select divide.
pub fn selection_coefficients(suitability: Fixed, loci: usize, p: &EpochParams) -> Vec<Fixed> {
    // signed_fit in [-ONE, ONE]: 2*suitability - 1.
    let signed_fit = (suitability + suitability) - Fixed::ONE;
    // s = sel_strength * signed_fit; both operands are bounded (|sel_strength| < 1,
    // |signed_fit| <= 1), so the product cannot overflow, then clamp to the safe interval.
    let s = p
        .sel_strength
        .checked_mul(signed_fit)
        .unwrap_or(Fixed::ZERO)
        .clamp(p.s_min, p.s_max);
    vec![s; loci]
}

/// Run the pre-dawn radiation epoch over a region's biosphere, mutating the lineage in place
/// and returning a summary. Deterministic: every draw keys on the world seed with the
/// generation in the tick coordinate.
pub fn run(seed: u64, bio: &mut Biosphere, region: &Region, p: &EpochParams) -> EpochReport {
    let mut report = EpochReport {
        generations: p.generations,
        ..EpochReport::default()
    };
    // Per-species population stock, capacity set from the species' region suitability.
    let mut pops: std::collections::BTreeMap<SpeciesId, Stock> = std::collections::BTreeMap::new();
    for id in bio.species.ids().collect::<Vec<_>>() {
        let suit = bio.species.get(id).unwrap().niche.suitability(&region.env);
        let cap = p.pop_capacity.checked_mul(suit).unwrap_or(Fixed::ZERO);
        pops.insert(id, Stock::new(cap, cap, p.pop_regen));
    }

    for g in 0..p.generations {
        // Selection and drift over every live pool, in canonical id order.
        let ids: Vec<SpeciesId> = bio.species.ids().collect();
        for id in &ids {
            let (suit, extinct, loci) = {
                let sp = bio.species.get(*id).unwrap();
                (
                    sp.niche.suitability(&region.env),
                    sp.extinct,
                    sp.pool.loci(),
                )
            };
            if extinct {
                continue;
            }
            let coeffs = selection_coefficients(suit, loci, p);
            let sp = bio.species.get_mut(*id).unwrap();
            sp.pool.select(&coeffs);
            sp.pool.drift(seed, id.0 as u64, g);

            // Population dynamics: capacity tracks suitability; collapse is extinction.
            if let Some(stock) = pops.get_mut(id) {
                let cap = p.pop_capacity.checked_mul(suit).unwrap_or(Fixed::ZERO);
                stock.set_capacity(cap);
                stock.step(Fixed::ZERO);
                if suit < p.extinction_floor || stock.is_collapsed() {
                    bio.species.get_mut(*id).unwrap().extinct = true;
                    report.extinctions += 1;
                }
            }
        }

        // Speciation on the cadence: fork a founder off each live species, bounded by the cap.
        if p.speciation_cadence != 0 && g % p.speciation_cadence == p.speciation_cadence - 1 {
            for id in &ids {
                if bio.species.len() >= p.max_species {
                    break;
                }
                let parent = bio.species.get(*id).unwrap();
                if parent.extinct {
                    continue;
                }
                let daughter_pool =
                    parent.pool.found(seed, id.0 as u64, g, p.founder_size, p.recovery_size);
                let daughter = crate::biosphere::Species {
                    layer: parent.layer,
                    niche: parent.niche.clone(),
                    draws_on: parent.draws_on.clone(),
                    pool: daughter_pool,
                    extinct: false,
                };
                if let Some(child) = bio.species.speciate(*id, daughter) {
                    report.daughters += 1;
                    let cap = {
                        let s = bio.species.get(child).unwrap().niche.suitability(&region.env);
                        p.pop_capacity.checked_mul(s).unwrap_or(Fixed::ZERO)
                    };
                    pops.insert(child, Stock::new(cap, cap, p.pop_regen));
                    // Orr-snowball: a deterministic incompatibility roll keyed on the ordered
                    // pair and the generation, so the count accumulates per sweep.
                    let rng =
                        DrawKey::pair(id.0 as u64, child.0 as u64, g, Phase::SPECIATE).rng(seed);
                    if rng.flip(0) {
                        report.incompatibilities += 1;
                    }
                }
            }
        }
    }

    report.alive = bio
        .species
        .ids()
        .filter(|&id| !bio.species.get(id).unwrap().extinct)
        .count() as u32;
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosphere::{generate, EnvProfile, GeneratorParams, Region};
    use std::collections::BTreeSet;

    fn region(temp: i64) -> Region {
        let mut abiotic = BTreeSet::new();
        abiotic.insert(0u16);
        abiotic.insert(1u16);
        Region {
            env: EnvProfile::new(vec![
                Fixed::from_ratio(5, 10),
                Fixed::from_ratio(6, 10),
                Fixed::from_ratio(temp, 10),
                Fixed::from_ratio(7, 10),
            ]),
            abiotic,
        }
    }

    #[test]
    fn selection_pushes_by_fit() {
        let p = EpochParams::dev_default();
        // A perfectly fit species gets the maximum positive coefficient (clamped).
        let hi = selection_coefficients(Fixed::ONE, 3, &p);
        assert!(hi.iter().all(|&s| s > Fixed::ZERO), "a good fit selects positively");
        // A total misfit gets a negative coefficient.
        let lo = selection_coefficients(Fixed::ZERO, 3, &p);
        assert!(lo.iter().all(|&s| s < Fixed::ZERO), "a misfit selects against");
        // A neutral fit is zero.
        let mid = selection_coefficients(Fixed::from_ratio(1, 2), 3, &p);
        assert!(mid.iter().all(|&s| s == Fixed::ZERO));
        // Coefficients stay in the clamp interval.
        assert!(hi.iter().all(|&s| s <= p.s_max) && lo.iter().all(|&s| s >= p.s_min));
    }

    #[test]
    fn the_epoch_radiates_and_replays_bit_identically() {
        let gp = GeneratorParams::dev_default();
        let ep = EpochParams::dev_default();
        let run_once = || {
            let mut bio = generate(0xB105, &region(4), 7, &gp);
            let founders = bio.len();
            let report = run(0xB105, &mut bio, &region(4), &ep);
            (founders, bio.len(), report)
        };
        let (founders_a, total_a, report_a) = run_once();
        let (founders_b, total_b, report_b) = run_once();
        assert_eq!((founders_a, total_a, report_a), (founders_b, total_b, report_b), "replays");
        assert!(report_a.daughters > 0, "the epoch radiates daughters");
        assert!(total_a > founders_a, "the lineage grows past the founders");
        assert_eq!(report_a.generations, ep.generations);
    }

    #[test]
    fn a_hostile_region_drives_extinctions() {
        let gp = GeneratorParams::dev_default();
        let ep = EpochParams::dev_default();
        // Seed in a mild region, then radiate in a hostile one (extreme temperature) so many
        // niches fall below the extinction floor.
        let mut bio = generate(0xB105, &region(4), 7, &gp);
        let report = run(0xB105, &mut bio, &region(10), &ep);
        assert!(report.extinctions > 0, "a hostile region kills poorly-fit species");
    }

    #[test]
    fn the_species_cap_bounds_the_radiation() {
        let gp = GeneratorParams::dev_default();
        let mut ep = EpochParams::dev_default();
        ep.max_species = 20;
        ep.generations = 200;
        let mut bio = generate(0xB105, &region(4), 7, &gp);
        run(0xB105, &mut bio, &region(4), &ep);
        assert!(bio.len() <= ep.max_species, "the cap bounds the lineage size");
    }
}
