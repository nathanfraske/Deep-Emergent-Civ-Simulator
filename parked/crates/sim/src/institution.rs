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

//! The emergent-institution and ADICO-norm substrate (design Part 36, record 62.8).
//!
//! An institution carries no authored category. What it is, is read off its structure: its
//! roles, the ADICO-grammar norms it enforces, the legitimacy it holds, its lineage, and above
//! all the function it coordinates, expressed as a blend over an institution-function substrate
//! (the etic floor of what coordination can be about, sibling to the value substrate of Part 21
//! and the semantic substrate of Part 33). Any human-readable type is recovered only as a
//! derived, non-authoritative [`EticDescriptor`] for legibility, never used to steer behaviour.
//!
//! This module builds the substrate the record pins:
//!
//! - the open [`FunctionRegistry`] of function axes (a data registry, not a closed enum, with a
//!   labelled [`FunctionRegistry::dev_seed`] of the human-analogue axes and room for exotic
//!   per-race axes), and the [`FunctionVec`] blend an institution occupies over it;
//! - the [`Institution`] representation and its [`Norm`] in the Ostrom-Crawford ADICO grammar,
//!   whose TYPE (strategy A.I.C, norm A.D.I.C, rule A.D.I.C.O) is DERIVED from which optional
//!   fields are present ([`Norm::norm_type`]), never a stored tag;
//! - the derived [`EticDescriptor`]: a fixed-point weighted-Tanimoto polythetic match against a
//!   [`TemplateLibrary`] that may be EMPTY (in which case every institution reads by its generic
//!   structural description and the engine still runs), plus [`institution_distance`], a
//!   fixed-point structural distance over the shared substrate reusing the value.rs pattern;
//! - the norm-firing predicate [`norm_fires`], the [`DecisionPropensity`] accumulator and
//!   [`emit_undertaking`] gate, the compact [`AggregateInstitution`] the pool tier carries, and
//!   the synthetic-stream [`crystallize`] mechanism.
//!
//! Principle 8 and Principle 9 hold throughout. A race enters ONLY through its function-axis
//! DATA: the crystallization mechanism, the feature extractor, and the distance are label-blind,
//! never branching on a concrete [`FunctionAxisId`], a race, or the draw key. Swap two axes'
//! labels and their columns and every similarity and distance is bit-identical
//! ([`crystallize`]'s tests). Everything is integer and fixed-point; the feature signature is a
//! canonical id-ordered aggregation and the recognition and distance sums are in canonical
//! feature order, so they are bit-identical across machines and thread counts; the one stochastic
//! step (a crystallization tie-break) is keyed on [`Phase::CRYSTALLIZE`] with id-ordered primary
//! ordering; and the [`EticDescriptor`], being a pure function of canonical state, is recomputable
//! on demand and does NOT enter the state hash (Principle 10, the way render state does not).
//!
//! Reserved for owner calibration, surfaced rather than fabricated, each with its Part 36 basis
//! (the reserved list is in the audit log and `calibration/reserved.toml`; the mechanism is fixed
//! Rust and every one of these reaches the code as a supplied parameter, never a hardcoded
//! constant, Principle 11):
//!
//! - `inst.function_substrate_axes`: the membership of the function-axis substrate itself. Basis:
//!   the functional domains a given world distinguishes, with force, the sacred, exchange,
//!   knowledge, and care a starting menu (the [`FunctionRegistry::dev_seed`] human-analogue
//!   fallback) and not a fixed fact, and exotic axes added per race as data.
//! - `inst.feature_weights` (the similarity metric weights): which features the owner treats as
//!   diagnostic of sameness, supplied to [`recognize`].
//! - `inst.distance_weights`: the per-feature weights of the institution-distance metric,
//!   supplied to [`institution_distance`]; which structural and function-space features count
//!   toward how far apart two institutions read.
//! - `inst.recognition_threshold`: the intended trade between over-labelling a novel form and
//!   falling back to a generic description, supplied to [`recognize`].
//! - `inst.crystallization_threshold` and `inst.crystallization_rate`: the propensity threshold a
//!   recurring coordination pattern must cross to crystallize and the per-observation
//!   accumulation rate, supplied to [`crystallize`] as [`CrystallizationParams`]. Basis: the
//!   intended institution-formation cadence in playtest, the hardest of these to set.
//! - `tier.decision_propensity`: the decision-propensity threshold and accumulation rate for
//!   aggregate collective undertakings, matched to the detailed tier rate for comparable
//!   conditions (the same reserved pair the crystallization gate reads through
//!   [`emit_undertaking`]).
//!
//! The LIVE crystallization detector wiring into the running agent loop is a NAMED FOLLOW-ON.
//! This module builds and tests the mechanism against an explicit synthetic
//! coordination-pattern stream ([`crystallize`]); feeding it the real decision layer's
//! coordinated undertakings, minting [`InstId`]s in the world, and recomputing descriptors on the
//! running tick are the deferred wiring, kept out so this build stays additive.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::{DrawKey, EventId, Fixed, InstId, Phase, StableId, StateHasher};

use civsim_foundation::decision::{ActionId, InputId};
use civsim_foundation::stocks::Stock;

// === (A) The institution-function substrate and the institution representation ===

/// A data-defined institution-function axis identifier (a newtype like the value substrate's
/// axis id, not a closed enum), so a world can carry function axes the engine's authors never
/// enumerated (Principle 11). Position in the [`FunctionRegistry`] is insert order, which is the
/// canonical column order a [`FunctionVec`] and a [`FeatureSignature`] are laid out in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FunctionAxisId(pub u32);

/// One function axis as data: its id, a human label, and a description. The label and description
/// are the observer's gloss and never enter a metric (the label-blind guarantee); only the
/// axis's column position and an institution's intensity on it do.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FunctionAxisDef {
    /// The axis identifier.
    pub id: FunctionAxisId,
    /// A human-readable label (observer gloss; never read by a metric).
    pub name: String,
    /// A human-readable description (observer gloss).
    pub description: String,
}

/// The open registry of function axes a world distinguishes: the etic floor of what coordination
/// can be about, authored once like the value and semantic substrates and authorable under
/// Principle 9 as an affordance rather than an outcome. Membership is owner data and grows with
/// the world; the [`FunctionRegistry::dev_seed`] is a clearly-labelled development fixture, NOT
/// the authoritative axis set.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FunctionRegistry {
    axes: Vec<FunctionAxisDef>,
}

impl FunctionRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        FunctionRegistry { axes: Vec::new() }
    }

    /// Append an axis, returning its column position. Axes keep insert order, which is the
    /// canonical column order every [`FunctionVec`] and [`FeatureSignature`] is laid out in.
    pub fn insert(&mut self, def: FunctionAxisDef) -> usize {
        let idx = self.axes.len();
        self.axes.push(def);
        idx
    }

    /// The axis definition for an id, if registered.
    pub fn get(&self, id: FunctionAxisId) -> Option<&FunctionAxisDef> {
        self.axes.iter().find(|a| a.id == id)
    }

    /// The axis at a column position, if any.
    pub fn axis_at(&self, index: usize) -> Option<&FunctionAxisDef> {
        self.axes.get(index)
    }

    /// The number of axes (the width of a [`FunctionVec`] over this substrate).
    pub fn len(&self) -> usize {
        self.axes.len()
    }

    /// Whether the substrate has no axes.
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }

    /// A labelled DEVELOPMENT FIXTURE, not the owner's axis set, so the mechanism runs and can be
    /// tested now. The five human-analogue anthropological axes (Parsons, Malinowski): organized
    /// force, the sacred and the production of legitimacy, exchange and credit, knowledge and its
    /// transmission, and care and provisioning. These are the starting menu the record names, NOT
    /// a fixed fact: an exotic people carries exotic axes (brood-tending, mana-channeling, a
    /// diapause-council) added to this same registry as data, and the reserved
    /// `inst.function_substrate_axes` is where the real per-world axis set is set.
    pub fn dev_seed() -> FunctionRegistry {
        let mut r = FunctionRegistry::new();
        for (i, (name, desc)) in [
            ("force", "organized force and coercion"),
            ("sacred", "the sacred and the production of legitimacy"),
            ("exchange", "exchange, credit, and the medium of value"),
            ("knowledge", "knowledge and its transmission"),
            ("care", "care and provisioning"),
        ]
        .into_iter()
        .enumerate()
        {
            r.insert(FunctionAxisDef {
                id: FunctionAxisId(i as u32),
                name: name.to_string(),
                description: desc.to_string(),
            });
        }
        r
    }
}

/// An emergent blend over the institution-function substrate (design Part 36): one [`Fixed`] per
/// axis, in [`FunctionAxisId`] column order. Not a chosen slot: a body high on both the sacred and
/// exchange axes is a temple that also banks, which no single-variant enum could express.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FunctionVec {
    /// The intensity on each axis, in substrate column order.
    pub intensity: Vec<Fixed>,
}

impl FunctionVec {
    /// A zero blend of the given width.
    pub fn zeros(width: usize) -> Self {
        FunctionVec {
            intensity: vec![Fixed::ZERO; width],
        }
    }

    /// A blend from explicit intensities.
    pub fn from_intensities(v: impl IntoIterator<Item = Fixed>) -> Self {
        FunctionVec {
            intensity: v.into_iter().collect(),
        }
    }

    /// The width (number of axes).
    pub fn width(&self) -> usize {
        self.intensity.len()
    }
}

/// A position within an institution. Structural, not an enum: a role is an id plus the actions it
/// may take (its powers) and the actions it is bound to perform (its duties), both referencing
/// the decision layer's open [`ActionId`] set, so a role carries no authored category.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RoleId(pub u32);

