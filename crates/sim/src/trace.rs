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

//! The trace-kind substrate and the pure derivations that free the evidence engine's authored
//! trace numbers (design Part 9.9, Part 40; the derive half of R-EVIDENCE `implication_weights`
//! and `trace_decay_curves`).
//!
//! A trace is a perceptible consequence an event leaves in the world (a corpse, a bloodstain, a
//! corroded blade). The evidence engine once carried two numbers per trace as owner calibration:
//! the belief weight a trace implies about its cause ("a fresh corpse is worth 5, a stale stain
//! 1"), and how a trace's perceptibility decays over time ("blood lasts days, bone lasts years").
//! Both are terran-forensic tables a hive, a silicate, or a dispersing-body race has no analogue
//! for. This module replaces the authored tables with a data substrate plus fixed derivations:
//! a trace kind carries its reliability and its decay law as data, and the weight and the salience
//! are COMPUTED from that data through cited primitives (Good's weight of evidence over the race's
//! own base rates, [`crate::evidence::good_weight`]; the physics reaction and corrosion kernels,
//! [`civsim_physics::laws`]).
//!
//! Principle 9 holds throughout: the derivation functions read per-trace-kind and per-race DATA
//! and never branch on a concrete [`TraceKindId`] or [`crate::value::RaceId`]. Swap two races' base
//! rates and the weights swap; swap two kinds' susceptibilities and the decay speeds swap; the
//! mechanism authors none of it.
//!
//! Scope: the derivations are pure and standalone. Wiring them into the running world (recomputing
//! a placed [`crate::world::Trace`]'s salience from its age and its place's temperature, and
//! feeding the implication weight into the perception step) is a NAMED FOLLOW-ON, deferred so this
//! build stays additive: the public `Trace` struct is untouched (no time-varying salience field,
//! no `created_tick` sweep), and the global `World` mortality path is untouched.

use civsim_core::Fixed;
use civsim_physics::laws;

use crate::base_rates::RaceBaseRates;
use crate::evidence::{good_weight, AttrKindId, ValueId};

/// A data-defined trace-kind identifier (a newtype like [`AttrKindId`]), not a closed enum, so a
/// world can carry trace kinds the engine's authors never enumerated (Principle 11). File order in
/// the [`TraceKindRegistry`] is mint order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TraceKindId(pub u32);

/// One implication a trace kind carries: perceiving the trace is evidence about a subject's
/// attribute toward a particular value (for a corpse: the attribute "vital status" toward "dead").
/// The pair is data; a trace kind can imply several things.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TraceImplicationSpec {
    /// The attribute the trace speaks to.
    pub attr: AttrKindId,
    /// The value the trace is evidence toward.
    pub toward: ValueId,
}

/// The physical law a trace's perceptibility decays under.
///
/// This is a CLOSED enum over the fixed physics kernels ([`civsim_physics::laws`]), not an open
/// registry, and that is a deliberate OWNER CALL flagged here. The argument for a closed enum:
/// `physics::laws` is itself a fixed, hand-authored kernel set (reaction, corrosion, and their
/// siblings are Rust functions, not data), so a decay law can only ever bind to a kernel that
/// exists in code; the membership cannot grow with world data the way a value axis or an
/// institution function can. The argument against (for the owner to weigh): if decay laws should
/// be composable or world-authored the way the value and semantic substrates are, this should
/// become a data-defined registry keyed to a kernel by id. The build leans enum, mirroring how the
/// laws it dispatches to are themselves a fixed kernel set; the reserved parameters inside each
/// variant are the data.
#[derive(Clone, Debug)]
pub enum DecayLaw {
    /// Organic decomposition: gated on thermal activity (the trace decomposes only above an
    /// activation barrier, so a frozen remains is preserved), then an exponential decay in elapsed
    /// time at a decomposition rate. Dispatches to [`civsim_physics::laws::reaction`] for the gate
    /// and [`Fixed::exp`] for the decay.
    Organic {
        /// The thermal-activation barrier: decomposition proceeds only where temperature reaches it
        /// (the generalization of an ignition or reaction threshold). RESERVED. Basis: the material's
        /// reaction barrier the physics data already defines.
        barrier: Fixed,
        /// The per-unit-elapsed decomposition rate (the exponential decay constant). RESERVED. Basis:
        /// the material's decomposition susceptibility through the reaction kernel, scaled per race by
        /// the race's own `decay_multiplier`.
        decomposition_rate: Fixed,
    },
    /// Electrochemical corrosion: a driving margin (oxidiser minus material potential, times
    /// susceptibility, times an acidity factor) integrated linearly against elapsed time.
    /// Dispatches to [`civsim_physics::laws::corrosion`].
    Corroding {
        /// The material's own electrode potential against the reference oxidiser. RESERVED. Basis:
        /// the material's measured potential the physics data defines; an active (corroding)
        /// material carries a negative potential against the reference.
        material_potential: Fixed,
        /// The material's corrosion susceptibility. RESERVED. Basis: the material's reactivity the
        /// physics data defines.
        susceptibility: Fixed,
        /// The environment acidity (pH), where a lower value is more aggressive. RESERVED. Basis: the
        /// chem.acidity axis the physics data defines.
        acidity: Fixed,
        /// The saturating cap on the corrosion rate. RESERVED. Basis: the rate ceiling the corrosion
        /// kernel clamps against.
        corrosion_max: Fixed,
    },
    /// A permanent trace: no physical decay (a carved mark, a moved boulder). Salience holds at
    /// full for any elapsed time.
    Static,
}

