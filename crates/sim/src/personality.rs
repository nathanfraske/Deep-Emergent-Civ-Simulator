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

//! The age-personality substrate (design Part 20, R-BEING-REP; Principles 8, 9, 11), the derive
//! column's `being.plasticity_by_age` and `being.maturity_targets`.
//!
//! A being's personality is not fixed at birth: it drifts over a lifetime, fast in youth and
//! slowing to a plateau by maturity, and each trait moves toward its own mature setting. The design
//! reserves two data for this (`being.plasticity_by_age`, the timing of that slowdown, and
//! `being.maturity_targets`, where each trait ends up), and the audit ruled both stay per-race
//! [`TraitDef`] data rather than a fixed Big Five: the timing derives from each race's own
//! `maturity_years` through [`crate::race::Race::maturation_fraction`] (which reads only
//! `maturity_years`; `lifespan_years` shapes the separate life-hazard fraction, not this plasticity
//! timing), and the trait axes, their plasticity curves, and their targets are the race's data. The
//! human maturity
//! principle (rising rank-order stability, rising conscientiousness, falling novelty-seeking; Roberts
//! and DelVecchio 2000; the Big-Five aging literature) is one race's data row, the only worked
//! example, never the mechanism.
//!
//! The mechanism is fixed Rust and reads no race id (Principle 9). Two races diverge purely through
//! their [`PersonalityProfile`] data: one whose trait rises and another whose trait falls come from
//! the one [`age_personality`] kernel, because a rising trait carries a target above its birth value
//! and a falling one a target below. Nothing in the kernel branches on which race it is; it reads
//! the per-race curve and target and the being's own age-scaled maturation fraction.
//!
//! Determinism is total: the drift is a pure fixed-point function of the being's current trait
//! value, its age, and the per-race data, with no RNG and no float, walked in canonical order, so it
//! reproduces bit for bit (Principle 3). Like the aging beat it rides on, the trait state is a
//! reconstructible deterministic function of the age cadence rather than an independent random walk,
//! so it needs no separate replay entropy.

use std::collections::BTreeMap;

use civsim_core::Fixed;

use crate::race::Race;
use crate::value::RaceId;
use civsim_bio::decision::Curve;

/// A data-defined personality trait axis identifier (an open registry, Principle 11): which trait a
/// race's personality moves along, for example a conscientiousness-like axis, a novelty-seeking one,
/// or an axis no human carries. A newtype rather than a closed enum, so a race authors its own axes
/// and the engine never hardcodes a fixed set of personality dimensions (design Part 20, the
/// no-templated-Big-Five ruling).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TraitAxisId(pub u32);

/// One race's definition of how a single personality trait axis matures: the axis, its plasticity
/// curve, and its maturity target. All three are per-race data (Principle 11).
///
/// The `plasticity_curve` is read at the being's maturation fraction (design Part 20,
/// [`Race::maturation_fraction`]), which runs from zero at birth to one at `maturity_years`, so a
/// curve high near zero and low near one gives the "fast in youth, low plateau by maturity" shape
/// (rising rank-order stability) as data, not as a hardcoded schedule. The `maturity_target` is the
/// value the trait drifts toward; a target above the birth value gives a trait that rises with age,
/// one below gives a trait that falls, so the direction of maturation is per-race data (the human
/// steer of rising conscientiousness and falling novelty-seeking is just one race's two rows).
#[derive(Clone, Debug)]
pub struct TraitDef {
    /// The trait axis this definition governs.
    pub axis: TraitAxisId,
    /// The plasticity curve (`being.plasticity_by_age`): the per-step fraction of the gap to the
    /// target that is closed, read at the race's own maturation fraction, so the slowdown timing
    /// derives from the race's `maturity_years`. High in youth, low plateau by maturity.
    pub plasticity_curve: Curve,
    /// The maturity target (`being.maturity_targets`): the value the trait drifts toward with age.
    /// Above the birth value the trait rises, below it the trait falls; the direction is data.
    pub maturity_target: Fixed,
}

/// A race's personality profile: its per-trait definitions, kept sorted and deduplicated by axis id
/// so the kernel walks them in one canonical order on every machine (Principle 3). A race with no
/// definitions authors no personality drift.
#[derive(Clone, Debug, Default)]
pub struct PersonalityProfile {
    defs: Vec<TraitDef>,
}