/// One role's structure: its id, its powers (actions it may take), and its duties (actions it is
/// bound to perform). All references into the decision layer's [`ActionId`] set (Principle 4: no
/// new authored action catalogue).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Role {
    /// The role identifier.
    pub id: RoleId,
    /// The actions this role may take.
    pub powers: Vec<ActionId>,
    /// The actions this role is bound to perform.
    pub duties: Vec<ActionId>,
}

/// An explicit, promoted institution (design Part 36). It carries no authored kind tag; its
/// identity is read off the fields below and any type is recovered only as the derived
/// [`Institution::descriptor`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Institution {
    /// The institution identifier (minted in crystallization order; not a category).
    pub id: InstId,
    /// The positions within it and their powers and duties.
    pub roles: Vec<Role>,
    /// The crystallized enforced behaviour, ADICO-shaped ([`Norm`]).
    pub rules: Vec<Norm>,
    /// The emergent blend over the institution-function substrate.
    pub coordinates: FunctionVec,
    /// A belief held by the governed (Part 9): erodes toward revolt.
    pub legitimacy: Fixed,
    /// Whatever the institution controls (treasury, holdings, sacra), reusing the ecological
    /// [`Stock`] compartment (Part 15) rather than a new resource type.
    pub resources: Vec<Stock>,
    /// The members it binds, by stable id.
    pub members: Vec<StableId>,
    /// Provenance to the crystallizing behaviour (the Principle 9 gate).
    pub founded: EventId,
    /// Institutions spawn and reform from others.
    pub parent: Option<InstId>,
    /// DERIVED for legibility only; recomputed from structure, never a behavioural input, and NOT
    /// folded into the state hash ([`Institution::hash_into`]).
    pub descriptor: EticDescriptor,
}

impl Institution {
    /// Extract this institution's feature signature by canonical id-ordered aggregation over its
    /// roles, members, and norms (design Part 36, Part 58). The width is the coordinate width; a
    /// counterpart [`AggregateInstitution`] built over the same substrate reproduces this vector
    /// exactly. The counts are order-independent aggregations, so shuffling the insertion order of
    /// roles, members, or norms leaves the signature bit-identical.
    pub fn feature_signature(&self) -> FeatureSignature {
        // A RULE is deontic AND sanction (A.D.I.C.O), not a bare sanction: gate on both, matching
        // Norm::norm_type. Counting `or_else.is_some()` alone miscounted a sanction-without-deontic
        // (an ill-formed O-without-D, which reads as a strategy) as a rule.
        let rules = self
            .rules
            .iter()
            .filter(|n| n.deontic.is_some() && n.or_else.is_some())
            .count() as i32;
        let obligations = self.rules.iter().filter(|n| n.deontic.is_some()).count() as i32;
        FeatureSignature::build(
            &self.coordinates.intensity,
            self.roles.len() as i32,
            self.members.len() as i32,
            self.rules.len() as i32,
            rules,
            obligations,
        )
    }

    /// The intensive blend a descriptor reads: the coordinate mass divided by the member count
    /// (with a zero count reading as the mass itself, so an institution with no members still has
    /// a readable blend). Distances and descriptors are computed over the extensive signature; this
    /// is the intensive projection for glosses.
    pub fn coordinates_intensity(&self) -> FunctionVec {
        intensive_blend(&self.coordinates.intensity, self.members.len() as i32)
    }

    /// The derived etic descriptor: the nearest recognition template above the reserved threshold,
    /// or generic. A pure function of canonical state (the signature, the library, the threshold),
    /// recomputable on demand, so it need never enter the state hash.
    pub fn descriptor_from(
        &self,
        library: &TemplateLibrary,
        threshold: Fixed,
        weights: &[Fixed],
    ) -> EticDescriptor {
        recognize(&self.feature_signature(), library, threshold, weights)
    }

    /// Recompute and store the descriptor (a convenience; the stored value is non-authoritative
    /// and out of the hash, so this can be called any time without affecting determinism).
    pub fn recompute_descriptor(
        &mut self,
        library: &TemplateLibrary,
        threshold: Fixed,
        weights: &[Fixed],
    ) {
        self.descriptor = self.descriptor_from(library, threshold, weights);
    }

    /// The dominant function axis (the highest-intensity column), for a generic structural gloss
    /// when no template matches. Returns the column index and the registry label, or `None` for a
    /// zero blend. Ties resolve to the lowest column, so the read is deterministic.
    pub fn dominant_axis(&self, registry: &FunctionRegistry) -> Option<(usize, String)> {
        let blend = self.coordinates_intensity();
        let mut best: Option<(usize, Fixed)> = None;
        for (i, &v) in blend.intensity.iter().enumerate() {
            match best {
                Some((_, bv)) if v <= bv => {}
                _ => best = Some((i, v)),
            }
        }
        best.filter(|(_, v)| *v > Fixed::ZERO).map(|(i, _)| {
            let label = registry
                .axis_at(i)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| format!("axis {i}"));
            (i, label)
        })
    }

    /// A generic structural description built from the dominant axis and the membership shape, for
    /// the case (including an empty template library) where the descriptor reads generic. Observer
    /// prose only; never an input.
    pub fn generic_gloss(&self, registry: &FunctionRegistry) -> String {
        match self.dominant_axis(registry) {
            Some((_, label)) => format!(
                "a {label}-coordinating body ({} roles, {} members, {} norms)",
                self.roles.len(),
                self.members.len(),
                self.rules.len()
            ),
            None => format!(
                "a structural body ({} roles, {} members, {} norms)",
                self.roles.len(),
                self.members.len(),
                self.rules.len()
            ),
        }
    }

    /// Fold the institution into a state hash in canonical order, folding EVERY authoritative
    /// field EXCEPT the derived [`Institution::descriptor`] (design Part 36, Principle 10): the
    /// descriptor is a pure function of the rest and recomputable, so it stays out of the hash the
    /// way render state does. Collections are sorted by their canonical id and length-prefixed.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u32(self.id.0);
        // Roles, by RoleId.
        let mut roles: Vec<&Role> = self.roles.iter().collect();
        roles.sort_by_key(|r| r.id);
        h.write_u64(roles.len() as u64);
        for r in roles {
            h.write_u32(r.id.0);
            hash_action_set(h, &r.powers);
            hash_action_set(h, &r.duties);
        }
        // Coordinates, length-prefixed in column order.
        h.write_u64(self.coordinates.intensity.len() as u64);
        for &c in &self.coordinates.intensity {
            h.write_fixed(c);
        }
        // Norms, sorted by their canonical key.
        let mut norms: Vec<&Norm> = self.rules.iter().collect();
        norms.sort_by_key(|n| n.sort_key());
        h.write_u64(norms.len() as u64);
        for n in norms {
            n.hash_into(h);
        }
        h.write_fixed(self.legitimacy);
        // Resources, in declared order (a treasury has no intrinsic id; the order is canonical).
        h.write_u64(self.resources.len() as u64);
        for s in &self.resources {
            h.write_fixed(s.amount());
            h.write_fixed(s.capacity());
            h.write_fixed(s.regen_rate());
        }
        // Members, by StableId.
        let mut members = self.members.clone();
        members.sort();
        h.write_u64(members.len() as u64);
        for m in members {
            h.write_stable(m);
        }
        h.write_u64(self.founded.0);
        h.write_u32(self.parent.is_some() as u32);
        h.write_u32(self.parent.map(|p| p.0).unwrap_or(0));
        // self.descriptor is deliberately NOT folded (Principle 10).
    }
}

fn hash_action_set(h: &mut StateHasher, actions: &[ActionId]) {
    let mut a: Vec<ActionId> = actions.to_vec();
    a.sort();
    h.write_u64(a.len() as u64);
    for x in a {
        h.write_u32(x.0);
    }
}

/// The intensive blend of an extensive coordinate mass over a member count: mass / count, or the
/// mass itself when the count is zero. Deterministic fixed-point division per component.
fn intensive_blend(mass: &[Fixed], count: i32) -> FunctionVec {
    if count <= 0 {
        return FunctionVec::from_intensities(mass.iter().copied());
    }
    let n = Fixed::from_int(count);
    FunctionVec::from_intensities(mass.iter().map(|&m| m.div(n)))
}

// === (B) The ADICO grammar ===

/// The deontic operator of a norm: a CLOSED enum, because deontic logic is complete and mutually
/// exclusive over one act (a logical primitive like the physics-kernel set, not world content). A
/// norm either permits, requires, or forbids its aim, or carries no deontic at all (a bare
/// strategy), which is the `Option<Deontic>` on [`Norm`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Deontic {
    /// The aim is permitted.
    May,
    /// The aim is required.
    Must,
    /// The aim is forbidden.
    MustNot,
}

impl Deontic {
    /// A stable discriminant for hashing.
    fn disc(self) -> u32 {
        match self {
            Deontic::May => 0,
            Deontic::Must => 1,
            Deontic::MustNot => 2,
        }
    }
}

/// To whom a norm applies: a set of roles and a set of specific members, selected by id, or the
/// society-wide `all` attribute (a law over everyone). Data, no enum: the ids are open, so a norm
/// binds whatever roles or members a world has, and a society-wide taboo is `all = true`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AttributeSel {
    /// The roles the norm binds, by id.
    pub roles: Vec<RoleId>,
    /// The specific members the norm binds, by id.
    pub members: Vec<StableId>,
    /// Whether the norm binds every member (a society-wide attribute).
    pub all: bool,
}

impl AttributeSel {
    /// A society-wide attribute (binds everyone).
    pub fn everyone() -> Self {
        AttributeSel {
            roles: Vec::new(),
            members: Vec::new(),
            all: true,
        }
    }

    /// An attribute binding a single role.
    pub fn role(id: RoleId) -> Self {
        AttributeSel {
            roles: vec![id],
            members: Vec::new(),
            all: false,
        }
    }

