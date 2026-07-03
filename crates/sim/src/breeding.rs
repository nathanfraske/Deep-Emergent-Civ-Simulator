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

//! The sex / breeding-system substrate (design Part 25, R-REPRO).
//!
//! A being's sex is a gene-fed phenotype, not a drawn attribute. It is read off a designated
//! sex-determination locus through the ordinary expression map ([`crate::genome::GeneSet::express`]
//! on [`crate::genome::Channel::SexDetermination`]), and a data-driven [`BreedingSystem`] maps the
//! expressed value to a [`SexClass`]. Because the sex ratio is a genotype-to-phenotype reading and
//! not an authored number, a 1:1 ratio emerges from Fisherian equal-investment selection on the
//! locus rather than being drawn or reserved (Principle 8, Principle 9). There is deliberately no
//! `Phase::SEX_DETERMINE` and no reserved ratio value: sex determination adds no RNG draw of its
//! own, so it is a pure deterministic function of the genome.
//!
//! The number of sex classes is data, never a closed binary enum. Binary anisogamy (two classes,
//! sperm and egg) is one registry entry; a race may carry N mating types (the fungal model) or be
//! hermaphroditic (one class, self-compatible). The mechanism that reads a [`BreedingSystem`] is
//! fixed Rust; the membership of the [`BreedingSystemRegistry`] is data and grows with the world,
//! sibling to the value substrate (Part 21), the semantic substrate (Part 33), and the
//! institution-function substrate (Part 36). The default compatibility rule is "any two distinct
//! classes pair"; a hermaphroditic entry allows self or any.
//!
//! What a [`SexClass`] feeds downstream: the reproductive-success census
//! ([`crate::census::ReproductiveCensus`]) tallies each breeder's sex and offspring count so an
//! effective population size Ne can be derived through one race-blind kernel. This module carries
//! only the phenotype and the mating-type registry; the Ne derivation lives in [`crate::census`].

use std::collections::BTreeMap;

use civsim_core::Fixed;

/// A sex or mating-type class: the phenotype a genotype expresses through the sex-determination
/// locus. A plain `u16` index into a [`BreedingSystem`]'s class list, so two classes (anisogamy),
/// N classes (mating types), or one (hermaphroditism) are the same type at different cardinalities.
/// Ordered so a census walks classes canonically (ascending id).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct SexClass(pub u16);

/// A data-defined breeding-system identifier (Part 40), carried on a [`crate::race::Race`] and
/// resolved through a [`BreedingSystemRegistry`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct BreedingSystemId(pub u32);

/// How a genotype's expressed sex-determination value is assigned to a [`SexClass`]. The kinds are
/// fixed mechanism affordances; which one a system uses, and its data (the class thresholds), is
/// registry data.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AssignmentRule {
    /// One class only (hermaphroditic): every individual is class 0 regardless of the expressed
    /// value. The single class breeds with self or any other.
    Monomorphic,
    /// The expressed value is partitioned into classes by ascending thresholds: a value below
    /// `thresholds[0]` is class 0, below `thresholds[1]` is class 1, and so on, with the last class
    /// taking everything at or above the final threshold. The threshold count is one less than the
    /// class count. This is the general anisogamy and N-mating-type assignment; the thresholds are
    /// data (they place the class boundaries on the locus's expressed scale).
    Thresholds(Vec<Fixed>),
}

/// How two classes decide whether they may pair. Fixed mechanism affordances; which one a system
/// uses is data. The default is [`CompatibilityRule::DistinctClasses`] ("any two distinct classes
/// pair"), the anisogamy and N-mating-type rule; a hermaphroditic system uses
/// [`CompatibilityRule::SelfOrOther`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompatibilityRule {
    /// Any two distinct classes pair; a class does not pair with itself (the anisogamy and
    /// mating-type default).
    DistinctClasses,
    /// Any pair is compatible, including a class with itself (the hermaphroditic rule).
    SelfOrOther,
}

/// One breeding system: its sex classes, how a genotype assigns to one, and how two classes decide
/// compatibility (design Part 25, R-REPRO). The mechanism is fixed Rust; the class count, the
/// assignment thresholds, and the compatibility rule are data, so binary anisogamy, N mating types,
/// and hermaphroditism are all one type at different data (Principle 11). The `label` is a
/// human-readable tag for dev fixtures and diagnostics only, never keyed on.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BreedingSystem {
    /// The system's identifier.
    pub id: BreedingSystemId,
    /// A human-readable label (diagnostics only).
    pub label: String,
    /// The sex classes this system carries, in canonical order. At least one.
    pub classes: Vec<SexClass>,
    /// How an expressed sex-determination value assigns to a class.
    pub assignment: AssignmentRule,
    /// How two classes decide whether they may pair.
    pub compatibility: CompatibilityRule,
}