/// A trace-kind definition: its reliability, what it implies, and how it decays. All data
/// (Principle 11); the mechanisms that consume it are fixed Rust.
#[derive(Clone, Debug)]
pub struct TraceKindDef {
    /// The kind's identifier.
    pub id: TraceKindId,
    /// `P(trace arises | its implied cause is true)`: how reliably the cause leaves this trace.
    /// RESERVED. Basis: the likelihood the causal event produces the trace, the numerator of Good's
    /// weight of evidence, per trace kind rather than a shared human table.
    pub reliability: Fixed,
    /// `P(trace arises | its implied cause is FALSE)`: the false-attribution likelihood, how often
    /// this trace appears when the implied cause did not occur (a corpse from a decoy, a bloodstain
    /// from a non-fatal injury). RESERVED. Basis: the base incidence of the trace absent its cause,
    /// the DENOMINATOR of Good's weight of evidence. Distinct from the race's background mortality,
    /// which is the belief's PRIOR (applied via [`crate::evidence::InferenceFrame::seed_prior`]),
    /// never part of the weight. A near-zero value makes the trace strongly diagnostic; a value near
    /// the reliability makes it nearly useless as evidence.
    pub false_attribution: Fixed,
    /// What perceiving the trace is evidence about (a trace kind can imply several things).
    pub implies: Vec<TraceImplicationSpec>,
    /// The physical law the trace's perceptibility decays under.
    pub decay: DecayLaw,
}

/// The set of trace kinds a world runs, in file (mint) order so a kind's position is stable and a
/// walk is reproducible. Data-defined and extensible: a new trace kind is covered the moment it is
/// registered.
#[derive(Clone, Debug, Default)]
pub struct TraceKindRegistry(pub Vec<TraceKindDef>);

impl TraceKindRegistry {
    /// The kind definition for an id, if registered.
    pub fn kind(&self, id: TraceKindId) -> Option<&TraceKindDef> {
        self.0.iter().find(|k| k.id == id)
    }

    /// A labelled DEVELOPMENT FIXTURE, not owner values, so the derivations run and can be tested
    /// now. A corpse (a reliable death-implication that decomposes organically), a bloodstain (a
    /// less reliable one that also decomposes), and a corroded blade (a corrosion trace). The
    /// numbers are fixtures to exercise the derivations, never fabricated calibrations; the real
    /// reliabilities and decay parameters are the owner's per-kind data.
    pub fn dev_default() -> TraceKindRegistry {
        TraceKindRegistry(vec![
            TraceKindDef {
                id: DEV_CORPSE,
                reliability: Fixed::from_ratio(9, 10),
                // A corpse almost never arises when the subject is alive: a small false-attribution
                // likelihood, so it is strongly diagnostic of death.
                false_attribution: Fixed::from_ratio(1, 100),
                implies: vec![TraceImplicationSpec {
                    attr: AttrKindId(0),
                    toward: 1,
                }],
                decay: DecayLaw::Organic {
                    barrier: Fixed::from_int(0),
                    decomposition_rate: Fixed::from_ratio(1, 100),
                },
            },
            TraceKindDef {
                id: DEV_BLOODSTAIN,
                reliability: Fixed::from_ratio(1, 2),
                // Blood is a weaker death-signal: a living subject sheds it from a non-fatal injury
                // fairly often, so the false-attribution likelihood is higher and the trace less
                // diagnostic.
                false_attribution: Fixed::from_ratio(1, 5),
                implies: vec![TraceImplicationSpec {
                    attr: AttrKindId(0),
                    toward: 1,
                }],
                decay: DecayLaw::Organic {
                    barrier: Fixed::from_int(0),
                    decomposition_rate: Fixed::from_ratio(1, 20),
                },
            },
            TraceKindDef {
                id: DEV_CORRODED_BLADE,
                reliability: Fixed::from_ratio(3, 4),
                false_attribution: Fixed::from_ratio(1, 10),
                implies: vec![TraceImplicationSpec {
                    attr: AttrKindId(1),
                    toward: 1,
                }],
                decay: DecayLaw::Corroding {
                    material_potential: Fixed::from_ratio(-44, 100),
                    susceptibility: Fixed::from_ratio(1, 100),
                    acidity: Fixed::from_int(7),
                    corrosion_max: Fixed::ONE,
                },
            },
        ])
    }
}