    fn hash_into(&self, h: &mut StateHasher) {
        let mut roles = self.roles.clone();
        roles.sort();
        h.write_u64(roles.len() as u64);
        for r in roles {
            h.write_u32(r.0);
        }
        let mut members = self.members.clone();
        members.sort();
        h.write_u64(members.len() as u64);
        for m in members {
            h.write_stable(m);
        }
        h.write_u32(self.all as u32);
    }
}

/// A minimal comparison predicate: the fixed logical primitive set an ADICO condition atom reads
/// a world fact through (a closed set of comparators, like [`Deontic`], not authored world
/// content). Which world facts exist is open (the [`InputId`] registry); how a value is compared
/// to a threshold is one of these three.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Predicate {
    /// The fact's value is at least the threshold.
    AtLeast,
    /// The fact's value is below the threshold.
    Below,
    /// The fact's value equals the threshold.
    Equal,
}

impl Predicate {
    fn disc(self) -> u32 {
        match self {
            Predicate::AtLeast => 0,
            Predicate::Below => 1,
            Predicate::Equal => 2,
        }
    }

    fn holds(self, value: Fixed, threshold: Fixed) -> bool {
        match self {
            Predicate::AtLeast => value >= threshold,
            Predicate::Below => value < threshold,
            Predicate::Equal => value == threshold,
        }
    }
}

/// One condition atom: a world fact (keyed off the decision layer's open [`InputId`] registry), a
/// comparison predicate, and a threshold. Not a rich authored AST: an atom is a single fact
/// compared to a value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Atom {
    /// Which world fact this atom reads (the data-defined [`InputId`] registry key).
    pub input: InputId,
    /// How the fact's value is compared to the threshold.
    pub predicate: Predicate,
    /// The comparison threshold.
    pub threshold: Fixed,
}

/// The condition under which a norm applies: a conjunction of [`Atom`]s (all must hold). An empty
/// conjunction is unconditional (the norm always applies). This keys off the [`InputId`] world-fact
/// registry and a minimal predicate set rather than a closed predicate enum or a rich authored
/// syntax tree (the owner's condition-grammar decision).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ConditionExpr {
    /// The atoms, all of which must hold for the condition to be satisfied.
    pub all_of: Vec<Atom>,
}

impl ConditionExpr {
    /// The unconditional condition (always satisfied).
    pub fn always() -> Self {
        ConditionExpr { all_of: Vec::new() }
    }

    /// A single-atom condition.
    pub fn atom(input: InputId, predicate: Predicate, threshold: Fixed) -> Self {
        ConditionExpr {
            all_of: vec![Atom {
                input,
                predicate,
                threshold,
            }],
        }
    }

    fn hash_into(&self, h: &mut StateHasher) {
        // Atoms are folded in declared order; a caller building a canonical condition sorts them,
        // but the conjunction's meaning is order-independent, so the sort is not required for
        // correctness, only for a canonical hash. We sort by (input, predicate, threshold).
        let mut atoms = self.all_of.clone();
        atoms.sort_by_key(|a| (a.input.0, a.predicate.disc(), a.threshold.to_bits()));
        h.write_u64(atoms.len() as u64);
        for a in atoms {
            h.write_u32(a.input.0);
            h.write_u32(a.predicate.disc());
            h.write_fixed(a.threshold);
        }
    }
}

/// A reference to a prescribed action, reusing the decision layer's open [`ActionId`] set rather
/// than authoring a new action catalogue (Principle 4).
pub type ActionRef = ActionId;

/// A reference to a sanction, also an [`ActionId`]: a sanction is an action taken against a
/// violator, drawn from the same open set.
pub type SanctionRef = ActionId;

/// A norm in the Ostrom-Crawford ADICO grammar (design Part 36). The statement's TYPE is emergent
/// from which components are present ([`Norm::norm_type`]): strategy = A.I.C, norm = A.D.I.C, rule
/// = A.D.I.C.O. There is no stored type tag; membership-gating and succession are ordinary norms
/// rather than authored sub-enums.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Norm {
    /// To whom it applies (which roles or members).
    pub attribute: AttributeSel,
    /// May / must / must-not; absent (`None`) means a mere strategy.
    pub deontic: Option<Deontic>,
    /// The action prescribed (an [`ActionId`]).
    pub aim: ActionRef,
    /// When, where, and how it applies.
    pub condition: ConditionExpr,
    /// The consequence of violation; absent means norm, present means rule.
    pub or_else: Option<SanctionRef>,
    /// How reliably enforced (crystallization strength).
    pub enforcement: Fixed,
    /// The repeated enforced behaviour it crystallized from (the Principle 9 provenance gate).
    pub provenance: EventId,
}

/// A total canonical ordering key for a norm: (provenance, aim, enforcement bits, deontic tag,
/// sanction tag, attribute digest, condition digest). Used to sort norms deterministically for
/// hashing and to dedup distinct norms in [`crystallize`], so the aggregation is a pure function of
/// the norm content rather than its arrival order. The two trailing 128-bit digests fold the
/// attribute (whom the norm binds) and the condition (when it fires), so two norms that differ ONLY
/// in role/member binding or firing condition get distinct keys: the earlier key omitted both, so
/// such distinct norms collided (undercounting the dedup) and sorted by arrival order (a
/// non-canonical hash).
pub type NormKey = (u64, u32, i64, u32, u32, u128, u128);

/// The emergent type of a norm, DERIVED from which optional fields are present, never stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NormType {
    /// A.I.C: an attribute, an aim, and a condition, with no deontic. A shared strategy.
    Strategy,
    /// A.D.I.C: a deontic added, but no sanction. A norm proper.
    Norm,
    /// A.D.I.C.O: a deontic and an or-else sanction. A rule.
    Rule,
}

impl Norm {
    /// The emergent norm type, read purely from which optional fields are present (design Part
    /// 36): no deontic is a strategy; a deontic without a sanction is a norm; a deontic with a
    /// sanction is a rule. A sanction without a deontic (an ill-formed O without a D) reads as a
    /// strategy, since the deontic is the operative component.
    pub fn norm_type(&self) -> NormType {
        match (self.deontic.is_some(), self.or_else.is_some()) {
            (false, _) => NormType::Strategy,
            (true, false) => NormType::Norm,
            (true, true) => NormType::Rule,
        }
    }

    /// A total canonical ordering key for deterministic norm sorts and tie-breaks, a FULL norm
    /// identity: the scalar components plus 128-bit content digests of the attribute (whom it binds)
    /// and the condition (when it fires). Two norms that differ only in attribute or condition get
    /// distinct keys, so [`crystallize`]'s dedup counts them separately and the norm sort in
    /// [`Institution::hash_into`] orders them stably. The scalar prefix leads, so previously-distinct
    /// norms keep their old relative order (the digests are consulted only on a scalar-prefix tie).
    fn sort_key(&self) -> NormKey {
        let mut ah = StateHasher::new();
        self.attribute.hash_into(&mut ah);
        let mut ch = StateHasher::new();
        self.condition.hash_into(&mut ch);
        (
            self.provenance.0,
            self.aim.0,
            self.enforcement.to_bits(),
            self.deontic.map(|d| d.disc() + 1).unwrap_or(0),
            self.or_else.map(|s| s.0 + 1).unwrap_or(0),
            ah.finish(),
            ch.finish(),
        )
    }

    fn hash_into(&self, h: &mut StateHasher) {
        self.attribute.hash_into(h);
        h.write_u32(self.deontic.map(|d| d.disc() + 1).unwrap_or(0));
        h.write_u32(self.aim.0);
        self.condition.hash_into(h);
        h.write_u32(self.or_else.is_some() as u32);
        h.write_u32(self.or_else.map(|s| s.0).unwrap_or(0));
        h.write_fixed(self.enforcement);
        h.write_u64(self.provenance.0);
    }
}

// === (C) The etic descriptor, the feature signature, recognition, and distance ===

/// The number of structural feature components after the function-axis columns in a
/// [`FeatureSignature`]: role count, member count, norm count, rule count, obligation count.
pub const STRUCTURAL_FEATURES: usize = 5;

/// A recognition-template identifier (a newtype), templates iterated in ascending id order so a
/// tie in similarity resolves to the lowest id deterministically.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TemplateId(pub u32);

/// A descriptive recognition template (design Part 36): a data-defined prototype feature vector
/// with per-feature weights, in canonical feature order. Templates are the observer's vocabulary,
/// read only in the recognition pass, with no path back into behaviour; the owner may ship none.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RecognitionTemplate {
    /// The template identifier.
    pub id: TemplateId,
    /// The prototype feature vector, in canonical feature order.
    pub features: Vec<Fixed>,
    /// The per-feature weights (which features are diagnostic of this template).
    pub weights: Vec<Fixed>,
}

/// The library of recognition templates. May be EMPTY, in which case every institution reads by
/// its generic structural description and the engine still runs (the record's safety net against
/// the recognition library being hard to calibrate).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TemplateLibrary {
    templates: Vec<RecognitionTemplate>,
}

impl TemplateLibrary {
    /// An empty library.
    pub fn new() -> Self {
        TemplateLibrary {
            templates: Vec::new(),
        }
    }

    /// Add a template.
    pub fn insert(&mut self, t: RecognitionTemplate) {
        self.templates.push(t);
    }

    /// The templates in ascending [`TemplateId`] order (the canonical recognition order).
    pub fn in_order(&self) -> Vec<&RecognitionTemplate> {
        let mut v: Vec<&RecognitionTemplate> = self.templates.iter().collect();
        v.sort_by_key(|t| t.id);
        v
    }

    /// How many templates the library holds.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether the library is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

/// The canonical extensive feature signature of an institution (design Part 36, Part 58). Layout,
/// in canonical feature order: the `k` function-axis coordinate-mass components (extensive over the
/// shared substrate), then the [`STRUCTURAL_FEATURES`] structural counts as [`Fixed`] integers
/// (roles, members, norms, rules, obligations). Every component is extensive, so a merge sums it
/// componentwise and a promotion moves it across the tier boundary unchanged: the vector is a
/// conserved projection (R-PROJ-REGISTER, R-TIER-CONSIST). The coordinate mass a metric reads as
/// an intensity is the mass divided by the member-count component.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FeatureSignature {
    /// The feature values in canonical order.
    pub values: Vec<Fixed>,
}