impl BreedingSystem {
    /// The number of sex classes (mating types) this system carries.
    pub fn class_count(&self) -> usize {
        self.classes.len()
    }

    /// Assign a [`SexClass`] to an expressed sex-determination value (the value read off the locus
    /// through [`crate::genome::GeneSet::express`]). A pure function, so sex is deterministic from
    /// the genome with no RNG. A degenerate system with no classes returns [`SexClass::default`]
    /// rather than panicking, so a misconfigured registry fails soft into a single class.
    pub fn assign(&self, expressed: Fixed) -> SexClass {
        if self.classes.is_empty() {
            return SexClass::default();
        }
        match &self.assignment {
            AssignmentRule::Monomorphic => self.classes[0],
            AssignmentRule::Thresholds(thresholds) => {
                let mut idx = 0usize;
                for t in thresholds {
                    if expressed < *t {
                        break;
                    }
                    idx += 1;
                }
                self.classes[idx.min(self.classes.len() - 1)]
            }
        }
    }

    /// Whether two classes may pair under this system's compatibility rule. The default rule is
    /// "any two distinct classes pair"; a hermaphroditic system also allows self.
    pub fn compatible(&self, a: SexClass, b: SexClass) -> bool {
        match self.compatibility {
            CompatibilityRule::DistinctClasses => a != b,
            CompatibilityRule::SelfOrOther => true,
        }
    }

    /// A labelled dev fixture: a binary anisogamous system (two classes, sperm and egg), the
    /// gonochoric default. Class 0 is expressed below the midpoint threshold, class 1 at or above
    /// it, and the two distinct classes pair. The threshold and weights are fixture values, not
    /// owner data.
    pub fn dev_binary_anisogamy(id: BreedingSystemId) -> Self {
        BreedingSystem {
            id,
            label: "binary-anisogamous".to_string(),
            classes: vec![SexClass(0), SexClass(1)],
            assignment: AssignmentRule::Thresholds(vec![Fixed::from_ratio(1, 2)]),
            compatibility: CompatibilityRule::DistinctClasses,
        }
    }

    /// A labelled dev fixture: a multi-mating-type system with `k` classes (the fungal model), to
    /// exercise the N-class generality. The expressed value is partitioned into `k` equal bins over
    /// the unit interval, and any two distinct types pair. `k` is clamped to at least one.
    pub fn dev_multi_mating_type(id: BreedingSystemId, k: u16) -> Self {
        let k = k.max(1);
        let classes: Vec<SexClass> = (0..k).map(SexClass).collect();
        let thresholds: Vec<Fixed> = (1..k)
            .map(|i| Fixed::from_ratio(i as i64, k as i64))
            .collect();
        BreedingSystem {
            id,
            label: format!("multi-mating-type-{k}"),
            classes,
            assignment: AssignmentRule::Thresholds(thresholds),
            compatibility: CompatibilityRule::DistinctClasses,
        }
    }

    /// A labelled dev fixture: a hermaphroditic system (one class, self-compatible), the
    /// monomorphic case that reduces Ne's sex term to no reduction at all.
    pub fn dev_hermaphroditic(id: BreedingSystemId) -> Self {
        BreedingSystem {
            id,
            label: "hermaphroditic".to_string(),
            classes: vec![SexClass(0)],
            assignment: AssignmentRule::Monomorphic,
            compatibility: CompatibilityRule::SelfOrOther,
        }
    }
}

/// The registry of breeding systems in a world (design Part 25, R-REPRO), sibling to the value and
/// domain registries: the mechanism that reads it is fixed Rust, the membership is data and grows
/// with the world (Principle 11). A race names its system by [`BreedingSystemId`]; an id with no
/// entry resolves to `None`, so a world with no registered system falls back to a single
/// (hermaphroditic-like) class, never a fabricated ratio.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct BreedingSystemRegistry {
    systems: BTreeMap<BreedingSystemId, BreedingSystem>,
}

impl BreedingSystemRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        BreedingSystemRegistry::default()
    }

    /// Register a breeding system, keyed by its id (overwriting any prior entry at that id).
    pub fn insert(&mut self, system: BreedingSystem) {
        self.systems.insert(system.id, system);
    }

    /// The breeding system at an id, if registered.
    pub fn get(&self, id: BreedingSystemId) -> Option<&BreedingSystem> {
        self.systems.get(&id)
    }

    /// How many systems are registered.
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// Whether the registry holds no systems.
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// A labelled dev-default registry: the binary anisogamous system at id 0, a multi-mating-type
    /// system (three types) at id 1, and a hermaphroditic system at id 2, so the generality is
    /// exercised from one fixture. Never authoritative; a real world registers its own data.
    pub fn dev_default() -> Self {
        let mut reg = BreedingSystemRegistry::new();
        reg.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
        reg.insert(BreedingSystem::dev_multi_mating_type(
            BreedingSystemId(1),
            3,
        ));
        reg.insert(BreedingSystem::dev_hermaphroditic(BreedingSystemId(2)));
        reg
    }
}