/// The corpse dev-fixture trace kind (a leaf id, not special-cased in any mechanism).
pub const DEV_CORPSE: TraceKindId = TraceKindId(0);
/// The bloodstain dev-fixture trace kind.
pub const DEV_BLOODSTAIN: TraceKindId = TraceKindId(1);
/// The corroded-blade dev-fixture trace kind.
pub const DEV_CORRODED_BLADE: TraceKindId = TraceKindId(2);

/// The weight of evidence a mortality-implying trace of `kind` carries: Good's log-likelihood ratio
/// of two LIKELIHOODS, `ln[P(trace | cause true) / P(trace | cause false)]`
/// ([`crate::evidence::good_weight`]), with the trace kind's `reliability` as `P(trace | dead)` and
/// its `false_attribution` as `P(trace | alive)`. Both are per-trace-kind data (Principle 11), so a
/// strongly diagnostic trace (low false attribution) carries a large weight and a weak one a small
/// weight, and swapping two kinds' likelihoods swaps their weights.
///
/// The base rate (a race's background mortality, `P(dead)`) is the belief's PRIOR, not the weight:
/// it enters through [`crate::evidence::InferenceFrame::seed_prior`] at wire-up, never here. Feeding
/// the prior into the likelihood-ratio slot (the earlier form) double-counted it as evidence and
/// conflated a prior with a likelihood. The `clamp` is the evidence engine's certainty clamp, reused
/// rather than re-invented.
pub fn mortality_implication_weight(kind: &TraceKindDef, clamp: Fixed) -> Fixed {
    good_weight(kind.reliability, kind.false_attribution, clamp)
}

/// The perceptibility (salience) remaining on an organic trace of `kind` after `elapsed` time at
/// ambient `temperature`, for a member of the race whose base rates are `race`.
///
/// The physics reaction kernel ([`civsim_physics::laws::reaction`]) gates thermal activity: below
/// the kind's activation barrier decomposition halts and a frozen remains keeps full salience;
/// only the boolean gate is consumed here, and the mass-weighted Hess-law enthalpy sums are a
/// named follow-on once a remains carries its own substance vector, so they enter as zero (their
/// difference, the discarded enthalpy, does not gate the salience). Past the barrier the salience
/// decays exponentially in elapsed time ([`Fixed::exp`]) at the kind's decomposition rate scaled by
/// the race's own `decay_multiplier`, so a race whose remains break down faster loses salience
/// faster through this one function rather than a per-race branch. A non-organic kind has no
/// organic decay and reads full salience.
///
/// `temperature` is an explicit argument; the `PlaceId -> (x, y) -> temperature` wiring that would
/// read it from the trace's location in the running world is a named follow-on.
pub fn organic_salience(
    elapsed: Fixed,
    temperature: Fixed,
    kind: &TraceKindDef,
    race: &RaceBaseRates,
) -> Fixed {
    let (barrier, decomposition_rate) = match kind.decay {
        DecayLaw::Organic {
            barrier,
            decomposition_rate,
        } => (barrier, decomposition_rate),
        _ => return Fixed::ONE,
    };
    // The reaction kernel reports whether temperature crosses the activation barrier. Only that
    // gate is used; the enthalpy sums are the deferred follow-on and enter as zero.
    let (_delta_h, thermally_active) =
        laws::reaction(Fixed::ZERO, Fixed::ZERO, temperature, barrier);
    if !thermally_active {
        // Below the barrier decomposition halts: a preserved remains keeps full salience.
        return Fixed::ONE;
    }
    // Effective decomposition rate: the kind's rate scaled by the race's own decay multiplier.
    let rate = match decomposition_rate.checked_mul(race.decay_multiplier) {
        Some(r) => r,
        // An unrepresentably large rate is effectively instantaneous decay: nothing remains.
        None => return Fixed::ZERO,
    };
    let scaled = match rate.checked_mul(elapsed) {
        Some(s) => s,
        None => return Fixed::ZERO,
    };
    // salience = exp(-rate * elapsed), a value in (0, 1]; the negation is saturating so an extreme
    // exponent cannot panic (exp itself saturates outside its representable window).
    let exponent = Fixed::from_bits(0i64.saturating_sub(scaled.to_bits()));
    exponent.exp()
}