impl PersonalityProfile {
    /// A profile from a set of trait definitions, sorted by axis id and deduplicated (a later
    /// definition of the same axis replaces an earlier one), so the walk order is a pure function of
    /// the axis ids.
    pub fn new(defs: impl IntoIterator<Item = TraitDef>) -> Self {
        let mut defs: Vec<TraitDef> = defs.into_iter().collect();
        defs.sort_by_key(|d| d.axis);
        defs.dedup_by_key(|d| d.axis);
        PersonalityProfile { defs }
    }

    /// The trait definitions, in ascending axis-id order.
    pub fn defs(&self) -> &[TraitDef] {
        &self.defs
    }

    /// A birth-neutral trait instance for this profile: every defined axis at [`Fixed::ZERO`], the
    /// neutral starting personality the drift moves away from toward each axis's target. A being
    /// expresses its own inherited starting personality in the full model; this is the substrate's
    /// neutral seed, so a target above zero reads as a rising trait and one below as a falling one.
    pub fn birth_instance(&self) -> TraitInstance {
        let values = self.defs.iter().map(|d| (d.axis, Fixed::ZERO)).collect();
        TraitInstance { values }
    }
}

/// The per-race registry of personality profiles (a data-defined substrate sibling to the value and
/// axiom substrates, Principle 11): the mechanism is fixed, the membership is per-race data and grows
/// with the world. A race with no entry authors no personality drift, so a world that installs no
/// registry runs exactly as before (the same fail-quiet-until-declared convention the mortality
/// hazard uses).
#[derive(Clone, Debug, Default)]
pub struct PersonalityRegistry {
    profiles: BTreeMap<RaceId, PersonalityProfile>,
}

impl PersonalityRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        PersonalityRegistry {
            profiles: BTreeMap::new(),
        }
    }

    /// Register a race's personality profile, replacing any earlier one for that race.
    pub fn set(&mut self, race: RaceId, profile: PersonalityProfile) {
        self.profiles.insert(race, profile);
    }

    /// The profile for a race, or `None` if the race authors no personality drift.
    pub fn profile(&self, race: RaceId) -> Option<&PersonalityProfile> {
        self.profiles.get(&race)
    }

    /// Whether the registry carries any profile at all (used to skip the beat cheaply when no world
    /// personality is installed).
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

/// A being's live personality: its current value on each trait axis, in the canonical axis order the
/// registry defines. Absent axes read as [`Fixed::ZERO`], so a being reads the neutral value on an
/// axis its instance has not been seeded with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraitInstance {
    values: BTreeMap<TraitAxisId, Fixed>,
}

impl TraitInstance {
    /// An empty trait instance (every axis neutral at zero).
    pub fn new() -> Self {
        TraitInstance {
            values: BTreeMap::new(),
        }
    }

    /// A trait instance from explicit `(axis, value)` pairs (for tests and for a being that carries
    /// an inherited starting personality).
    pub fn from_values(values: impl IntoIterator<Item = (TraitAxisId, Fixed)>) -> Self {
        TraitInstance {
            values: values.into_iter().collect(),
        }
    }