/// The Fisherian equal-investment selection coefficient on the class-1-producing allele, given the
/// current fraction of class-1 individuals and the parental investment cost of each class (design
/// Part 25, R-REPRO; Fisher 1930). The reproductive value of a class is inversely proportional to
/// its frequency, because a member of the rarer sex has more mating opportunity, so a gene biasing
/// toward the rarer sex spreads; the equal-investment weighting divides that value by the class's
/// cost. The coefficient is `(RV_1 / cost_1) / (RV_2 / cost_2) - 1`, which with `RV_i = 1/(2 f_i)`
/// reduces to `(f_2 cost_2) / (f_1 cost_1) - 1`. It is zero (an evolutionary fixed point) exactly
/// when `f_1 cost_1 == f_2 cost_2`, so under equal cost the fixed point is `f_1 == f_2`, the 1:1
/// ratio, which emerges here rather than being written anywhere. The fraction is clamped off the
/// endpoints so the reciprocals never divide by zero.
pub fn sex_ratio_selection_coeff(frac_class1: Fixed, cost1: Fixed, cost2: Fixed) -> Fixed {
    let eps = Fixed::from_ratio(1, 1_000_000);
    let f1 = frac_class1.clamp(eps, Fixed::ONE - eps);
    let f2 = Fixed::ONE - f1;
    let num = f2.mul(cost2);
    let den = f1.mul(cost1);
    if den == Fixed::ZERO {
        return Fixed::ZERO;
    }
    num.div(den) - Fixed::ONE
}