/// The perceptibility (salience) remaining on a corroding trace of `kind` after `elapsed` time.
///
/// The physics corrosion kernel ([`civsim_physics::laws::corrosion`]) reports the corrosion rate
/// (a driving margin) from the material potential, susceptibility, and acidity the kind carries;
/// the rate is integrated linearly against elapsed time, and the salience is what remains. A more
/// corrosion-prone kind (a higher susceptibility) loses salience strictly faster at a fixed
/// elapsed, through this one function rather than a per-kind branch. A non-corroding kind reads
/// full salience.
///
/// The oxidiser (fluid) potential is the definitional reference of the electrode-potential scale
/// (the Standard Hydrogen Electrode, zero by convention, a scale zero and not an authored value);
/// an active material carries a negative potential against it, so its driving margin is positive
/// and it corrodes. The per-place medium chemistry that would supply the actual oxidiser potential
/// is a named follow-on, exactly as the temperature wiring is for organic decay.
pub fn corroding_salience(elapsed: Fixed, kind: &TraceKindDef) -> Fixed {
    let (material_potential, susceptibility, acidity, corrosion_max) = match kind.decay {
        DecayLaw::Corroding {
            material_potential,
            susceptibility,
            acidity,
            corrosion_max,
        } => (material_potential, susceptibility, acidity, corrosion_max),
        _ => return Fixed::ONE,
    };
    let rate = laws::corrosion(
        Fixed::ZERO,
        material_potential,
        susceptibility,
        acidity,
        corrosion_max,
    );
    let loss = match rate.checked_mul(elapsed) {
        Some(l) => l,
        // An unrepresentably large accumulated loss leaves nothing.
        None => return Fixed::ZERO,
    };
    // salience = 1 - rate*elapsed, clamped into the unit interval; the subtraction is saturating so
    // a loss beyond one cannot panic before the clamp.
    let remaining = Fixed::from_bits(Fixed::ONE.to_bits().saturating_sub(loss.to_bits()));
    remaining.clamp(Fixed::ZERO, Fixed::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_rates::RaceBaseRateRegistry;
    use crate::decision::Curve;
    use crate::value::RaceId;

    const CLAMP: Fixed = Fixed::from_int(20);

    fn flat_mortality(num: i64, den: i64) -> Curve {
        Curve::new([(Fixed::from_int(0), Fixed::from_ratio(num, den))])
    }

    fn base_rates(race: u32, mortality: Curve, decay_multiplier: Fixed) -> RaceBaseRates {
        RaceBaseRates {
            race: RaceId(race),
            natural_mortality: mortality,
            visibility: Fixed::from_ratio(1, 2),
            decay_multiplier,
        }
    }

    fn corpse() -> TraceKindDef {
        TraceKindRegistry::dev_default()
            .kind(DEV_CORPSE)
            .unwrap()
            .clone()
    }

    // === The implication weight is Good's log-likelihood ratio of two LIKELIHOODS, not a prior ===

    #[test]
    fn the_weight_is_the_log_likelihood_ratio_of_two_likelihoods_not_a_prior() {
        // Regression (audit defect 6): the weight equals good_weight over the trace kind's two
        // LIKELIHOODS (reliability = P(trace|dead), false_attribution = P(trace|alive)), never the
        // race's background mortality (a PRIOR). It is exactly Good's log-likelihood ratio.
        let kind = corpse();
        let w = mortality_implication_weight(&kind, CLAMP);
        assert_eq!(
            w,
            good_weight(kind.reliability, kind.false_attribution, CLAMP),
            "the weight is the LLR of the kind's two likelihoods"
        );
        // It does not read the race's base rate at all: the signature no longer takes one, and the
        // weight is a pure function of the kind's likelihoods (the prior is applied separately as the
        // belief's seed_prior, never folded into the weight).
        assert!(
            w > Fixed::ZERO,
            "a reliable, rarely-spurious trace is positive evidence"
        );
    }

    // === Non-steering swap: the trace kind's two likelihoods are the sole author of the weight ===

    #[test]
    fn a_more_diagnostic_trace_kind_carries_more_weight() {
        // A strongly diagnostic kind (high reliability, low false attribution) carries strictly more
        // weight of evidence than a weak one (lower reliability, higher false attribution). Swapping
        // the two kinds' likelihoods swaps the assignment, so the weight tracks the per-kind data,
        // never a label.
        let strong = TraceKindDef {
            id: DEV_CORPSE,
            reliability: Fixed::from_ratio(9, 10),
            false_attribution: Fixed::from_ratio(1, 100),
            implies: vec![],
            decay: DecayLaw::Static,
        };
        let weak = TraceKindDef {
            id: DEV_BLOODSTAIN,
            reliability: Fixed::from_ratio(1, 2),
            false_attribution: Fixed::from_ratio(1, 5),
            implies: vec![],
            decay: DecayLaw::Static,
        };
        let w_strong = mortality_implication_weight(&strong, CLAMP);
        let w_weak = mortality_implication_weight(&weak, CLAMP);
        assert!(
            w_strong > w_weak,
            "the more diagnostic kind is worth more ({w_strong:?} > {w_weak:?})"
        );
        // Swap the likelihoods between the two ids: the assignment swaps, so the id carries no bias.
        let strong2 = TraceKindDef {
            id: DEV_CORPSE,
            reliability: Fixed::from_ratio(1, 2),
            false_attribution: Fixed::from_ratio(1, 5),
            implies: vec![],
            decay: DecayLaw::Static,
        };
        let weak2 = TraceKindDef {
            id: DEV_BLOODSTAIN,
            reliability: Fixed::from_ratio(9, 10),
            false_attribution: Fixed::from_ratio(1, 100),
            implies: vec![],
            decay: DecayLaw::Static,
        };
        assert!(
            mortality_implication_weight(&strong2, CLAMP)
                < mortality_implication_weight(&weak2, CLAMP),
            "swapping the likelihoods swaps which id carries more weight"
        );
    }

    // === Non-steering swap (3): corrosion susceptibility is the sole author of the decay speed ===

    fn corroding_kind(susceptibility: Fixed) -> TraceKindDef {
        TraceKindDef {
            id: DEV_CORRODED_BLADE,
            reliability: Fixed::from_ratio(3, 4),
            false_attribution: Fixed::from_ratio(1, 10),
            implies: vec![],
            decay: DecayLaw::Corroding {
                material_potential: Fixed::from_ratio(-44, 100),
                susceptibility,
                acidity: Fixed::from_int(7),
                corrosion_max: Fixed::ONE,
            },
        }
    }

    #[test]
    fn a_more_corrosion_prone_kind_decays_strictly_faster() {
        let elapsed = Fixed::from_int(2);
        let prone = corroding_kind(Fixed::from_ratio(3, 100));
        let resistant = corroding_kind(Fixed::from_ratio(1, 100));
        let s_prone = corroding_salience(elapsed, &prone);
        let s_resistant = corroding_salience(elapsed, &resistant);
        assert!(
            s_prone < s_resistant,
            "the more corrosion-prone kind has less salience left ({s_prone:?} < {s_resistant:?})"
        );

        // Swap the susceptibilities and the faster-decaying kind swaps with them.
        let prone2 = corroding_kind(Fixed::from_ratio(1, 100));
        let resistant2 = corroding_kind(Fixed::from_ratio(3, 100));
        assert!(
            corroding_salience(elapsed, &prone2) > corroding_salience(elapsed, &resistant2),
            "swapping susceptibility swaps which decays faster"
        );
    }

    #[test]
    fn corrosion_accumulates_with_elapsed_time() {
        let kind = corroding_kind(Fixed::from_ratio(2, 100));
        let early = corroding_salience(Fixed::from_int(1), &kind);
        let late = corroding_salience(Fixed::from_int(3), &kind);
        assert!(
            early > late,
            "salience falls as corrosion accumulates ({early:?} > {late:?})"
        );
        assert!(
            early <= Fixed::ONE && late >= Fixed::ZERO,
            "salience stays in the unit interval"
        );
    }

    // === Organic decay: the thermal gate and the exponential fall ===

    #[test]
    fn organic_decay_is_gated_by_the_thermal_barrier() {
        let reg = RaceBaseRateRegistry::dev_default();
        let race = reg.get(crate::base_rates::DEV_LONGLIVED).unwrap();
        // A kind that only decomposes above a warm barrier.
        let kind = TraceKindDef {
            id: DEV_CORPSE,
            reliability: Fixed::from_ratio(9, 10),
            false_attribution: Fixed::from_ratio(1, 100),
            implies: vec![],
            decay: DecayLaw::Organic {
                barrier: Fixed::from_int(10),
                decomposition_rate: Fixed::from_ratio(1, 10),
            },
        };
        let elapsed = Fixed::from_int(20);
        // Below the barrier (frozen): preserved, full salience.
        let cold = organic_salience(elapsed, Fixed::from_int(0), &kind, race);
        assert_eq!(cold, Fixed::ONE, "below the barrier a remains is preserved");
        // Above the barrier (warm): decomposed, salience strictly below full.
        let warm = organic_salience(elapsed, Fixed::from_int(30), &kind, race);
        assert!(
            warm < Fixed::ONE,
            "above the barrier a remains decomposes ({warm:?})"
        );
        assert!(
            warm > Fixed::ZERO,
            "some salience remains after finite decay"
        );
    }

    #[test]
    fn a_faster_decaying_race_loses_organic_salience_faster() {
        // Two races identical except decay_multiplier: the higher multiplier decays faster.
        let fast = base_rates(0, flat_mortality(1, 10), Fixed::from_int(2));
        let slow = base_rates(1, flat_mortality(1, 10), Fixed::ONE);
        let kind = corpse(); // barrier 0, so any positive temperature is thermally active
        let elapsed = Fixed::from_int(50);
        let temp = Fixed::from_int(20);
        let s_fast = organic_salience(elapsed, temp, &kind, &fast);
        let s_slow = organic_salience(elapsed, temp, &kind, &slow);
        assert!(
            s_fast < s_slow,
            "the higher decay multiplier leaves less salience ({s_fast:?} < {s_slow:?})"
        );
    }

    #[test]
    fn a_static_kind_never_decays() {
        let reg = RaceBaseRateRegistry::dev_default();
        let race = reg.get(crate::base_rates::DEV_LONGLIVED).unwrap();
        let kind = TraceKindDef {
            id: TraceKindId(9),
            reliability: Fixed::ONE,
            false_attribution: Fixed::from_ratio(1, 100),
            implies: vec![],
            decay: DecayLaw::Static,
        };
        assert_eq!(
            organic_salience(Fixed::from_int(1000), Fixed::from_int(50), &kind, race),
            Fixed::ONE,
            "a static kind is not organic and reads full salience"
        );
        assert_eq!(
            corroding_salience(Fixed::from_int(1000), &kind),
            Fixed::ONE,
            "a static kind is not corroding and reads full salience"
        );
    }

    // === Principle 9: no derivation branches on a concrete race or trace-kind id ===

    #[test]
    fn derivations_never_branch_on_a_race_or_kind_literal() {
        // The derivation functions must read per-race and per-trace-kind DATA, never special-case a
        // concrete RaceId or TraceKindId. A literal-id comparison or a match on an id would be the
        // steering violation; the fixtures may CONSTRUCT ids, so only the production code (before the
        // test module) is scanned, and the anti-pattern strings below live in the test module and so
        // do not poison their own check.
        let sources = [
            include_str!("trace.rs"),
            include_str!("base_rates.rs"),
            include_str!("absence.rs"),
        ];
        // Comparing an id to a variable (a registry lookup like `k.id == id`) is data-driven and
        // allowed; only a comparison or match against a concrete id LITERAL is the violation.
        let anti = [
            "== RaceId(",
            "== TraceKindId(",
            "RaceId(0) =>",
            "TraceKindId(0) =>",
            "match race.race",
            "match kind.id",
        ];
        for src in sources {
            let code = src.split("#[cfg(test)]").next().unwrap();
            for pattern in anti {
                assert!(
                    !code.contains(pattern),
                    "a derivation branches on an id literal: {pattern}"
                );
            }
        }
        // Positively: the derivations do read the per-race and per-kind data fields.
        let trace_code = include_str!("trace.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(trace_code.contains("kind.reliability"));
        assert!(trace_code.contains("race.decay_multiplier"));
        let absence_code = include_str!("absence.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(absence_code.contains("race.visibility"));
        assert!(
            absence_code.contains("race.natural_mortality")
                || absence_code.contains("natural_mortality")
        );
    }
}
