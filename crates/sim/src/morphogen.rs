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

//! The developmental-growth kernel (emergent-anatomy Step 2): a genome grows a body's STRUCTURE from a
//! seed under bounded, deterministic recursion, and the Step-1 function-law dispatch
//! ([`civsim_compose::derive_capabilities`]) reads each grown segment's function from its own geometry
//! and material. No catalog supplies the parts; the genome's expressed growth program and the physics
//! floors do. This is the substrate the whole arc was built toward: with it, `Body::from_body_plan`'s
//! hand-drawn layout can be replaced by growth-from-genome, and one generative substrate serves both a
//! biosphere creature and a sentient body.
//!
//! What is heritable is the growth PROGRAM, expressed through [`Channel::Morphogen`] exactly as the
//! controller weights and composition coordinates are: one value per morphogen-parameter id, summed over
//! the genome's loci by [`GeneSet::express`], so a body's shape drifts, recombines, mutates, promotes,
//! and demotes through the existing pool with no per-channel code. The mechanism (this kernel) is fixed
//! Rust; the growth program's axis membership and parameter count are data (Principle 11), sibling to the
//! controller layout and the composition-axis registry.
//!
//! Determinism is the tall pole (Principle 3). The recursion is integer fixed-point, draws no float, and
//! keys every stochastic growth branch on [`Phase::MORPHOGEN`], folding the master seed, the growing
//! being's id, the parent segment and child index, and the growth generation, with a per-draw-site slot
//! (R-RNG-COORD). It is hard-capped in depth and in total segments, a termination guarantee (not a
//! natural bound), so growth always halts. The grown structure folds into `state_hash` in canonical
//! segment order. The kernel reads no race, kind, or niche id (Principle 9): a body's shape is a pure
//! function of its genome, the seed, and its id, and its parts' functions are a pure read of the grown
//! physics, so what a grown creature can do emerges rather than being tagged.
//!
//! Honest limits. Growth here is a single-rule scaling recursion (a segment spawns scaled children); the
//! richer L-system with several segment symbols, which grows qualitatively distinct organs in one body,
//! is the next increment. This slice is OFF the run path: it grows and reads a structure in isolation and
//! is unit-proven; wiring growth into the body assembly (replacing the catalog draw) is the following,
//! hash-changing slice. The geometry-to-value mapping grounds each axis in the floor's own range, but the
//! per-axis sub-ranges a segment uses are labelled dev fixtures until the floor registry supplies them.

use std::collections::BTreeMap;

use civsim_compose::{
    derive_capabilities, CapabilityCaps, CapabilityRefs, CapabilityVector, FunctionLawId,
    FunctionLawRegistry,
};
use civsim_core::{DrawKey, Fixed, Phase, StableId, StateHasher};

use crate::anatomy::{BodyPlan, Part, Temperament};
use crate::genome::{
    Channel, DominanceMode, GeneDef, GeneEffect, GeneId, GeneSet, Genome, MorphogenParamId,
};

/// One geometry or material axis a grown segment carries, with the floor range the expressed fraction
/// maps into. The id is a floor axis id (`mech.contact_area`, `mat.yield_strength`, `opt.refractive_index`,
/// and the rest); `lo`/`hi` are the range a segment's value on that axis spans, so the genome expresses a
/// dimensionless fraction in `[0, 1]` and growth maps it into the physical range. RESERVED (the ranges are
/// the floor's own axis ranges); labelled dev fixtures until a floor registry supplies them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisSpec {
    /// The floor axis id.
    pub axis: String,
    /// The low end of the range a segment's value on this axis spans.
    pub lo: Fixed,
    /// The high end of the range.
    pub hi: Fixed,
}

/// The data-defined layout of a morphogen program: which geometry and material axes each grown segment
/// carries (with their floor ranges), and the reserved growth bounds. The mechanism (the [`grow`] kernel)
/// is fixed Rust; the axis membership and the parameter count are data that grow with the floor
/// (Principle 11), the sibling to the controller layout and the composition-axis registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MorphogenProgram {
    /// The geometry (form) axes a segment carries (`mech.*`), each with its floor range.
    pub geometry_axes: Vec<AxisSpec>,
    /// The material axes a segment carries (`mat.*`, `opt.*`), each with its floor range.
    pub material_axes: Vec<AxisSpec>,
    /// The biology-floor tissue-composition axes a segment carries (`bio.*`, energy density, water fraction,
    /// and the rest), each with its floor range (emergent-anatomy Step 3, the metabolic-tier grow). These are
    /// grown into the same per-segment material map as the mechanical and optical axes, but they feed the
    /// METABOLISM rather than the function-law dispatch: a grown body's reserve capacity is summed directly
    /// off this composition ([`Structure::backed_capacity`]), so a grown body sources its own metabolism from
    /// its tissue with no organ kind id. Their parameters sit AFTER the branch and spawn parameters, so adding
    /// a bio axis does not shift the geometry, material, branch, or spawn parameter indices.
    pub bio_axes: Vec<AxisSpec>,
    /// RESERVED. The hard cap on growth generations (recursion depth), a termination guarantee. Basis: the
    /// number of developmental tiers a body plan represents (a handful), a performance-and-termination
    /// bound rather than a realism one.
    pub max_depth: u16,
    /// RESERVED. The hard cap on total segments, a termination guarantee. Basis: the largest part count a
    /// body plan represents; a performance-and-termination bound.
    pub max_segments: u16,
    /// RESERVED. The hard cap on children a segment may spawn in one generation. Basis: the maximum
    /// branching of a developmental node.
    pub max_branch: u16,
}