/// One episode of equal-investment selection on the sex-determination allele: given the current
/// class-1 fraction `p` and the two classes' investment costs, return the next-generation fraction
/// under the standard gene-frequency selection map `p' = p(1 + s) / (1 + p s)`, where `s` is
/// [`sex_ratio_selection_coeff`]. The map drives `p` to the equal-investment fixed point from any
/// interior start; under equal cost that point is 1:1 (0.5), and under unequal cost it shifts to
/// `cost_2 / (cost_1 + cost_2)`, so the ratio is derived from the investment assumption rather than
/// hardcoded. Clamped to the unit interval. Deterministic, no RNG.
pub fn fisher_select_step(p: Fixed, cost1: Fixed, cost2: Fixed) -> Fixed {
    let s = sex_ratio_selection_coeff(p, cost1, cost2);
    let denom = Fixed::ONE + p.mul(s);
    if denom == Fixed::ZERO {
        return p;
    }
    p.mul(Fixed::ONE + s)
        .div(denom)
        .clamp(Fixed::ZERO, Fixed::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{
        Allele, Channel, DominanceMode, GeneDef, GeneEffect, GeneId, GeneSet, Genome, Haplotype,
        SchemeId,
    };

    const SEX: Channel = Channel::SexDetermination;

    /// A one-locus gene set whose single gene feeds the sex-determination channel with unit weight,
    /// so `express(SexDetermination)` returns the summed allele values at that locus. This is the
    /// designated sex-determination locus; that it feeds `SexDetermination` is data (a `GeneEffect`),
    /// the engine only knows the channel.
    fn sex_gene_set() -> GeneSet {
        GeneSet {
            genes: vec![GeneDef {
                id: GeneId(0),
                effects: vec![GeneEffect {
                    channel: SEX,
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            }],
        }
    }

    /// A haploid genome carrying one sex-determination allele of the given additive value.
    fn sex_genome(additive: Fixed) -> Genome {
        Genome {
            scheme: SchemeId(0),
            haps: vec![Haplotype {
                alleles: vec![Allele::additive(additive)],
            }],
        }
    }

    #[test]
    fn sex_is_a_gene_fed_phenotype_read_through_express() {
        // The locus expresses a value; the breeding system assigns the class. A being carrying the
        // low allele reads class 0, the high allele class 1, entirely through the ordinary
        // expression map, with no bespoke sex phase and no draw.
        let genes = sex_gene_set();
        let system = BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0));
        let low = genes.express(&sex_genome(Fixed::ZERO), SEX, Fixed::ZERO);
        let high = genes.express(&sex_genome(Fixed::ONE), SEX, Fixed::ZERO);
        assert_eq!(system.assign(low), SexClass(0));
        assert_eq!(system.assign(high), SexClass(1));
    }

    #[test]
    fn mating_types_are_data_driven_not_a_closed_binary_enum() {
        // Two classes is one entry; N classes is another, from the same type. A three-type system
        // partitions the expressed value into three classes and pairs any two distinct types.
        let two = BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0));
        let many = BreedingSystem::dev_multi_mating_type(BreedingSystemId(1), 3);
        let herm = BreedingSystem::dev_hermaphroditic(BreedingSystemId(2));
        assert_eq!(two.class_count(), 2);
        assert_eq!(many.class_count(), 3);
        assert_eq!(herm.class_count(), 1);
        // The multi-type assignment lands each bin in its own class.
        assert_eq!(many.assign(Fixed::from_ratio(1, 6)), SexClass(0));
        assert_eq!(many.assign(Fixed::from_ratio(1, 2)), SexClass(1));
        assert_eq!(many.assign(Fixed::from_ratio(5, 6)), SexClass(2));
        // Compatibility: distinct classes pair; a hermaphrodite pairs with self.
        assert!(two.compatible(SexClass(0), SexClass(1)));
        assert!(!two.compatible(SexClass(0), SexClass(0)));
        assert!(many.compatible(SexClass(0), SexClass(2)));
        assert!(!many.compatible(SexClass(2), SexClass(2)));
        assert!(herm.compatible(SexClass(0), SexClass(0)));
    }

    #[test]
    fn registry_resolves_and_grows_like_a_data_substrate() {
        let reg = BreedingSystemRegistry::dev_default();
        assert_eq!(reg.len(), 3);
        assert_eq!(reg.get(BreedingSystemId(0)).unwrap().class_count(), 2);
        assert_eq!(reg.get(BreedingSystemId(1)).unwrap().class_count(), 3);
        assert!(
            reg.get(BreedingSystemId(99)).is_none(),
            "an unregistered id resolves to None"
        );
    }

    /// The realised class-1 fraction of a cohort at sex-determination allele frequency `p`: a
    /// fraction `p` of the cohort carries the high (class-1) allele, the rest the low one; each
    /// being's sex is read off the locus through `express` and assigned by the system. This ties the
    /// population sex ratio to the locus so the Fisherian dynamic below acts on the real phenotype.
    fn realised_class1_fraction(p: Fixed, cohort: u32, system: &BreedingSystem) -> Fixed {
        let genes = sex_gene_set();
        let carriers = p.mul(Fixed::from_int(cohort as i32)).to_int().max(0) as u32;
        let mut class1 = 0u32;
        for i in 0..cohort {
            let genome = if i < carriers {
                sex_genome(Fixed::ONE)
            } else {
                sex_genome(Fixed::ZERO)
            };
            let expressed = genes.express(&genome, SEX, Fixed::ZERO);
            if system.assign(expressed) == SexClass(1) {
                class1 += 1;
            }
        }
        Fixed::from_ratio(class1 as i64, cohort as i64)
    }

    #[test]
    fn sex_ratio_emerges_one_to_one_from_locus_under_equal_investment() {
        // The Fisherian claim, made concrete on the locus: a 1:1 ratio is the fixed point of
        // equal-investment selection on the sex-determination allele, reached from any starting
        // ratio, and it is nowhere written as 0.5. The realised ratio is read off the locus.
        let system = BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0));
        let half = Fixed::from_ratio(1, 2);
        let equal = Fixed::ONE; // equal parental investment cost per class
        let tol = Fixed::from_ratio(1, 100);

        for &start in &[Fixed::from_ratio(1, 5), Fixed::from_ratio(4, 5)] {
            // The realised sex ratio tracks the allele frequency read off the locus.
            let realised = realised_class1_fraction(start, 100, &system);
            assert!((realised - start).abs() <= Fixed::from_ratio(2, 100));
            // One episode of equal-investment selection reaches the equilibrium; iterating holds it.
            let mut p = start;
            for _ in 0..8 {
                p = fisher_select_step(p, equal, equal);
            }
            assert!(
                (p - half).abs() <= tol,
                "equal investment drives the ratio to 1:1 from any start, not to a written 0.5"
            );
        }
        // The 1:1 point is a genuine fixed point: applied there, it does not move.
        assert!((fisher_select_step(half, equal, equal) - half).abs() <= tol);

        // Non-hardcoded: shift the investment costs and the equilibrium shifts off 1:1, to the
        // derived point cost2 / (cost1 + cost2). Sons costing twice as much settle near 1:2.
        let cost_male = Fixed::from_int(2);
        let cost_female = Fixed::ONE;
        let expected = cost_female.div(cost_male + cost_female); // 1/3, derived from the costs
        let mut p = Fixed::from_ratio(1, 2);
        for _ in 0..8 {
            p = fisher_select_step(p, cost_male, cost_female);
        }
        assert!(
            (p - expected).abs() <= tol,
            "unequal investment shifts the equilibrium off 1:1, proving 0.5 is derived not authored"
        );
        assert!(
            (expected - half).abs() > Fixed::from_ratio(1, 10),
            "the biased equilibrium is well away from 1:1"
        );
    }
}