impl FeatureSignature {
    /// Build a signature from the coordinate mass and the five structural counts.
    pub fn build(
        coords: &[Fixed],
        roles: i32,
        members: i32,
        norms: i32,
        rules: i32,
        obligations: i32,
    ) -> Self {
        let mut values = coords.to_vec();
        values.push(Fixed::from_int(roles));
        values.push(Fixed::from_int(members));
        values.push(Fixed::from_int(norms));
        values.push(Fixed::from_int(rules));
        values.push(Fixed::from_int(obligations));
        FeatureSignature { values }
    }

    /// The number of function-axis columns this signature carries (its length minus the fixed
    /// structural tail).
    pub fn axis_width(&self) -> usize {
        self.values.len().saturating_sub(STRUCTURAL_FEATURES)
    }

    /// The coordinate-mass slice (the function-substrate part of the signature).
    pub fn coordinate_mass(&self) -> &[Fixed] {
        &self.values[..self.axis_width()]
    }

    /// The member-count structural component as a `u32` (the second structural feature).
    pub fn member_count(&self) -> u32 {
        let k = self.axis_width();
        self.values
            .get(k + 1)
            .map(|f| f.to_int().max(0) as u32)
            .unwrap_or(0)
    }

    /// The role, member, norm, rule, obligation counts as integers.
    pub fn structural_counts(&self) -> [i32; STRUCTURAL_FEATURES] {
        let k = self.axis_width();
        let mut out = [0i32; STRUCTURAL_FEATURES];
        for (j, o) in out.iter_mut().enumerate() {
            *o = self.values.get(k + j).map(|f| f.to_int()).unwrap_or(0);
        }
        out
    }

    /// The total feature mass as a 128-bit bit sum, the conserved scalar the projection registry
    /// measures (fixed-point addition in `i128` space is exact and order-independent).
    pub fn mass_bits(&self) -> i128 {
        Fixed::sum_bits(self.values.iter().copied())
    }

    /// Componentwise sum with another signature (a merge). The two must share a width; a mismatch
    /// pads the shorter with zeros so the operation is total.
    pub fn add(&self, other: &FeatureSignature) -> FeatureSignature {
        let n = self.values.len().max(other.values.len());
        let mut values = vec![Fixed::ZERO; n];
        for (i, v) in values.iter_mut().enumerate() {
            let a = self.values.get(i).copied().unwrap_or(Fixed::ZERO);
            let b = other.values.get(i).copied().unwrap_or(Fixed::ZERO);
            *v = a + b;
        }
        FeatureSignature { values }
    }

    /// Partition into two signatures that sum back to this one exactly (a split). The function-axis
    /// coordinate mass is halved by raw bits with the remainder to the first part; the structural
    /// counts are halved as integers with the remainder to the first part, so both parts carry
    /// valid integer counts and their componentwise sum reconstructs the original exactly.
    pub fn split_two(&self) -> (FeatureSignature, FeatureSignature) {
        let k = self.axis_width();
        let mut a = vec![Fixed::ZERO; self.values.len()];
        let mut b = vec![Fixed::ZERO; self.values.len()];
        for (i, &v) in self.values.iter().enumerate() {
            if i < k {
                // Coordinate mass: split raw bits, remainder to the first part.
                let bits = v.to_bits();
                let half = bits / 2;
                let rem = bits - half * 2;
                a[i] = Fixed::from_bits(half + rem);
                b[i] = Fixed::from_bits(half);
            } else {
                // Structural count: split the integer, remainder to the first part.
                let c = v.to_int();
                let half = c / 2;
                let rem = c - half * 2;
                a[i] = Fixed::from_int(half + rem);
                b[i] = Fixed::from_int(half);
            }
        }
        (
            FeatureSignature { values: a },
            FeatureSignature { values: b },
        )
    }
}

/// The derived etic descriptor (design Part 36): the nearest recognition template above the
/// reserved threshold, or none (generic), with the similarity surfaced so a weak match reads as
/// "weakly X". Recomputable from canonical state, so it never enters the state hash (Principle 10).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct EticDescriptor {
    /// The nearest recognition template, or `None` for a generic structural description.
    pub best_match: Option<TemplateId>,
    /// How well it matches (a weak match below the threshold still surfaces its similarity).
    pub similarity: Fixed,
}

/// The weighted-Tanimoto (weighted min-over-max) similarity of two feature vectors in canonical
/// feature order, a fixed-point polythetic match: no single feature is necessary or sufficient and
/// membership is graded (Needham and Rosch, made integer-exact). Both vectors' components are
/// non-negative here (intensities and counts), so the min and max are well-defined and the ratio
/// lies in `[0, ONE]`. Two all-zero vectors read as similarity zero (no shared mass). The sum is
/// in canonical order and fixed-point, so it is bit-identical across machines and thread counts,
/// and permuting the columns of both vectors and the weights consistently leaves it unchanged.
pub fn weighted_tanimoto(a: &[Fixed], b: &[Fixed], weights: &[Fixed]) -> Fixed {
    let n = a.len().min(b.len()).min(weights.len());
    let mut num = Fixed::ZERO;
    let mut den = Fixed::ZERO;
    for i in 0..n {
        let mn = a[i].min(b[i]);
        let mx = a[i].max(b[i]);
        num += weights[i].mul(mn);
        den += weights[i].mul(mx);
    }
    if den == Fixed::ZERO {
        Fixed::ZERO
    } else {
        num.div(den)
    }
}

/// Recognize a feature signature against a template library (design Part 36): the best-matching
/// template by [`weighted_tanimoto`], reported only when its similarity clears the reserved
/// threshold, else a generic descriptor (`best_match = None`) that still carries the similarity.
/// An empty library yields a generic descriptor with zero similarity and the engine runs. Templates
/// are iterated in ascending id order, so a similarity tie resolves to the lowest template id.
pub fn recognize(
    sig: &FeatureSignature,
    library: &TemplateLibrary,
    threshold: Fixed,
    weights: &[Fixed],
) -> EticDescriptor {
    let mut best: Option<(TemplateId, Fixed)> = None;
    for t in library.in_order() {
        // The similarity weights are the reserved inst.feature_weights supplied by the caller,
        // combined multiplicatively with the template's own per-feature weights, so a feature is
        // diagnostic only where both the world's weighting and the template's agree.
        let combined = combine_weights(weights, &t.weights);
        let s = weighted_tanimoto(&sig.values, &t.features, &combined);
        match best {
            Some((_, bs)) if s <= bs => {}
            _ => best = Some((t.id, s)),
        }
    }
    match best {
        Some((id, s)) if s >= threshold => EticDescriptor {
            best_match: Some(id),
            similarity: s,
        },
        Some((_, s)) => EticDescriptor {
            best_match: None,
            similarity: s,
        },
        None => EticDescriptor {
            best_match: None,
            similarity: Fixed::ZERO,
        },
    }
}

/// Combine the world's feature weights with a template's per-feature weights, multiplicatively and
/// per-position. A shorter side is treated as weight one past its end, so a template need not
/// re-specify the structural weights the world already sets.
fn combine_weights(world: &[Fixed], template: &[Fixed]) -> Vec<Fixed> {
    let n = world.len().max(template.len());
    (0..n)
        .map(|i| {
            let w = world.get(i).copied().unwrap_or(Fixed::ONE);
            let t = template.get(i).copied().unwrap_or(Fixed::ONE);
            w.mul(t)
        })
        .collect()
}

/// The fixed-point structural distance between two feature signatures: a weighted Euclidean over
/// the shared function substrate and feature signature (design Part 36), reusing the value.rs
/// Euclidean pattern. The weights are the reserved `inst.distance_weights` supplied by the caller.
/// The sum is in canonical feature order, so permuting the columns of both signatures and the
/// weights consistently leaves the distance bit-identical (the label-blind guarantee).
pub fn signature_distance(a: &FeatureSignature, b: &FeatureSignature, weights: &[Fixed]) -> Fixed {
    let n = a.values.len().min(b.values.len()).min(weights.len());
    // Accumulate the weighted sum of squared differences in i128 Q32.32 bit space. The structural
    // components are extensive counts (members, norms) that can reach the thousands, so a component
    // difference and its square overflow Fixed::mul's i64 narrowing (a diff >= ~46341 wraps the
    // square NEGATIVE, dragging the accumulator below zero so sqrt reads zero and two very different
    // large institutions read distance ~0). Each term w*d^2 is kept exact in i128 bits here, and the
    // final root is a widened integer sqrt, so a large difference reads a large (non-zero) distance.
    let mut acc: i128 = 0; // Q32.32 bits of the weighted sum of squares, non-negative
    for ((av, bv), w) in a.values[..n].iter().zip(&b.values[..n]).zip(&weights[..n]) {
        let d = (av.to_bits() as i128) - (bv.to_bits() as i128); // Q32.32 diff, exact in i128
        let d2 = (d * d) >> Fixed::FRAC_BITS; // Q32.32 bits of d^2, non-negative
        let term = ((w.to_bits() as i128) * d2) >> Fixed::FRAC_BITS; // Q32.32 bits of w*d^2
        acc = acc.saturating_add(term);
    }
    if acc <= 0 {
        return Fixed::ZERO;
    }
    // sqrt of a Q32.32 value in i128 bits: sqrt(acc / 2^F) * 2^F = isqrt(acc << F). Guard the shift
    // against u128 overflow (unreachable at realistic counts); a saturating radicand caps the
    // distance at Fixed::MAX rather than wrapping.
    let radicand = (acc as u128).checked_shl(Fixed::FRAC_BITS);
    let bits = match radicand {
        Some(r) => r.isqrt(),
        None => return Fixed::MAX,
    };
    Fixed::from_bits(bits.min(i64::MAX as u128) as i64)
}