impl MorphogenProgram {
    /// The number of heritable parameters this program expresses: per geometry axis a root fraction and a
    /// per-generation growth fraction, per material axis a fraction, plus a branch fraction and a spawn
    /// fraction. The morphogen block appended to a founder gene set is this many loci.
    pub fn param_count(&self) -> usize {
        self.geometry_axes.len() * 2 + self.material_axes.len() + 2 + self.bio_axes.len()
    }

    /// The parameter index of geometry axis `i`'s root fraction.
    fn root_geo_param(&self, i: usize) -> usize {
        i
    }

    /// The parameter index of geometry axis `i`'s per-generation growth fraction.
    fn growth_param(&self, i: usize) -> usize {
        self.geometry_axes.len() + i
    }

    /// The parameter index of material axis `i`'s fraction.
    fn material_param(&self, i: usize) -> usize {
        self.geometry_axes.len() * 2 + i
    }

    /// The parameter index of the branch fraction.
    fn branch_param(&self) -> usize {
        self.geometry_axes.len() * 2 + self.material_axes.len()
    }

    /// The parameter index of the spawn fraction.
    fn spawn_param(&self) -> usize {
        self.branch_param() + 1
    }

    /// The parameter index of bio axis `i`'s tissue-composition fraction, placed AFTER the spawn parameter so
    /// adding a bio axis does not shift the geometry, material, branch, or spawn indices.
    fn bio_param(&self, i: usize) -> usize {
        self.spawn_param() + 1 + i
    }

    /// A labelled DEVELOPMENT FIXTURE program over the mechanical and optical floor axes a body part's
    /// function is read from, with plausible per-segment ranges grounded in the floor: a weapon reads
    /// PIERCE from a small `contact_area` and a hard material, a limb reads LOCOMOTE from a `section_modulus`
    /// and `arm_length` under yield, an eye reads REFRACT from a refractive index above the medium. Not
    /// owner canon; the ranges and caps are reserved-with-basis, and a canonical program reads them from
    /// the floor registry and the calibration manifest.
    pub fn dev_default() -> MorphogenProgram {
        let geo = |axis: &str, lo: Fixed, hi: Fixed| AxisSpec {
            axis: axis.to_string(),
            lo,
            hi,
        };
        MorphogenProgram {
            geometry_axes: vec![
                geo("mech.contact_area", dec("0.00000005"), dec("0.01")),
                geo("mech.section_modulus", dec("0.0000001"), dec("0.001")),
                geo("mech.arm_length", dec("0.01"), dec("1")),
                geo("mech.edge_radius", dec("0.00000005"), dec("0.01")),
            ],
            material_axes: vec![
                geo("mat.indentation_hardness", dec("1"), dec("3000")),
                geo("mat.yield_strength", dec("1"), dec("200")),
                geo("opt.refractive_index", dec("1"), dec("1.5")),
            ],
            // The biology-floor tissue-composition axes the metabolism reads: energy density (gross bomb
            // calorimetry, kJ/g) and water fraction, the two the dev homeostatic reserves back. RESERVED
            // (the floor's own axis ranges, biology_floor.toml); labelled dev fixtures until a floor registry
            // supplies them.
            bio_axes: vec![
                geo("bio.energy_density", dec("0"), dec("38")),
                geo("bio.water_fraction", dec("0"), dec("1")),
            ],
            // Labelled dev caps: a shallow developmental tree, wide enough to grow a limbed, multi-part
            // body but hard-bounded so the recursion always halts.
            max_depth: 4,
            max_segments: 32,
            max_branch: 3,
        }
    }
}

/// One grown segment: its place in the developmental tree (parent and depth) and its physics, the
/// geometry and material a body part's function is read from ([`civsim_compose::derive_capabilities`]).
/// The `geo`/`mat` accessors return zero for an absent axis, the same substrate-absence convention the
/// per-kind [`crate::anatomy::KindDef`] uses, so the function-law dispatch reads a grown segment exactly
/// as it reads a catalog part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// The parent segment index (the root has none).
    pub parent: Option<usize>,
    /// The growth generation the segment was grown at (its depth in the developmental tree).
    pub depth: u16,
    /// The segment's geometry on the form axes, keyed by axis id, sorted for a deterministic walk.
    pub geometry: BTreeMap<String, Fixed>,
    /// The segment's material on the mechanical and optical axes, keyed by axis id, sorted.
    pub material: BTreeMap<String, Fixed>,
}

impl Segment {
    /// The segment's value on a geometry axis, or zero if it carries none (the absence convention).
    pub fn geo(&self, axis: &str) -> Fixed {
        self.geometry.get(axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// The segment's value on a material axis, or zero if it carries none.
    pub fn mat(&self, axis: &str) -> Fixed {
        self.material.get(axis).copied().unwrap_or(Fixed::ZERO)
    }
}

/// A grown body: the segment graph a morphogen program produces. The parts' functions are never stored;
/// each is a pure read of the segment's grown geometry and material through the Step-1 dispatch, so a
/// creature the catalog never named still has weapons, limbs, and eyes exactly to the extent its grown
/// physics reads them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Structure {
    /// The segments, in growth order (the root first, then each child after its parent).
    pub segments: Vec<Segment>,
}

impl Structure {
    /// The number of segments.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Whether the structure is empty (no segments).
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// The deepest growth generation any segment reached.
    pub fn depth(&self) -> u16 {
        self.segments.iter().map(|s| s.depth).max().unwrap_or(0)
    }