    /// The being's current value on a trait axis, neutral zero where the axis is not present.
    pub fn value(&self, axis: TraitAxisId) -> Fixed {
        self.values.get(&axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// The `(axis, value)` pairs this instance carries, in ascending axis-id order (the `BTreeMap`'s
    /// canonical walk), so a state hash folds a being's personality in one deterministic order.
    pub fn entries(&self) -> impl Iterator<Item = (TraitAxisId, Fixed)> + '_ {
        self.values.iter().map(|(&axis, &value)| (axis, value))
    }

    /// Set the being's value on a trait axis.
    pub fn set(&mut self, axis: TraitAxisId, value: Fixed) {
        self.values.insert(axis, value);
    }
}

/// The age-scaled plasticity of one trait for a being of a race at `age` life-cadence steps: the
/// race's plasticity curve evaluated at its own maturation fraction (design Part 20). The result is
/// clamped to the unit interval, so it reads as a per-step fraction of the gap to the target (a rate,
/// the same clamp the belief-diffusion coupling uses), high in youth and low at the maturity plateau
/// where the curve flattens. The slowdown timing derives entirely from the race's `maturity_years`
/// through [`Race::maturation_fraction`]: a slow-maturing race stays plastic for more absolute years
/// from the one curve, with no per-race branch (Principle 9).
pub fn plasticity_at(def: &TraitDef, race: &Race, age: u32) -> Fixed {
    def.plasticity_curve
        .eval(race.maturation_fraction(age))
        .clamp(Fixed::ZERO, Fixed::ONE)
}

/// The age-personality kernel: drift a being's personality one life-cadence step. For each trait the
/// race defines (in canonical axis order), the value moves toward the axis's maturity target by the
/// age-scaled plasticity fraction of the remaining gap:
///
/// `new = value + plasticity * (target - value)`.
///
/// This is a deterministic exponential approach: a trait far from its target moves fastest, and as
/// plasticity falls to its low plateau by maturity the trait settles near its target and barely
/// moves after, matching the rising rank-order stability of the maturity principle. Because
/// plasticity is clamped to `[0, 1]` the value stays between its old value and the target, so the
/// drift is bounded and cannot overshoot or oscillate. Nothing here reads a race id; a race whose
/// trait rises and one whose trait falls run the identical arithmetic and diverge only through their
/// per-race curve and target (Principle 9). Pure fixed-point, no RNG, so the trajectory replays bit
/// for bit (Principle 3).
pub fn age_personality(
    inst: &mut TraitInstance,
    profile: &PersonalityProfile,
    race: &Race,
    age: u32,
) {
    for def in profile.defs() {
        let plasticity = plasticity_at(def, race, age);
        let cur = inst.value(def.axis);
        let gap = def.maturity_target - cur;
        let next = cur + plasticity.mul(gap);
        inst.set(def.axis, next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axiom::{Axiom, AxiomAxisId, EpistemicStance, EvidenceRing, IntrinsicBeliefs};
    use crate::value::{ValueAxisId, ValueProfile};
    use crate::SourceModeId;
    use civsim_bio::genome::{
        Channel, CognitionChannel, DominanceMode, GeneDef, GeneEffect, GeneId, GenePool, GeneSet,
        GeneticScheme, ReproductionMode, SchemeId,
    };

    /// A minimal intrinsic-belief seed (the personality kernel reads none of it; the Race just
    /// needs one). A labelled fixture, not owner data.
    fn beliefs() -> IntrinsicBeliefs {
        IntrinsicBeliefs {
            values: ValueProfile::with([(ValueAxisId(0), 1)]),
            axioms: vec![Axiom {
                axis: AxiomAxisId(0),
                stance: Fixed::ZERO,
                strength: Fixed::from_ratio(1, 2),
                confidence: Fixed::from_ratio(1, 2),
                entrenchment: 1,
                salience: Fixed::from_ratio(1, 2),
                stubbornness: Fixed::from_ratio(1, 8),
                innate_seed: Fixed::ZERO,
                evidence: EvidenceRing::new(3),
            }],
            epistemic: EpistemicStance::new(
                [(SourceModeId(1), Fixed::ONE)],
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ZERO,
            ),
        }
    }

    /// A race carrying only the identity and the owner-set lifespan and maturity the kernel reads
    /// through [`Race::maturation_fraction`]. Everything else is a shared labelled fixture, so any
    /// divergence between two such races is forced through their per-race data alone.
    fn race(id: u32, maturity_years: u32, lifespan_years: u32) -> Race {
        let genes = GeneSet {
            genes: vec![GeneDef {
                id: GeneId(0),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::ReasoningAcuity),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            }],
        };
        let pool = GenePool::new(SchemeId(0), 20, vec![Fixed::from_ratio(1, 2)]);
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
            beliefs(),
            Fixed::ZERO,
            Fixed::ZERO,
            lifespan_years,
            maturity_years,
        )
    }

    /// A plasticity curve high in youth (fraction 0) and low at the maturity plateau (fraction 1),
    /// the shape the maturity principle gives: personality changes fast when young and settles.
    fn youth_high_curve() -> Curve {
        Curve::new([
            (Fixed::ZERO, Fixed::from_ratio(1, 2)),
            (Fixed::ONE, Fixed::from_ratio(1, 20)),
        ])
    }

    const AXIS: TraitAxisId = TraitAxisId(0);

    #[test]
    fn plasticity_is_high_in_youth_and_low_by_maturity() {
        let r = race(0, 20, 80);
        let def = TraitDef {
            axis: AXIS,
            plasticity_curve: youth_high_curve(),
            maturity_target: Fixed::ONE,
        };
        let young = plasticity_at(&def, &r, 0);
        let mature = plasticity_at(&def, &r, 20);
        let past = plasticity_at(&def, &r, 40);
        assert!(
            young > mature,
            "plasticity falls with age ({young:?} > {mature:?})"
        );
        assert_eq!(
            mature, past,
            "past maturity the fraction saturates at one, so plasticity plateaus"
        );
    }

    #[test]
    fn two_races_diverge_from_data_one_rising_one_falling() {
        // One kernel, no race branch: race A's trait rises (target above the neutral birth), race B's
        // falls (target below), purely from their per-race TraitDef data.
        let rising = TraitDef {
            axis: AXIS,
            plasticity_curve: youth_high_curve(),
            maturity_target: Fixed::from_ratio(4, 5),
        };
        let falling = TraitDef {
            axis: AXIS,
            plasticity_curve: youth_high_curve(),
            maturity_target: Fixed::from_ratio(-4, 5),
        };
        let race_a = race(1, 20, 80);
        let race_b = race(2, 20, 80);
        let profile_a = PersonalityProfile::new([rising]);
        let profile_b = PersonalityProfile::new([falling]);
        let mut inst_a = profile_a.birth_instance();
        let mut inst_b = profile_b.birth_instance();

        // Both start neutral at zero.
        assert_eq!(inst_a.value(AXIS), Fixed::ZERO);
        assert_eq!(inst_b.value(AXIS), Fixed::ZERO);

        // Age both through their maturation and beyond, one life-cadence step at a time.
        for age in 1..=40u32 {
            age_personality(&mut inst_a, &profile_a, &race_a, age);
            age_personality(&mut inst_b, &profile_b, &race_b, age);
        }

        assert!(
            inst_a.value(AXIS) > Fixed::ZERO,
            "race A's trait rose toward its positive target ({:?})",
            inst_a.value(AXIS)
        );
        assert!(
            inst_b.value(AXIS) < Fixed::ZERO,
            "race B's trait fell toward its negative target ({:?})",
            inst_b.value(AXIS)
        );
        // The magnitudes mirror each other (mirror-image data through one kernel), proving no
        // directional bias lives in the mechanism. The one kernel runs the identical arithmetic for
        // both; they diverge only through the sign of their target. The two trajectories are exact
        // negatives up to the floor rounding of `Fixed::mul` on a negative operand, so `a + b` sits
        // within a few least-significant bits of zero rather than exactly at it.
        let residual = (inst_a.value(AXIS).to_bits() + inst_b.value(AXIS).to_bits()).abs();
        assert!(
            residual < 1024,
            "mirror-image targets give mirror-image trajectories from the one kernel (residual {residual} bits)"
        );
    }

    #[test]
    fn a_slow_maturing_race_stays_plastic_longer() {
        // Same curve; the only difference is maturity_years. The slowdown timing is not a hardcoded
        // age: the fast race reaches its low plateau at age 10 (maturation fraction saturates), while
        // at that same age the slow race is only a tenth matured and stays plastic. So at a late age
        // the slow race's plasticity is strictly higher, purely from its per-race datum through the
        // one maturation-fraction path (Principle 9).
        let def = TraitDef {
            axis: AXIS,
            plasticity_curve: youth_high_curve(),
            maturity_target: Fixed::ONE,
        };
        let fast = race(1, 10, 40);
        let slow = race(2, 100, 400);
        let p_fast = plasticity_at(&def, &fast, 30);
        let p_slow = plasticity_at(&def, &slow, 30);
        assert!(
            p_slow > p_fast,
            "the slow-maturing race is still plastic where the fast one has plateaued ({p_slow:?} > {p_fast:?})"
        );
        // And the fast race has reached its low plateau (maturation fraction is saturated at one).
        assert_eq!(
            p_fast,
            plasticity_at(&def, &fast, 999),
            "past maturity the fast race's plasticity is flat at its plateau"
        );
    }

    #[test]
    fn the_drift_is_deterministic() {
        let def = TraitDef {
            axis: AXIS,
            plasticity_curve: youth_high_curve(),
            maturity_target: Fixed::from_ratio(3, 5),
        };
        let r = race(1, 20, 80);
        let profile = PersonalityProfile::new([def]);
        let mut a = profile.birth_instance();
        let mut b = profile.birth_instance();
        for age in 1..=25u32 {
            age_personality(&mut a, &profile, &r, age);
            age_personality(&mut b, &profile, &r, age);
        }
        assert_eq!(a, b, "the same data and ages reproduce the same trajectory");
    }
}