/// The institution distance between two explicit institutions: [`signature_distance`] over their
/// extracted feature signatures. Two peoples that both crystallized something guild-like land
/// close even when their emic names differ, and a people whose academy doubles as a court lands
/// between clusters (the DiMaggio-Powell isomorphism measured rather than imposed).
pub fn institution_distance(a: &Institution, b: &Institution, weights: &[Fixed]) -> Fixed {
    signature_distance(&a.feature_signature(), &b.feature_signature(), weights)
}

// === (D) Norm-firing, the propensity accumulator, and the crystallization mechanism ===

/// The world facts a norm's condition reads, keyed by the decision layer's [`InputId`] registry.
/// A [`BTreeMap`] so any canonical walk is deterministic (R-CANON-WALK).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Conditions {
    facts: BTreeMap<InputId, Fixed>,
}

impl Conditions {
    /// An empty condition set (no facts known).
    pub fn new() -> Self {
        Conditions {
            facts: BTreeMap::new(),
        }
    }

    /// A condition set from fact pairs.
    pub fn with(pairs: impl IntoIterator<Item = (InputId, Fixed)>) -> Self {
        Conditions {
            facts: pairs.into_iter().collect(),
        }
    }

    /// Set a world fact.
    pub fn set(&mut self, input: InputId, value: Fixed) {
        self.facts.insert(input, value);
    }

    /// The current value of a world fact, if known.
    pub fn get(&self, input: InputId) -> Option<Fixed> {
        self.facts.get(&input).copied()
    }
}

/// Whether a norm fires: whether its [`ConditionExpr`] is satisfied by the current world facts. An
/// atom whose input is not present in the conditions reads as unsatisfied (a norm cannot fire on a
/// fact the world does not supply), so an empty conjunction fires unconditionally while any missing
/// required fact holds the norm dormant. Independent of the deontic: a strategy, a norm, and a rule
/// all fire by their condition.
pub fn norm_fires(norm: &Norm, conds: &Conditions) -> bool {
    norm.condition.all_of.iter().all(|atom| {
        conds
            .get(atom.input)
            .map(|v| atom.predicate.holds(v, atom.threshold))
            .unwrap_or(false)
    })
}

/// The decision-propensity accumulator (design Part 54, Part 36): a recurring coordination pattern
/// builds propensity as its norms fire, and an undertaking becomes ripe when the accumulation
/// crosses the reserved `tier.decision_propensity` threshold. Saturating, so a long-running pattern
/// cannot overflow.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DecisionPropensity {
    accumulated: Fixed,
}

impl DecisionPropensity {
    /// A fresh accumulator at zero.
    pub fn new() -> Self {
        DecisionPropensity {
            accumulated: Fixed::ZERO,
        }
    }

    /// Add to the accumulation (the reserved `inst.crystallization_rate` per firing observation),
    /// saturating.
    pub fn accumulate(&mut self, amount: Fixed) {
        self.accumulated = self.accumulated.saturating_add(amount);
    }

    /// The current accumulation.
    pub fn level(&self) -> Fixed {
        self.accumulated
    }

    /// Whether the accumulation has crossed the reserved threshold.
    pub fn crossed(&self, threshold: Fixed) -> bool {
        self.accumulated >= threshold
    }

    /// Reset to zero (after an undertaking is emitted).
    pub fn reset(&mut self) {
        self.accumulated = Fixed::ZERO;
    }
}

/// Whether a collective undertaking should emit now: at least one norm fires under the conditions
/// AND the accumulated propensity has crossed the reserved `tier.decision_propensity` threshold
/// (design Part 54, the aggregate collective-undertaking gate). The threshold is supplied by the
/// caller from the manifest, never fabricated here.
pub fn emit_undertaking(
    propensity: &DecisionPropensity,
    threshold: Fixed,
    norms: &[Norm],
    conds: &Conditions,
) -> bool {
    propensity.crossed(threshold) && norms.iter().any(|n| norm_fires(n, conds))
}

/// The compact pool-tier institution (design Part 36): a feature vector and a count rather than
/// explicit roles and members, enough to compute descriptors and distances at the pool level.
/// Promotion materializes an explicit [`Institution`] whose feature signature reproduces this
/// vector, which R-TIER-CONSIST carries as a declared conserved projection. The coordinate mass
/// and the feature signature are extensive (they sum on a merge, partition on a split, and move
/// unchanged on a promotion), so total feature mass and total legitimacy mass are conserved.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AggregateInstitution {
    /// The extensive coordinate mass over the shared substrate (the mirror of the signature's
    /// function-axis columns). The intensive blend a descriptor reads is this divided by count.
    pub coordinates: FunctionVec,
    /// The compact extensive feature signature promotion must reproduce.
    pub feature_signature: FeatureSignature,
    /// The extensive aggregate legitimacy mass (intensive legitimacy is this divided by count).
    pub legitimacy: Fixed,
    /// The number of coordinated members the aggregate summarizes.
    pub count: u32,
}

impl AggregateInstitution {
    /// Build a compact aggregate from an extensive coordinate mass, the structural counts, and the
    /// legitimacy mass. The member count is taken from the members structural count.
    pub fn from_parts(
        coordinates: FunctionVec,
        roles: i32,
        members: i32,
        norms: i32,
        rules: i32,
        obligations: i32,
        legitimacy: Fixed,
    ) -> Self {
        let feature_signature = FeatureSignature::build(
            &coordinates.intensity,
            roles,
            members,
            norms,
            rules,
            obligations,
        );
        AggregateInstitution {
            coordinates,
            feature_signature,
            legitimacy,
            count: members.max(0) as u32,
        }
    }

    /// Rebuild an aggregate from a (possibly merged or split) feature signature and legitimacy,
    /// re-deriving the coordinate mirror and the member count from the signature. This is the
    /// canonical constructor after a structural operation on the signature.
    pub fn from_signature(signature: FeatureSignature, legitimacy: Fixed) -> Self {
        let coordinates =
            FunctionVec::from_intensities(signature.coordinate_mass().iter().copied());
        let count = signature.member_count();
        AggregateInstitution {
            coordinates,
            feature_signature: signature,
            legitimacy,
            count,
        }
    }

    /// Merge two aggregates into one (design Part 36's aggregate-tier merge): sum the feature
    /// signatures componentwise and the legitimacy masses, so total feature mass and legitimacy
    /// mass are conserved exactly across the operation.
    pub fn merge(&self, other: &AggregateInstitution) -> AggregateInstitution {
        AggregateInstitution::from_signature(
            self.feature_signature.add(&other.feature_signature),
            self.legitimacy + other.legitimacy,
        )
    }

    /// Split one aggregate into two that recombine to it exactly: the feature signature partitions
    /// componentwise ([`FeatureSignature::split_two`]) and the legitimacy mass splits by raw bits
    /// with the remainder to the first part, so total feature mass and legitimacy are conserved.
    pub fn split_two(&self) -> (AggregateInstitution, AggregateInstitution) {
        let (sa, sb) = self.feature_signature.split_two();
        let bits = self.legitimacy.to_bits();
        let half = bits / 2;
        let rem = bits - half * 2;
        let la = Fixed::from_bits(half + rem);
        let lb = Fixed::from_bits(half);
        (
            AggregateInstitution::from_signature(sa, la),
            AggregateInstitution::from_signature(sb, lb),
        )
    }

    /// Fold an explicit institution back into the compact aggregate form (the demotion the tier
    /// boundary needs): its feature signature and legitimacy carry over unchanged, so total feature
    /// mass and legitimacy mass are conserved across the crossing exactly as promotion conserves
    /// them the other way.
    pub fn from_institution(inst: &Institution) -> AggregateInstitution {
        AggregateInstitution::from_signature(inst.feature_signature(), inst.legitimacy)
    }

    /// The intensive coordinate blend (coordinate mass divided by count), for a descriptor gloss.
    pub fn coordinates_intensity(&self) -> FunctionVec {
        intensive_blend(&self.coordinates.intensity, self.count as i32)
    }

    /// Materialize an explicit [`Institution`] whose feature signature reproduces this compact
    /// vector exactly (the promotion the tier-consistency mechanism audits). The coordinate mass is
    /// copied verbatim and exactly the counted number of generic roles, members, and norms are
    /// generated (with the first `rule_count` carrying a sanction and the first `obligation_count`
    /// a deontic), so [`Institution::feature_signature`] recomputes to this signature bit for bit.
    /// The generated members are placeholder ids; wiring the real member ids across the tier
    /// boundary is part of the named live-detector follow-on.
    pub fn materialize(&self, id: InstId, founded: EventId) -> Institution {
        let counts = self.feature_signature.structural_counts();
        let [role_count, member_count, norm_count, rule_count, obligation_count] = counts;
        let roles: Vec<Role> = (0..role_count.max(0))
            .map(|i| Role {
                id: RoleId(i as u32),
                powers: Vec::new(),
                duties: Vec::new(),
            })
            .collect();
        let members: Vec<StableId> = (0..member_count.max(0))
            .map(|i| StableId(i as u64))
            .collect();
        let rules: Vec<Norm> = (0..norm_count.max(0))
            .map(|i| Norm {
                attribute: AttributeSel::everyone(),
                deontic: if i < obligation_count {
                    Some(Deontic::Must)
                } else {
                    None
                },
                aim: ActionId(0),
                condition: ConditionExpr::always(),
                // A rule is deontic AND sanction: only give a sanction to a norm that also gets a
                // deontic, so a sanctioned norm materializes as a well-formed Rule (not an
                // O-without-D that would recount as a strategy). Under the valid invariant
                // rule_count <= obligation_count this is identical to `i < rule_count`; the extra
                // guard keeps the round-trip exact even on a malformed input signature.
                or_else: if i < rule_count && i < obligation_count {
                    Some(ActionId(0))
                } else {
                    None
                },
                enforcement: Fixed::ZERO,
                provenance: founded,
            })
            .collect();
        let coordinates =
            FunctionVec::from_intensities(self.feature_signature.coordinate_mass().iter().copied());
        Institution {
            id,
            roles,
            rules,
            coordinates,
            legitimacy: self.legitimacy,
            resources: Vec::new(),
            members,
            founded,
            parent: None,
            descriptor: EticDescriptor::default(),
        }
    }