    /// The capability vector one segment reads, a pure function of its own grown geometry and material
    /// through the Step-1 dispatch (blind to any id). The caller supplies the reserved reference levels
    /// and the physics ceilings, exactly as the catalog-part read does.
    pub fn segment_capabilities(
        &self,
        i: usize,
        fns: &FunctionLawRegistry,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> CapabilityVector {
        match self.segments.get(i) {
            Some(seg) => {
                let geo = |axis: &str| seg.geo(axis);
                let mat = |axis: &str| seg.mat(axis);
                derive_capabilities(fns, &geo, &mat, refs, caps)
            }
            None => CapabilityVector::default(),
        }
    }

    /// The greatest capability any segment reads on a function law: the body performs the function if a
    /// segment does, so a grown body strikes because a segment reads PIERCE, moves because one reads
    /// LOCOMOTE, sees because one reads REFRACT, by physics and never by a tag.
    pub fn max_capability(
        &self,
        law: FunctionLawId,
        fns: &FunctionLawRegistry,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> Fixed {
        let mut best = Fixed::ZERO;
        for i in 0..self.segments.len() {
            best = best.max(self.segment_capabilities(i, fns, refs, caps).score(law));
        }
        best
    }

    /// The whole-body functional viability (emergent-anatomy Step 3): the greatest capability any grown
    /// segment reads on ANY function law, a pure physics read over the grown graph. A body that reads a
    /// positive capability on some function (it can move, strike, or see) is functionally coherent; one that
    /// reads near zero on EVERY law is inert matter no life function can run on, so it reads a viability near
    /// zero and falls through a viability floor. It is the maximum over the same per-segment dispatch the
    /// affordance and speed reads use, blind to any race, kind, or label (Principle 9), never a stored score:
    /// the run sets a grown being's derived integrity reserve to this each tick, so an incoherent grown body
    /// dies through the ordinary reserve-floor cull with no predicate that inspects morphology to reject it.
    pub fn whole_body_viability(
        &self,
        fns: &FunctionLawRegistry,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> Fixed {
        let mut best = Fixed::ZERO;
        for def in fns.defs() {
            best = best.max(self.max_capability(def.id, fns, refs, caps));
        }
        best
    }

    /// The metabolic reserve capacity a grown body's tissue backs on a biology-floor axis (emergent-anatomy
    /// Step 3, the metabolic-tier grow): the sum over grown segments of each segment's development-weighted
    /// tissue composition on that axis. It mirrors how [`crate::homeostasis::Homeostasis::new`] sums a catalog
    /// organ set (`Σ development · composition`), but reads DIRECTLY off the grown segments' `bio.*` composition
    /// with no organ kind id and no registry, the metabolic sibling of the affordance and speed direct reads.
    /// Each segment's development is a uniform fraction of a full body (`1 / MASS_REFERENCE_SEGMENTS`, the same
    /// reference the digest's `body_mass` uses, so the reserve scale and the mass scale agree). A body whose
    /// grown tissue carries none of the axis backs no reserve there (the absence convention), so an
    /// energy-less grown body carries no energy reserve and starves at birth through the ordinary cull.
    pub fn backed_capacity(&self, axis_id: &str) -> Fixed {
        let per_segment = Fixed::from_ratio(1, MASS_REFERENCE_SEGMENTS as i64);
        let mut sum = Fixed::ZERO;
        for seg in &self.segments {
            let backed = per_segment
                .checked_mul(seg.mat(axis_id))
                .unwrap_or(Fixed::ZERO);
            sum = sum.saturating_add(backed);
        }
        sum
    }

    /// The whole-body per-mass energy density a grown body's tissue reads (`bio.energy_density`), the
    /// specific energy the derived metabolic drain reads to bridge the reserve to stored joules
    /// (emergent-anatomy Step 3): the development-weighted average over the grown segments, mirroring
    /// [`crate::physiology::whole_body_energy_density`] over a catalog organ set but read DIRECTLY off the
    /// grown tissue. Every grown segment carries the same development, so this is the mean of the segments'
    /// energy density. Zero for a body with no energy tissue (which starves at once, so the derived drain is
    /// never reached on it).
    pub fn whole_body_energy_density(&self) -> Fixed {
        let n = self.segments.len();
        if n == 0 {
            return Fixed::ZERO;
        }
        let mut sum = Fixed::ZERO;
        for seg in &self.segments {
            sum = sum.saturating_add(seg.mat("bio.energy_density"));
        }
        sum.checked_div(Fixed::from_int(n as i32))
            .unwrap_or(Fixed::ZERO)
    }

    /// The best locomotor limb the structure bears: the greatest LOCOMOTE capability any segment reads and
    /// that segment's leg length (`mech.arm_length`), the two the grown-limb ground speed
    /// ([`crate::locomotion::locomotion_speed_structure`]) reads. A structure whose every segment reads zero
    /// LOCOMOTE bears no limb, so the capability is zero and the body is rooted, by physics not by a tag.
    pub fn best_locomotor_stride(
        &self,
        fns: &FunctionLawRegistry,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> (Fixed, Fixed) {
        let mut best_cap = Fixed::ZERO;
        let mut stride = Fixed::ZERO;
        for (i, seg) in self.segments.iter().enumerate() {
            let cap = self
                .segment_capabilities(i, fns, refs, caps)
                .score(FunctionLawRegistry::ID_LOCOMOTE);
            if cap > best_cap {
                best_cap = cap;
                stride = seg.geo("mech.arm_length");
            }
        }
        (best_cap, stride)
    }

    /// The LOD-0 aggregate digest of the grown structure: the scalar [`BodyPlan`] the aggregate tier carries
    /// while the full segment graph materialises only on promotion (the spec's LOD split). It derives the
    /// morphology scalar the run reads on the tick, `body_mass`, as a coarse aggregate of the grown
    /// structure's extent (the segment count over a reserved reference, clamped), and leaves the behavioural
    /// and metabolic fields at neutral defaults: the temperament is a separate heritable channel, and the
    /// metabolic organs (`bio.*` tissue composition, which this function-morphology kernel does not grow yet)
    /// are supplied by the caller in the documented hybrid. A grown body's FUNCTION is read from the
    /// structure directly ([`Structure::max_capability`], the affordance and speed reads), never from the
    /// digest's part-lists, so those stay empty: the digest is the scalar summary, not the function source.
    pub fn digest(&self) -> BodyPlan {
        let count = Fixed::from_int(self.segments.len() as i32);
        let reference = Fixed::from_int(MASS_REFERENCE_SEGMENTS as i32);
        let body_mass = count
            .checked_div(reference)
            .unwrap_or(Fixed::ZERO)
            .clamp(Fixed::ZERO, Fixed::ONE);
        BodyPlan {
            body_mass,
            encephalization: HALF,
            diet_breadth: HALF,
            weapons: Vec::new(),
            covering: Part {
                kind: 0,
                development: HALF,
            },
            senses: Vec::new(),
            locomotion: Vec::new(),
            organs: Vec::new(),
            temperament: Temperament {
                boldness: HALF,
                exploration: HALF,
                activity: HALF,
                sociability: HALF,
                aggression: Fixed::from_ratio(1, 4),
            },
        }
    }

    /// Fold the structure into a state hasher in canonical order: each segment in growth order, and within
    /// a segment the parent, depth, then its geometry and material axes in sorted key order (the `BTreeMap`
    /// walk is already sorted). Determinism (Principle 3): a grown structure folds identically wherever it
    /// is recomputed, so the run's `state_hash` is a pure function of the grown body.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u32(self.segments.len() as u32);
        for seg in &self.segments {
            h.write_u64(seg.parent.map(|p| p as u64).unwrap_or(u64::MAX));
            h.write_u32(seg.depth as u32);
            h.write_u32(seg.geometry.len() as u32);
            for (axis, v) in &seg.geometry {
                h.write_bytes(axis.as_bytes());
                h.write_fixed(*v);
            }
            h.write_u32(seg.material.len() as u32);
            for (axis, v) in &seg.material {
                h.write_bytes(axis.as_bytes());
                h.write_fixed(*v);
            }
        }
    }
}

/// Express a morphogen program's parameter vector from a genome, one value per morphogen-parameter id, by
/// summing over the genome's loci exactly as the controller weights are expressed. Pure, no RNG, no float.
pub fn express_program(program: &MorphogenProgram, genes: &GeneSet, genome: &Genome) -> Vec<Fixed> {
    (0..program.param_count())
        .map(|k| {
            genes.express(
                genome,
                Channel::Morphogen(MorphogenParamId(k as u32)),
                Fixed::ZERO,
            )
        })
        .collect()
}

/// A founder gene set for a morphogen program: one unit-weight locus per growth parameter, feeding
/// [`Channel::Morphogen`], mirroring [`crate::evolve::controller_gene_set`]. A founder pool built over
/// this evolves the body's shape exactly as the controller weights evolve.
pub fn morphogen_gene_set(program: &MorphogenProgram) -> GeneSet {
    let genes = (0..program.param_count())
        .map(|k| GeneDef {
            id: GeneId(k as u32),
            effects: vec![GeneEffect {
                channel: Channel::Morphogen(MorphogenParamId(k as u32)),
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode::additive(),
        })
        .collect();
    GeneSet { genes }
}

/// The slot for the spawn draw (whether a candidate child segment grows).
const SLOT_SPAWN: u32 = 0;
/// The slot for the per-child geometry jitter draw.
const SLOT_JITTER: u32 = 1;
/// The bound on the multiplicative geometry jitter a child's growth carries, so a lineage's segments vary
/// a little around the deterministic growth without the caps being at risk. RESERVED. Basis: the
/// developmental noise a growth process carries around its programmed rate; small, a fraction.
const JITTER_MAG: Fixed = Fixed::from_bits(1i64 << (Fixed::FRAC_BITS - 3)); // 1/8

/// Grow a body's structure from an expressed morphogen program under bounded, deterministic recursion. A
/// root segment is grown from the root parameters; each generation, every frontier segment may spawn up to
/// `max_branch` children, each spawn gated by a counter-keyed roll against the program's spawn fraction and
/// each child's geometry the parent's scaled by the per-axis growth factor with a small counter-keyed
/// jitter. The recursion halts at the reserved depth or segment cap, whichever comes first (a termination
/// guarantee). Deterministic and id-keyed: `(program, params, seed, id)` grows one structure, folded into
/// `state_hash` by [`Structure::hash_into`], blind to any race, kind, or niche.
pub fn grow(program: &MorphogenProgram, params: &[Fixed], seed: u64, id: StableId) -> Structure {
    let max_seg = program.max_segments.max(1) as usize;
    // The root segment: geometry and material mapped from the root fractions into the floor ranges.
    let mut geometry = BTreeMap::new();
    for (i, spec) in program.geometry_axes.iter().enumerate() {
        let f = frac(params, program.root_geo_param(i));
        geometry.insert(spec.axis.clone(), map_range(f, spec.lo, spec.hi));
    }
    let mut material = BTreeMap::new();
    for (i, spec) in program.material_axes.iter().enumerate() {
        let f = frac(params, program.material_param(i));
        material.insert(spec.axis.clone(), map_range(f, spec.lo, spec.hi));
    }
    // The biology-floor tissue composition (emergent-anatomy Step 3): grown into the same material map as the
    // mechanical and optical axes, so a grown segment carries its metabolic tissue (`bio.*`) alongside its
    // function-geometry, and a child inherits it exactly as it inherits the material. This is what the
    // metabolism sums off ([`Structure::backed_capacity`]); the function-law dispatch reads only the
    // mechanical and optical axes, so the bio composition is metabolic-only and blind to the affordance reads.
    for (i, spec) in program.bio_axes.iter().enumerate() {
        let f = frac(params, program.bio_param(i));
        material.insert(spec.axis.clone(), map_range(f, spec.lo, spec.hi));
    }
    let mut segments = vec![Segment {
        parent: None,
        depth: 0,
        geometry,
        material,
    }];

    let branch = branch_count(program, params);
    let spawn_threshold = frac(params, program.spawn_param());
    let mut frontier = vec![0usize];
    for gen in 0..program.max_depth {
        if segments.len() >= max_seg {
            break;
        }
        let mut next = Vec::new();
        for &s in &frontier {
            for c in 0..branch {
                if segments.len() >= max_seg {
                    break;
                }
                let locus = ((s as u64) << 8) | c as u64;
                let spawn = DrawKey::pair(id.0, locus, gen as u64, Phase::MORPHOGEN)
                    .slot(SLOT_SPAWN)
                    .rng(seed)
                    .unit_fixed(0);
                if spawn >= spawn_threshold {
                    continue; // this candidate child does not grow
                }
                let child = grow_child(program, params, &segments[s], s, gen, c, seed, id);
                let idx = segments.len();
                segments.push(child);
                next.push(idx);
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    Structure { segments }
}

/// Grow one child segment: its geometry is the parent's scaled by the per-axis growth factor and a small
/// counter-keyed jitter, clamped back into the axis range; its material is inherited from the parent (one
/// limb shares its material). Pure fixed-point.
#[allow(clippy::too_many_arguments)]
fn grow_child(
    program: &MorphogenProgram,
    params: &[Fixed],
    parent: &Segment,
    parent_idx: usize,
    gen: u16,
    child: u16,
    seed: u64,
    id: StableId,
) -> Segment {
    let mut geometry = BTreeMap::new();
    for (i, spec) in program.geometry_axes.iter().enumerate() {
        let growth = growth_factor(program, params, i);
        // A counter-keyed multiplicative jitter in [1 - JITTER_MAG, 1 + JITTER_MAG].
        let locus = ((parent_idx as u64) << 8) | child as u64;
        let roll = DrawKey::pair(id.0, locus, gen as u64, Phase::MORPHOGEN)
            .slot(SLOT_JITTER)
            .rng(seed)
            .unit_fixed(i as u64);
        let jitter = Fixed::ONE + (roll - HALF).mul(Fixed::from_int(2)).mul(JITTER_MAG);
        let raw = parent.geo(&spec.axis).mul(growth).mul(jitter);
        geometry.insert(spec.axis.clone(), raw.clamp(spec.lo, spec.hi));
    }
    Segment {
        parent: Some(parent_idx),
        depth: gen + 1,
        geometry,
        material: parent.material.clone(),
    }
}

/// The number of children a segment may spawn per generation, the branch fraction mapped to `0..=max_branch`
/// and rounded. Bounded by the reserved `max_branch`.
fn branch_count(program: &MorphogenProgram, params: &[Fixed]) -> u16 {
    let f = frac(params, program.branch_param());
    let scaled = f.mul(Fixed::from_int(program.max_branch as i32));
    // Round to the nearest whole child (add one half, truncate), clamped to the cap.
    let n = (scaled + HALF).to_int().max(0) as u16;
    n.min(program.max_branch)
}

/// The per-generation growth factor for geometry axis `i`: the growth fraction mapped into a bounded
/// multiplier so a child is at most modestly larger or smaller than its parent, keeping the recursion's
/// geometry within the floor range. RESERVED bounds. Basis: the per-tier size change a developmental step
/// carries, from a taper (below one) to a modest enlargement (above one).
fn growth_factor(program: &MorphogenProgram, params: &[Fixed], i: usize) -> Fixed {
    // Map the [0, 1] growth fraction into [GROWTH_LO, GROWTH_HI].
    let f = frac(params, program.growth_param(i));
    GROWTH_LO + f.mul(GROWTH_HI - GROWTH_LO)
}

/// The reserved lower bound on a per-generation growth factor (a taper). Basis: the strongest taper a
/// developmental step carries (a segment shrinking toward a tip).
const GROWTH_LO: Fixed = Fixed::from_bits(1i64 << (Fixed::FRAC_BITS - 1)); // 1/2
/// The reserved upper bound on a per-generation growth factor (an enlargement). Basis: the largest
/// per-tier enlargement a developmental step carries.
const GROWTH_HI: Fixed = Fixed::from_bits(3i64 << (Fixed::FRAC_BITS - 1)); // 3/2

/// One half, the jitter and rounding centre.
const HALF: Fixed = Fixed::from_bits(1i64 << (Fixed::FRAC_BITS - 1));

/// The reserved reference segment count a full-mass body's grown structure reaches, so the LOD-0
/// [`Structure::digest`] maps segment count to a `body_mass` fraction in `[0, 1]`. RESERVED. Basis: the
/// segment count a maximal-mass body plan grows to (near the segment cap), a coarse aggregate the metabolism
/// scale reads until a volumetric aggregate over the grown geometry replaces it.
const MASS_REFERENCE_SEGMENTS: u16 = 16;

/// An expressed parameter clamped to a `[0, 1]` fraction (the genome expresses a dimensionless coordinate;
/// growth maps it into a physical range). An absent parameter reads zero.
fn frac(params: &[Fixed], i: usize) -> Fixed {
    params
        .get(i)
        .copied()
        .unwrap_or(Fixed::ZERO)
        .clamp(Fixed::ZERO, Fixed::ONE)
}

/// Map a `[0, 1]` fraction into `[lo, hi]`.
fn map_range(fraction: Fixed, lo: Fixed, hi: Fixed) -> Fixed {
    lo + fraction.mul(hi - lo)
}

/// A decimal-string to `Fixed` for the labelled dev-fixture axis ranges. Panics on a malformed literal (a
/// fixture programming error, never runtime input).
fn dec(s: &str) -> Fixed {
    Fixed::from_decimal_str(s).expect("morphogen dev-fixture decimal literal")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{Allele, Haplotype, SchemeId};
    use crate::homeostasis::{AffordanceRegistry, Homeostasis, HomeostaticRegistry, MOVE, STRIKE};
    use crate::locomotion::{locomotion_speed_structure, LocomotionParams};

    fn caps() -> CapabilityCaps {
        CapabilityCaps {
            pressure: Fixed::from_int(150_000),
            depth: Fixed::from_int(100),
        }
    }

    /// A parameter vector of the program's length, all zero except the given (index, fraction) pairs.
    fn params(program: &MorphogenProgram, set: &[(usize, &str)]) -> Vec<Fixed> {
        let mut v = vec![Fixed::ZERO; program.param_count()];
        for &(i, s) in set {
            v[i] = dec(s);
        }
        v
    }

    #[test]
    fn growth_is_deterministic_and_folds_identically() {
        // Principle 3: the same program, parameters, seed, and id grow the identical structure, folded to
        // the identical hash, however many times it is recomputed. Growth draws no float and no wall clock.
        let program = MorphogenProgram::dev_default();
        // A branching program (some spawn, a couple of branches) so the recursion and its RNG are exercised.
        let p = params(
            &program,
            &[
                (0, "0.3"),
                (1, "0.5"),
                (2, "0.4"),
                (4, "0.6"),
                (5, "0.6"),
                (6, "0.6"),
                (7, "0.6"),
                (11, "0.7"), // branch fraction
                (12, "0.8"), // spawn fraction
            ],
        );
        let a = grow(&program, &p, 0xC0FFEE, StableId(7));
        let b = grow(&program, &p, 0xC0FFEE, StableId(7));
        assert_eq!(a, b, "growth replays bit for bit");
        let mut ha = StateHasher::new();
        a.hash_into(&mut ha);
        let mut hb = StateHasher::new();
        b.hash_into(&mut hb);
        assert_eq!(
            ha.finish(),
            hb.finish(),
            "the grown structure folds identically"
        );
        assert!(a.len() > 1, "the branching program grew more than the root");
    }

    #[test]
    fn a_different_id_or_seed_grows_a_different_structure_but_stays_deterministic() {
        // The stochastic growth branches key on (seed, id), so two beings with the same program diverge in
        // shape, yet each is a pure function of its own key: shape is heritable-plus-individual, never random.
        let program = MorphogenProgram::dev_default();
        let p = params(
            &program,
            &[
                (1, "0.5"),
                (2, "0.4"),
                (4, "0.6"),
                (5, "0.6"),
                (11, "1"),
                (12, "0.5"),
            ],
        );
        let a = grow(&program, &p, 0x1, StableId(1));
        let b = grow(&program, &p, 0x1, StableId(2));
        assert_ne!(
            a, b,
            "a different id grows a different body from the same program"
        );
        let a2 = grow(&program, &p, 0x1, StableId(1));
        assert_eq!(a, a2, "but each being's growth is deterministic");
    }

    #[test]
    fn growth_terminates_under_the_depth_and_segment_caps() {
        // The termination guarantee (Principle 11 reserved caps): even a program that spawns the maximum
        // branches with certainty is bounded by the depth and segment caps, so the recursion always halts.
        let program = MorphogenProgram::dev_default();
        // Growth 1.0 on every axis (no taper), max branch fraction, spawn fraction 1 (every candidate grows).
        let p = params(
            &program,
            &[
                (4, "1"),
                (5, "1"),
                (6, "1"),
                (7, "1"),
                (11, "1"), // branch fraction -> max_branch
                (12, "1"), // spawn fraction -> every candidate spawns
            ],
        );
        for id in 0..8u64 {
            let s = grow(&program, &p, 0xABCD, StableId(id));
            assert!(
                s.len() <= program.max_segments as usize,
                "segment count is capped ({} <= {})",
                s.len(),
                program.max_segments
            );
            assert!(
                s.depth() <= program.max_depth,
                "depth is capped ({} <= {})",
                s.depth(),
                program.max_depth
            );
        }
    }

    #[test]
    fn a_grown_segment_earns_its_function_from_its_physics() {
        // The Step-1 dispatch reads function off the GROWN graph: a body grown with weapon-like geometry
        // reads PIERCE and not LOCOMOTE, one grown with limb-like geometry reads LOCOMOTE, one grown with a
        // dense optical tissue reads REFRACT, by physics and never by a catalog kind or a tag.
        let program = MorphogenProgram::dev_default();
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = caps();

        // A weapon body: a tiny contact area (fraction 0 -> the range low) and a hard material (fraction 1
        // -> the range high). Spawn 0 so it is the single root segment. It should read PIERCE, not LOCOMOTE.
        let weapon = grow(
            &program,
            &params(&program, &[(0, "0"), (8, "1")]),
            0x1,
            StableId(1),
        );
        assert!(
            weapon.max_capability(FunctionLawRegistry::ID_PIERCE, &fns, &refs, &caps) > Fixed::ZERO,
            "a hard, small-area grown segment is a weapon by its physics"
        );
        assert_eq!(
            weapon.max_capability(FunctionLawRegistry::ID_LOCOMOTE, &fns, &refs, &caps),
            Fixed::ZERO,
            "and it is no limb: it bears no section modulus"
        );

        // A limb body: a real section modulus and arm length, a bony yield, a blunt (large) contact area so
        // it is no weapon. It should read LOCOMOTE, not PIERCE.
        let limb = grow(
            &program,
            &params(&program, &[(0, "1"), (1, "0.5"), (2, "0.4"), (9, "0.75")]),
            0x1,
            StableId(1),
        );
        assert!(
            limb.max_capability(FunctionLawRegistry::ID_LOCOMOTE, &fns, &refs, &caps) > Fixed::ZERO,
            "a grown limb bears its propulsive load and is a locomotor by its physics"
        );
        assert_eq!(
            limb.max_capability(FunctionLawRegistry::ID_PIERCE, &fns, &refs, &caps),
            Fixed::ZERO,
            "and its blunt large-area tip is no weapon"
        );

        // An eye body: an optical tissue whose refractive index (fraction 1 -> 1.5) exceeds the medium.
        let eye = grow(&program, &params(&program, &[(10, "1")]), 0x1, StableId(1));
        assert!(
            eye.max_capability(FunctionLawRegistry::ID_REFRACT, &fns, &refs, &caps) > Fixed::ZERO,
            "a grown tissue denser than the medium focuses light and is an eye by its physics"
        );
    }

    #[test]
    fn whole_body_viability_is_positive_for_a_functional_body_and_zero_for_inert_matter() {
        // Emergent-anatomy Step 3, the viability read the run-tier cull sets a grown being's integrity from:
        // a body that reads a positive capability on SOME function (a limb, a weapon, or an eye) is
        // functionally coherent and reads a positive whole-body viability; a body grown from an all-zero
        // program is inert matter (every axis at the floor's low end, so its tiny section buckles, its soft
        // tip cannot pierce, and its tissue matches the medium and does not focus), reads no viable function,
        // and its whole-body viability is zero. The read is the maximum over the same per-segment dispatch the
        // affordance and speed reads use, so it earns its verdict from the grown physics and no tag.
        let program = MorphogenProgram::dev_default();
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = caps();

        let limb = grow(
            &program,
            &params(&program, &[(0, "1"), (1, "0.5"), (2, "0.4"), (9, "0.75")]),
            0x1,
            StableId(1),
        );
        assert!(
            limb.whole_body_viability(&fns, &refs, &caps) > Fixed::ZERO,
            "a body that reads a viable function is functionally coherent"
        );

        // An all-zero program: the root geometry maps every fraction to the floor's low end, a degenerate
        // body no function can run on.
        let inert = grow(
            &program,
            &vec![Fixed::ZERO; program.param_count()],
            0x1,
            StableId(1),
        );
        assert_eq!(
            inert.whole_body_viability(&fns, &refs, &caps),
            Fixed::ZERO,
            "inert matter reads no viable function and falls through the viability floor"
        );
    }

    #[test]
    fn a_grown_body_sources_its_metabolic_reserve_from_its_own_tissue() {
        // Emergent-anatomy Step 3, the metabolic-tier grow: a grown body sources its own metabolic reserve
        // capacity from its grown bio.* tissue composition, summed DIRECTLY off the segments with no organ
        // kind id (the metabolic sibling of the affordance direct read). A body whose tissue carries energy
        // density backs a positive energy reserve; one whose tissue carries none backs zero and starves at
        // birth through the ordinary reserve-floor cull, never a morphology gate.
        let program = MorphogenProgram::dev_default();
        // The bio parameters sit after spawn: energy_density then water_fraction (see `bio_param`).
        let energy = program.param_count() - 2;
        let water = program.param_count() - 1;

        let nourished = grow(
            &program,
            &params(&program, &[(energy, "0.5"), (water, "0.5")]),
            0x1,
            StableId(1),
        );
        assert!(
            nourished.backed_capacity("bio.energy_density") > Fixed::ZERO,
            "a grown body whose tissue carries energy density backs a positive energy reserve"
        );
        assert!(
            nourished.backed_capacity("bio.water_fraction") > Fixed::ZERO,
            "and a positive water reserve from its water fraction"
        );

        let energyless = grow(
            &program,
            &vec![Fixed::ZERO; program.param_count()],
            0x1,
            StableId(1),
        );
        assert_eq!(
            energyless.backed_capacity("bio.energy_density"),
            Fixed::ZERO,
            "a grown body whose tissue carries no energy backs no energy reserve"
        );

        // Homeostasis::from_structure builds the reserve set over a metabolizing (energy and water) registry:
        // a nourished grown body is born alive on its own reserves, an energy-less one is born already dead
        // (its reserve is at the floor), the metabolic viability that lets a grown race need no catalog body.
        let reg = HomeostaticRegistry::dev_default();
        assert!(
            Homeostasis::from_structure(&reg, &nourished).is_alive(&reg),
            "a grown body with metabolic tissue is birth-viable on its own grown reserves"
        );
        assert!(
            !Homeostasis::from_structure(&reg, &energyless).is_alive(&reg),
            "an energy-less grown body carries no reserve and starves at birth"
        );
    }

    #[test]
    fn the_morphogen_channel_expresses_and_is_heritable() {
        // The growth program is heritable data expressed through Channel::Morphogen, exactly like the
        // controller weights: a locus carrying a nonzero allele lifts its parameter above one that does not,
        // and the read is deterministic. So a body's shape is a lineage's inheritance.
        let program = MorphogenProgram::dev_default();
        let genes = morphogen_gene_set(&program);
        let n = program.param_count();
        let homozygous = |set: &[(usize, Fixed)]| -> Genome {
            let mut alleles = vec![Allele::additive(Fixed::ZERO); n];
            for &(k, v) in set {
                alleles[k] = Allele::additive(v);
            }
            let hap = Haplotype { alleles };
            Genome {
                scheme: SchemeId(0),
                haps: vec![hap.clone(), hap],
            }
        };
        let blank = homozygous(&[]);
        let carrier = homozygous(&[(2, Fixed::from_ratio(1, 4))]); // a nonzero allele at parameter 2

        let blank_params = express_program(&program, &genes, &blank);
        let carrier_params = express_program(&program, &genes, &carrier);
        assert_eq!(
            blank_params.len(),
            n,
            "one expressed value per morphogen parameter"
        );
        assert!(
            carrier_params[2] > blank_params[2],
            "a nonzero allele lifts its growth parameter (heritable shape)"
        );
        assert_eq!(
            express_program(&program, &genes, &carrier),
            carrier_params,
            "expression is deterministic"
        );
        // And the expressed program grows a body: the two genotypes grow different structures.
        let a = grow(&program, &blank_params, 0x5, StableId(1));
        let b = grow(&program, &carrier_params, 0x5, StableId(1));
        assert_ne!(
            a, b,
            "a heritable growth-parameter difference grows a different body"
        );
    }

    // --- Slice B, the digest and the direct-read bridge (still off the run path) ---

    /// A branching program that grows a multi-segment body, so the digest and the reads see structure.
    fn branching(program: &MorphogenProgram) -> Vec<Fixed> {
        params(
            program,
            &[
                (1, "0.5"),
                (2, "0.4"),
                (4, "0.7"),
                (5, "0.7"),
                (9, "0.75"),
                (11, "1"),   // branch fraction -> max_branch
                (12, "0.9"), // spawn fraction -> most candidates grow
            ],
        )
    }

    #[test]
    fn the_digest_aggregates_the_grown_structure_to_a_bodyplan() {
        // The LOD-0 aggregate: a larger grown structure digests to a greater body_mass, the one morphology
        // scalar the metabolism reads on the tick. The digest is a valid BodyPlan (its function is read from
        // the structure directly, so its part-lists are empty by design).
        let program = MorphogenProgram::dev_default();
        let big = grow(&program, &branching(&program), 0x7, StableId(1));
        let small = grow(&program, &params(&program, &[]), 0x7, StableId(1)); // spawn 0 -> the root alone
        assert!(
            big.len() > small.len(),
            "the branching program grew a bigger body"
        );
        assert!(
            big.digest().body_mass > small.digest().body_mass,
            "a bigger grown structure digests to a greater body_mass"
        );
        assert!(
            small.len() == 1,
            "the non-branching program is the single root"
        );
        assert!(
            big.digest().weapons.is_empty() && big.digest().locomotion.is_empty(),
            "the digest's part-lists are empty: a grown body's function is read from the structure"
        );
    }

    #[test]
    fn the_run_reads_a_grown_body_directly_by_physics() {
        // The direct-read bridge (owner's slice-B choice): the affordance gate and the ground speed read a
        // grown Structure directly, with no catalog kind id and no organs registry. A grown limbed body
        // affords MOVE and moves; a rooted grown body (no segment reads LOCOMOTE) affords no MOVE and does
        // not move; a grown weapon body affords STRIKE. Function is a pure read of the grown physics.
        let program = MorphogenProgram::dev_default();
        let p = LocomotionParams::dev_default();
        let refs = CapabilityRefs::dev_refs();
        let caps = CapabilityCaps {
            pressure: Fixed::from_int(150_000),
            depth: Fixed::from_int(100),
        };
        let half = Fixed::from_ratio(1, 2);

        // A limbed body: a real section modulus, arm length, and bony yield.
        let limbed = grow(
            &program,
            &params(&program, &[(1, "0.5"), (2, "0.4"), (9, "0.75")]),
            0x1,
            StableId(1),
        );
        let afford = AffordanceRegistry::dev_default();
        assert!(
            afford
                .afforded_structure(&limbed, &refs, &caps)
                .contains(&MOVE),
            "a grown limbed body affords MOVE by physics"
        );
        assert!(
            locomotion_speed_structure(&limbed, half, Fixed::ONE, &p) > Fixed::ZERO,
            "and it moves"
        );

        // A rooted body: everything at the low end, so no segment bears a load-bearing limb.
        let rooted = grow(&program, &params(&program, &[]), 0x1, StableId(1));
        assert!(
            !afford
                .afforded_structure(&rooted, &refs, &caps)
                .contains(&MOVE),
            "a rooted grown body affords no movement: no segment reads LOCOMOTE"
        );
        assert_eq!(
            locomotion_speed_structure(&rooted, half, Fixed::ONE, &p),
            Fixed::ZERO,
            "and it does not move"
        );

        // A weapon body: a small hard tip, in a combat-capable affordance fixture, affords STRIKE.
        let weapon = grow(
            &program,
            &params(&program, &[(0, "0"), (8, "1")]),
            0x1,
            StableId(1),
        );
        let predator = AffordanceRegistry::dev_predator();
        assert!(
            predator
                .afforded_structure(&weapon, &refs, &caps)
                .contains(&STRIKE),
            "a grown weapon body affords a strike by its physics"
        );
    }
}