    /// Fold the aggregate into a state hash in canonical order (its feature signature, legitimacy,
    /// and count; no descriptor is stored on the compact form).
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u64(self.feature_signature.values.len() as u64);
        for &v in &self.feature_signature.values {
            h.write_fixed(v);
        }
        h.write_fixed(self.legitimacy);
        h.write_u32(self.count);
    }
}

/// The reserved crystallization calibration (design Part 36): the propensity threshold a recurring
/// coordination pattern must cross to crystallize (the `inst.crystallization_threshold` /
/// `tier.decision_propensity` pair) and the per-observation accumulation rate
/// (`inst.crystallization_rate`). Supplied by the caller from the manifest, never fabricated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CrystallizationParams {
    /// The propensity threshold at which the pattern crystallizes.
    pub threshold: Fixed,
    /// The propensity added per firing observation.
    pub rate: Fixed,
}

/// One observation in a synthetic coordination-pattern stream (the mechanism's test input; the
/// live wiring to the real decision layer is a NAMED FOLLOW-ON). It records the function-space
/// intensities of a coordinated act, who coordinated, the norms it instantiated, the legitimacy it
/// conferred, and the world facts under which it occurred.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CoordinationObservation {
    /// The function-space intensities of this coordinated act (one per substrate axis).
    pub coordinates: FunctionVec,
    /// Who coordinated, by stable id.
    pub members: Vec<StableId>,
    /// The norms this act instantiated.
    pub norms: Vec<Norm>,
    /// The legitimacy this act conferred.
    pub legitimacy: Fixed,
    /// The world facts under which it occurred (for the norm-firing check).
    pub conditions: Conditions,
}

/// Run the crystallization mechanism over a synthetic coordination-pattern stream (design Part
/// 36). Every observation whose norms fire under its conditions adds `params.rate` to the
/// propensity and folds its coordinate mass, legitimacy, members, and norms into the accumulating
/// aggregate by canonical id-ordered aggregation. If the propensity crosses `params.threshold`,
/// the pattern crystallizes into an [`AggregateInstitution`]; otherwise it returns `None` (the
/// pattern is not yet stable enough to be an institution). The mechanism carries NO race parameter:
/// a race enters ONLY through the coordinate DATA the stream's observations carry, so the identical
/// call on two races' streams diverges purely from their function-axis intensities.
pub fn crystallize(
    stream: &[CoordinationObservation],
    substrate_len: usize,
    params: &CrystallizationParams,
) -> Option<AggregateInstitution> {
    let mut prop = DecisionPropensity::new();
    let mut coord_mass = vec![Fixed::ZERO; substrate_len];
    let mut legitimacy = Fixed::ZERO;
    let mut members: BTreeSet<StableId> = BTreeSet::new();
    let mut role_ids: BTreeSet<RoleId> = BTreeSet::new();
    // Distinct norms keyed by their canonical sort key, with the (has-sanction, has-deontic) flags.
    let mut norms: BTreeMap<NormKey, (bool, bool)> = BTreeMap::new();
    let mut fired_any = false;

    for obs in stream {
        if !obs.norms.iter().any(|n| norm_fires(n, &obs.conditions)) {
            continue;
        }
        fired_any = true;
        prop.accumulate(params.rate);
        legitimacy = legitimacy.saturating_add(obs.legitimacy);
        for (i, c) in obs
            .coordinates
            .intensity
            .iter()
            .enumerate()
            .take(substrate_len)
        {
            coord_mass[i] = coord_mass[i].saturating_add(*c);
        }
        for m in &obs.members {
            members.insert(*m);
        }
        for n in &obs.norms {
            for r in &n.attribute.roles {
                role_ids.insert(*r);
            }
            // The dedup keys on the FULL norm identity (sort_key now folds attribute and condition),
            // so norms that differ only in binding or firing condition count separately. The rule
            // flag is deontic AND sanction (A.D.I.C.O), not a bare sanction (matching Norm::norm_type
            // and feature_signature), so an ill-formed O-without-D is not miscounted as a rule.
            let is_rule = n.deontic.is_some() && n.or_else.is_some();
            norms.insert(n.sort_key(), (is_rule, n.deontic.is_some()));
        }
    }

    if !fired_any || !prop.crossed(params.threshold) {
        return None;
    }

    let rule_count = norms.values().filter(|(is_rule, _)| *is_rule).count() as i32;
    let obligation_count = norms.values().filter(|(_, is_oblig)| *is_oblig).count() as i32;
    Some(AggregateInstitution::from_parts(
        FunctionVec::from_intensities(coord_mass),
        role_ids.len() as i32,
        members.len() as i32,
        norms.len() as i32,
        rule_count,
        obligation_count,
        legitimacy,
    ))
}

/// The canonical crystallization order for a set of ripe patterns (design Part 36's determinism
/// pin). Each candidate is a `(primary_key, secondary_key)` pair. The order is PRIMARY by
/// `primary_key` ascending (the id-ordered canonical key). A genuine tie (two candidates with an
/// exact-equal primary key) is broken by the [`Phase::CRYSTALLIZE`] draw stream, folding the master
/// seed, the locus, the tick, and the phase, with the candidate's secondary key as the RNG locus,
/// and the secondary key itself as a final total-order fallback. The RNG is consulted ONLY for an
/// exact primary tie, so the order is a pure function of canonical state (Principle 3) and is
/// independent of the order the candidates were supplied in.
pub fn crystallization_order(
    candidates: &[(u64, u64)],
    master_seed: u64,
    locus: u64,
    tick: u64,
) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..candidates.len()).collect();
    idx.sort_by(|&i, &j| {
        let (pi, si) = candidates[i];
        let (pj, sj) = candidates[j];
        pi.cmp(&pj)
            .then_with(|| {
                let di = DrawKey::pair(locus, si, tick, Phase::CRYSTALLIZE)
                    .rng(master_seed)
                    .at(0);
                let dj = DrawKey::pair(locus, sj, tick, Phase::CRYSTALLIZE)
                    .rng(master_seed)
                    .at(0);
                di.cmp(&dj)
            })
            .then_with(|| si.cmp(&sj))
            .then_with(|| i.cmp(&j))
    });
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    /// A uniform-weight fixture over a signature width, a TEST FIXTURE and never an owner value:
    /// it exercises the metrics while the manifest path stays fail-loud until the reserved
    /// inst.feature_weights / inst.distance_weights are set.
    fn unit_weights(width: usize) -> Vec<Fixed> {
        vec![Fixed::ONE; width]
    }

    /// A norm that always fires, of a chosen ADICO shape.
    fn norm(deontic: Option<Deontic>, or_else: Option<SanctionRef>) -> Norm {
        Norm {
            attribute: AttributeSel::everyone(),
            deontic,
            aim: ActionId(0),
            condition: ConditionExpr::always(),
            or_else,
            enforcement: Fixed::ZERO,
            provenance: EventId(0),
        }
    }

    /// Build an explicit institution directly with chosen coordinates and structural counts (all
    /// generated norms firing), a test helper mirroring what materialize does.
    fn inst(id: u32, coords: Vec<Fixed>, members: u32, norms: u32) -> Institution {
        AggregateInstitution::from_parts(
            FunctionVec::from_intensities(coords),
            1,
            members as i32,
            norms as i32,
            0,
            0,
            Fixed::ZERO,
        )
        .materialize(InstId(id), EventId(0))
    }

    #[test]
    fn large_different_institutions_have_a_large_nonzero_signature_distance() {
        // Regression (audit defect 7): two institutions with large, very different extensive counts
        // (members, norms) read a LARGE (non-zero) distance. The old i64 squaring wrapped the squared
        // component difference NEGATIVE for a diff >= ~46341, dragging the accumulator below zero so
        // sqrt read zero and two very different large institutions collapsed to distance ~0.
        let a = FeatureSignature::build(&[Fixed::ZERO], 1, 50_000, 40_000, 0, 0);
        let b = FeatureSignature::build(&[Fixed::ZERO], 1, 100, 80, 0, 0);
        let w = unit_weights(a.values.len());
        let d = signature_distance(&a, &b, &w);
        assert!(
            d > Fixed::from_int(10_000),
            "very different large institutions are far apart, not distance ~0 (got {d:?})"
        );
        // Identical signatures are exactly distance zero (the fix does not perturb the zero case).
        assert_eq!(signature_distance(&a, &a, &w), Fixed::ZERO);
    }

    #[test]
    fn crystallize_does_not_dedup_norms_differing_only_in_attribute_or_condition() {
        // Regression (audit defect 8): two norms that differ ONLY in whom they bind (attribute) or
        // ONLY in their firing condition are distinct norms, not one. The old dedup keyed on a
        // sort_key that omitted attribute and condition, so such norms collided and were undercounted.
        let params = CrystallizationParams {
            threshold: fx(1, 4),
            rate: fx(1, 2),
        };
        let base = || Norm {
            attribute: AttributeSel::everyone(),
            deontic: Some(Deontic::Must),
            aim: ActionId(0),
            condition: ConditionExpr::always(),
            or_else: None,
            enforcement: Fixed::ZERO,
            provenance: EventId(0),
        };
        // Two norms differing only in attribute (everyone vs a role binding).
        let n_all = base();
        let n_role = Norm {
            attribute: AttributeSel::role(RoleId(0)),
            ..base()
        };
        let obs = CoordinationObservation {
            coordinates: FunctionVec::from_intensities(vec![fx(1, 2)]),
            members: vec![StableId(1)],
            norms: vec![n_all.clone(), n_role],
            legitimacy: fx(1, 10),
            conditions: Conditions::new(),
        };
        let agg = crystallize(&[obs.clone(), obs], 1, &params).expect("crystallizes");
        // structural_counts: [roles, members, norms, rules, obligations]; norms is index 2.
        assert_eq!(
            agg.feature_signature.structural_counts()[2],
            2,
            "two attribute-distinct norms count as two, not deduped into one"
        );

        // Two norms differing only in condition (unconditional vs a single-atom condition).
        let n_cond = Norm {
            condition: ConditionExpr::atom(InputId(0), Predicate::AtLeast, fx(1, 2)),
            ..base()
        };
        let obs2 = CoordinationObservation {
            coordinates: FunctionVec::from_intensities(vec![fx(1, 2)]),
            members: vec![StableId(1)],
            norms: vec![n_all, n_cond],
            legitimacy: fx(1, 10),
            // The conditioned norm must fire too, so supply the fact its atom reads.
            conditions: Conditions::with([(InputId(0), Fixed::ONE)]),
        };
        let agg2 = crystallize(&[obs2.clone(), obs2], 1, &params).expect("crystallizes");
        assert_eq!(
            agg2.feature_signature.structural_counts()[2],
            2,
            "two condition-distinct norms count as two, not deduped into one"
        );
    }

    // (1) THE NON-STEERING TEST.
    #[test]
    fn divergence_comes_from_axis_data_not_a_code_branch() {
        // A shared 6-axis substrate: the 5 human-analogue dev-seed axes plus one exotic axis,
        // brood-tending, that race B carries and race A does not.
        let mut substrate = FunctionRegistry::dev_seed();
        let brood = substrate.insert(FunctionAxisDef {
            id: FunctionAxisId(5),
            name: "brood_tending".to_string(),
            description: "a hive race's brood-tending coordination".to_string(),
        });
        assert_eq!(brood, 5);
        let k = substrate.len();
        let params = CrystallizationParams {
            threshold: fx(1, 2),
            rate: fx(1, 4),
        };

        // A firing norm and a full-firing observation over a chosen coordinate blend.
        let obs = |coords: Vec<Fixed>| CoordinationObservation {
            coordinates: FunctionVec::from_intensities(coords),
            members: vec![StableId(1), StableId(2)],
            norms: vec![norm(Some(Deontic::Must), None)],
            legitimacy: fx(1, 10),
            conditions: Conditions::new(),
        };

        // Race A's streams load only axes 0..4; brood-tending (axis 5) stays zero. Three distinct
        // A institutions in different regions of A's reachable subspace.
        let a_streams = [
            vec![
                fx(8, 10),
                fx(1, 10),
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ZERO,
            ],
            vec![
                fx(1, 10),
                fx(8, 10),
                fx(1, 10),
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ZERO,
            ],
            vec![
                Fixed::ZERO,
                Fixed::ZERO,
                fx(2, 10),
                fx(7, 10),
                fx(1, 10),
                Fixed::ZERO,
            ],
        ];
        // The SAME mechanism (crystallize) on each stream, no race parameter.
        let a_insts: Vec<Institution> = a_streams
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let stream = vec![obs(c.clone()), obs(c.clone())];
                let agg = crystallize(&stream, k, &params).expect("A pattern crystallizes");
                agg.materialize(InstId(i as u32), EventId(0))
            })
            .collect();

        // Race B's stream loads the exotic brood-tending axis heavily: a region A cannot reach.
        let b_stream = vec![
            obs(vec![
                fx(1, 10),
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ZERO,
                fx(1, 10),
                fx(9, 10),
            ]),
            obs(vec![
                fx(1, 10),
                Fixed::ZERO,
                Fixed::ZERO,
                Fixed::ZERO,
                fx(1, 10),
                fx(9, 10),
            ]),
        ];
        let b_agg = crystallize(&b_stream, k, &params).expect("B pattern crystallizes");
        let b_inst = b_agg.materialize(InstId(100), EventId(0));

        // B occupies a function-space region A cannot reach: every A institution has zero mass on
        // the brood-tending axis, B has positive mass.
        for a in &a_insts {
            assert_eq!(
                a.coordinates.intensity[brood],
                Fixed::ZERO,
                "race A never loads the exotic axis"
            );
        }
        assert!(
            b_inst.coordinates.intensity[brood] > Fixed::ZERO,
            "race B occupies the exotic axis"
        );

        let w = unit_weights(k + STRUCTURAL_FEATURES);
        // Every within-A distance.
        let mut max_within_a = Fixed::ZERO;
        for i in 0..a_insts.len() {
            for j in (i + 1)..a_insts.len() {
                let d = institution_distance(&a_insts[i], &a_insts[j], &w);
                if d > max_within_a {
                    max_within_a = d;
                }
            }
        }
        // The A-to-B distance exceeds every within-A distance: the divergence is from the axis DATA
        // (which axes the race's stream loads), not a per-race code branch.
        for a in &a_insts {
            let cross = institution_distance(a, &b_inst, &w);
            assert!(
                cross > max_within_a,
                "cross-race distance {cross:?} must exceed the largest within-A distance {max_within_a:?}"
            );
        }
    }

    // (2) GLOSS-BLINDNESS.
    #[test]
    fn permuting_axis_labels_and_columns_leaves_metrics_bit_identical() {
        let k = 5usize;
        let a = inst(
            0,
            vec![fx(8, 10), fx(1, 10), fx(3, 10), Fixed::ZERO, fx(5, 10)],
            3,
            2,
        );
        let b = inst(
            1,
            vec![fx(1, 10), fx(9, 10), Fixed::ZERO, fx(4, 10), fx(2, 10)],
            3,
            2,
        );
        let mut lib = TemplateLibrary::new();
        let tmpl_features = vec![
            fx(7, 10),
            fx(1, 10),
            fx(3, 10),
            Fixed::ZERO,
            fx(5, 10),
            Fixed::from_int(1),
            Fixed::from_int(3),
            Fixed::from_int(2),
            Fixed::ZERO,
            Fixed::from_int(2),
        ];
        let tmpl_weights = vec![Fixed::ONE; k + STRUCTURAL_FEATURES];
        lib.insert(RecognitionTemplate {
            id: TemplateId(0),
            features: tmpl_features.clone(),
            weights: tmpl_weights.clone(),
        });
        let w = unit_weights(k + STRUCTURAL_FEATURES);
        let threshold = Fixed::ZERO;

        let sim_a = recognize(&a.feature_signature(), &lib, threshold, &w).similarity;
        let dist = institution_distance(&a, &b, &w);

        // A permutation of the k function-axis columns, applied consistently to both institutions'
        // coordinates, the template's first-k features and weights, and the distance weights.
        let perm = [3usize, 0, 4, 1, 2];
        let permute_first_k = |v: &[Fixed]| -> Vec<Fixed> {
            let mut out = v.to_vec();
            for (dst, &src) in perm.iter().enumerate() {
                out[dst] = v[src];
            }
            out
        };
        let a2 = Institution {
            coordinates: FunctionVec::from_intensities(permute_first_k(&a.coordinates.intensity)),
            ..a.clone()
        };
        let b2 = Institution {
            coordinates: FunctionVec::from_intensities(permute_first_k(&b.coordinates.intensity)),
            ..b.clone()
        };
        let mut features2 = permute_first_k(&tmpl_features[..k]);
        features2.extend_from_slice(&tmpl_features[k..]);
        let mut lib2 = TemplateLibrary::new();
        lib2.insert(RecognitionTemplate {
            id: TemplateId(0),
            features: features2,
            weights: tmpl_weights.clone(),
        });

        let sim_a2 = recognize(&a2.feature_signature(), &lib2, threshold, &w).similarity;
        let dist2 = institution_distance(&a2, &b2, &w);
        assert_eq!(sim_a, sim_a2, "similarity is label-blind");
        assert_eq!(dist, dist2, "distance is label-blind");
    }

    // (3) EMPTY-LIBRARY.
    #[test]
    fn an_empty_library_reads_every_institution_as_generic_and_the_engine_runs() {
        let registry = FunctionRegistry::dev_seed();
        let lib = TemplateLibrary::new();
        assert!(lib.is_empty());
        let a = inst(
            0,
            vec![fx(1, 10), fx(8, 10), fx(1, 10), Fixed::ZERO, Fixed::ZERO],
            4,
            3,
        );
        let d = a.descriptor_from(&lib, fx(1, 2), &unit_weights(5 + STRUCTURAL_FEATURES));
        assert_eq!(d.best_match, None, "no template matches an empty library");
        assert_eq!(d.similarity, Fixed::ZERO);
        // The engine still produces a legible generic description from the dominant axis.
        let gloss = a.generic_gloss(&registry);
        assert!(
            gloss.contains("sacred"),
            "dominant axis names the generic gloss: {gloss}"
        );
    }

    // (4) NORM-TYPE-EMERGENCE.
    #[test]
    fn norm_type_reads_purely_from_field_presence() {
        let strategy = norm(None, None);
        let a_norm = norm(Some(Deontic::Must), None);
        let rule = norm(Some(Deontic::MustNot), Some(ActionId(7)));
        assert_eq!(strategy.norm_type(), NormType::Strategy);
        assert_eq!(a_norm.norm_type(), NormType::Norm);
        assert_eq!(rule.norm_type(), NormType::Rule);
        // A sanction without a deontic (an ill-formed O without a D) reads as a strategy: the
        // deontic is the operative component, and there is no stored tag to consult.
        let dangling = norm(None, Some(ActionId(7)));
        assert_eq!(dangling.norm_type(), NormType::Strategy);
    }

    // (6) ORDER-INDEPENDENCE.
    #[test]
    fn feature_extraction_and_recognition_are_insertion_order_independent() {
        let coords = vec![fx(6, 10), fx(2, 10), fx(1, 10), Fixed::ZERO, fx(3, 10)];
        let base = Institution {
            id: InstId(0),
            roles: vec![
                Role {
                    id: RoleId(2),
                    powers: vec![ActionId(1)],
                    duties: vec![],
                },
                Role {
                    id: RoleId(0),
                    powers: vec![],
                    duties: vec![ActionId(3)],
                },
                Role {
                    id: RoleId(1),
                    powers: vec![],
                    duties: vec![],
                },
            ],
            rules: vec![
                norm(Some(Deontic::Must), Some(ActionId(1))),
                norm(None, None),
                norm(Some(Deontic::May), None),
            ],
            coordinates: FunctionVec::from_intensities(coords.clone()),
            legitimacy: fx(1, 2),
            resources: vec![],
            members: vec![StableId(5), StableId(1), StableId(9)],
            founded: EventId(0),
            parent: None,
            descriptor: EticDescriptor::default(),
        };
        // A version with roles, members, and norms inserted in a different order.
        let mut shuffled = base.clone();
        shuffled.roles.reverse();
        shuffled.members.reverse();
        shuffled.rules.reverse();
        assert_eq!(
            base.feature_signature(),
            shuffled.feature_signature(),
            "feature extraction is order-independent"
        );
        let mut lib = TemplateLibrary::new();
        lib.insert(RecognitionTemplate {
            id: TemplateId(0),
            features: base.feature_signature().values.clone(),
            weights: vec![Fixed::ONE; base.feature_signature().values.len()],
        });
        let w = unit_weights(base.feature_signature().values.len());
        assert_eq!(
            base.descriptor_from(&lib, fx(1, 2), &w),
            shuffled.descriptor_from(&lib, fx(1, 2), &w),
            "recognition is order-independent"
        );
    }

    // (7) DETERMINISM REPLAY (the crystallization tie-break and descriptor recompute).
    #[test]
    fn crystallization_order_is_deterministic_and_input_order_independent() {
        // Two candidates share an exact primary key (a genuine tie): the CRYSTALLIZE draw decides,
        // and distinct-primary candidates never touch the RNG.
        let candidates = vec![(10u64, 1u64), (10u64, 2u64), (3u64, 9u64)];
        let seed = 0xC0FFEE;
        let order1 = crystallization_order(&candidates, seed, 42, 7);
        // Recompute: identical (a pure function of canonical state).
        let order2 = crystallization_order(&candidates, seed, 42, 7);
        assert_eq!(order1, order2, "the order is a pure function of state");
        // Distinct-primary candidate 3 sorts first regardless of the RNG.
        assert_eq!(order1[0], 2, "the lowest primary key sorts first");
        // Supplying the tied candidates in the opposite order yields the same canonical order (the
        // two tied entries keep the same relative order by their secondary keys / the draw).
        let swapped = vec![(10u64, 2u64), (10u64, 1u64), (3u64, 9u64)];
        let order_swapped = crystallization_order(&swapped, seed, 42, 7);
        // Map both orders back to the (primary, secondary) pairs they select, which must match.
        let pairs1: Vec<(u64, u64)> = order1.iter().map(|&i| candidates[i]).collect();
        let pairs_swapped: Vec<(u64, u64)> = order_swapped.iter().map(|&i| swapped[i]).collect();
        assert_eq!(
            pairs1, pairs_swapped,
            "the order is input-order independent"
        );
    }

    #[test]
    fn descriptor_recomputed_on_demand_matches() {
        let a = inst(
            0,
            vec![fx(2, 10), fx(7, 10), fx(1, 10), Fixed::ZERO, fx(1, 10)],
            3,
            2,
        );
        let mut lib = TemplateLibrary::new();
        lib.insert(RecognitionTemplate {
            id: TemplateId(0),
            features: a.feature_signature().values.clone(),
            weights: vec![Fixed::ONE; a.feature_signature().values.len()],
        });
        let w = unit_weights(a.feature_signature().values.len());
        let d1 = a.descriptor_from(&lib, fx(1, 2), &w);
        let d2 = a.descriptor_from(&lib, fx(1, 2), &w);
        assert_eq!(d1, d2, "the descriptor is recomputable and stable");
        assert_eq!(
            d1.best_match,
            Some(TemplateId(0)),
            "an exact template is recognized"
        );
    }

    // Supporting: materialize reproduces the compact signature exactly (the promotion audit).
    #[test]
    fn materialize_reproduces_the_compact_signature() {
        let agg = AggregateInstitution::from_parts(
            FunctionVec::from_intensities(vec![fx(4, 10), fx(6, 10), Fixed::ZERO]),
            2,
            5,
            4,
            1,
            3,
            fx(3, 4),
        );
        let inst = agg.materialize(InstId(0), EventId(0));
        assert_eq!(
            inst.feature_signature(),
            agg.feature_signature,
            "the materialized institution reproduces the pool's compact vector"
        );
    }

    // Supporting: merge and split conserve feature mass and legitimacy exactly.
    #[test]
    fn merge_and_split_conserve_feature_and_legitimacy_mass() {
        let a = AggregateInstitution::from_parts(
            FunctionVec::from_intensities(vec![fx(4, 10), fx(6, 10), Fixed::ZERO]),
            2,
            5,
            4,
            1,
            3,
            fx(3, 4),
        );
        let b = AggregateInstitution::from_parts(
            FunctionVec::from_intensities(vec![fx(1, 10), fx(2, 10), fx(5, 10)]),
            1,
            3,
            2,
            0,
            1,
            fx(1, 4),
        );
        let before_feature = a.feature_signature.mass_bits() + b.feature_signature.mass_bits();
        let before_legit = a.legitimacy.to_bits() as i128 + b.legitimacy.to_bits() as i128;
        let merged = a.merge(&b);
        assert_eq!(
            merged.feature_signature.mass_bits(),
            before_feature,
            "merge conserves feature mass"
        );
        assert_eq!(
            merged.legitimacy.to_bits() as i128,
            before_legit,
            "merge conserves legitimacy mass"
        );
        let (sa, sb) = merged.split_two();
        assert_eq!(
            sa.feature_signature.mass_bits() + sb.feature_signature.mass_bits(),
            before_feature,
            "split conserves feature mass"
        );
        assert_eq!(
            sa.legitimacy.to_bits() as i128 + sb.legitimacy.to_bits() as i128,
            before_legit,
            "split conserves legitimacy mass"
        );
    }

    #[test]
    fn norm_fires_reads_conditions_off_the_input_registry() {
        let hungry = InputId(0);
        let n = Norm {
            attribute: AttributeSel::everyone(),
            deontic: Some(Deontic::Must),
            aim: ActionId(0),
            condition: ConditionExpr::atom(hungry, Predicate::AtLeast, fx(1, 2)),
            or_else: None,
            enforcement: Fixed::ZERO,
            provenance: EventId(0),
        };
        // The fact is above the threshold: the norm fires.
        assert!(norm_fires(&n, &Conditions::with([(hungry, fx(7, 10))])));
        // Below the threshold: it does not.
        assert!(!norm_fires(&n, &Conditions::with([(hungry, fx(1, 10))])));
        // The fact is absent: a norm cannot fire on a fact the world does not supply.
        assert!(!norm_fires(&n, &Conditions::new()));
        // An unconditional norm always fires.
        let uncond = norm(Some(Deontic::May), None);
        assert!(norm_fires(&uncond, &Conditions::new()));
    }

    #[test]
    fn emit_undertaking_gates_on_both_firing_and_the_threshold() {
        let mut prop = DecisionPropensity::new();
        let norms = vec![norm(Some(Deontic::Must), None)];
        let conds = Conditions::new();
        let threshold = fx(1, 2);
        // Below the threshold, no undertaking even though the norm fires.
        prop.accumulate(fx(1, 4));
        assert!(!emit_undertaking(&prop, threshold, &norms, &conds));
        // Cross the threshold: it emits.
        prop.accumulate(fx(1, 4));
        assert!(emit_undertaking(&prop, threshold, &norms, &conds));
        // A crossed threshold with no firing norm still does not emit.
        let silent = vec![Norm {
            condition: ConditionExpr::atom(InputId(0), Predicate::AtLeast, Fixed::ONE),
            ..norm(Some(Deontic::Must), None)
        }];
        assert!(!emit_undertaking(&prop, threshold, &silent, &conds));
    }

    #[test]
    fn a_below_threshold_pattern_does_not_crystallize() {
        // A single firing observation with a rate below the threshold does not crystallize.
        let obs = CoordinationObservation {
            coordinates: FunctionVec::zeros(3),
            members: vec![StableId(1)],
            norms: vec![norm(Some(Deontic::Must), None)],
            legitimacy: Fixed::ZERO,
            conditions: Conditions::new(),
        };
        let params = CrystallizationParams {
            threshold: Fixed::ONE,
            rate: fx(1, 10),
        };
        assert!(crystallize(&[obs], 3, &params).is_none());
    }

    #[test]
    fn a_stream_whose_norms_never_fire_does_not_crystallize() {
        let obs = CoordinationObservation {
            coordinates: FunctionVec::zeros(3),
            members: vec![StableId(1)],
            norms: vec![Norm {
                condition: ConditionExpr::atom(InputId(0), Predicate::AtLeast, Fixed::ONE),
                ..norm(Some(Deontic::Must), None)
            }],
            legitimacy: Fixed::ZERO,
            conditions: Conditions::new(),
        };
        let params = CrystallizationParams {
            threshold: fx(1, 100),
            rate: fx(1, 2),
        };
        assert!(crystallize(&[obs.clone(), obs], 3, &params).is_none());
    }
}
