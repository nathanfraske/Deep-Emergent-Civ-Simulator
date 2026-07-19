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

use std::collections::{BTreeMap, BTreeSet};

use civsim_compose::{
    derive_capabilities, CapabilityCaps, CapabilityRefs, CapabilityVector, FunctionLawId,
    FunctionLawRegistry,
};
use civsim_core::{DrawKey, Fixed, Phase, StableId, StateHasher};

use crate::insult;
use civsim_bio::anatomy::{BodyPlan, Part, Temperament};
use civsim_bio::genome::{
    Channel, DominanceMode, GeneDef, GeneEffect, GeneId, GeneSet, Genome, MorphogenParamId,
};
use civsim_physics::laws;

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
    /// The tissue-composition axes a segment carries that feed the METABOLISM rather than the function-law
    /// dispatch (emergent-anatomy Step 3, the metabolic-tier grow): the biology-floor `bio.*` axes (energy
    /// density, water fraction, convective and respiratory surface), the `mat.fracture_strength` a muscle's
    /// force integrates over, the `therm.specific_heat` a body's thermal mass reads, and the `mat.density` its
    /// buoyancy reads, each with its floor range. These are grown into the same per-segment material map as
    /// the mechanical and optical axes, but a grown body's reserve capacity, muscle force, exchange surface,
    /// and thermal mass are summed or averaged directly off this composition ([`Structure::composition_sum`],
    /// [`Structure::composition_mean`]), so a grown body sources its own metabolism and physiology from its
    /// tissue with no organ kind id. Their parameters sit AFTER the branch and spawn parameters, so adding a
    /// composition axis does not shift the geometry, material, branch, or spawn parameter indices.
    pub composition_axes: Vec<AxisSpec>,
    /// The ACTUATOR geometry axes a segment carries for the stroke-rate substrate (`mech.cross_section_area`,
    /// `mech.stroke_length`), each with its floor range. They are GEOMETRY axes (grown into the per-segment
    /// geometry map like [`Self::geometry_axes`], with a root and a per-generation growth fraction), so a grown
    /// body's actuator has a real cross-section and stroke and its `F d` blow is non-zero. They are held in a
    /// SEPARATE list, and their parameters sit at the END, AFTER the composition parameters, so adding an
    /// actuator axis shifts NONE of the existing indices (geometry, material, branch, spawn, composition): the
    /// growth is strictly additive, so every existing grown value is byte-identical and only the new
    /// cross-section and stroke are added. Because the actuator params are now last, the composition energy-
    /// density and water-fraction axes are addressed by `composition_param(composition_axes.len() - 2)` and
    /// `- 1`, not by `param_count()`. Empty leaves growth unchanged (a body without an actuator).
    pub actuator_axes: Vec<AxisSpec>,
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
        self.geometry_axes.len() * 2
            + self.material_axes.len()
            + 2
            + self.actuator_axes.len() * 2
            + self.composition_axes.len()
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

    /// The parameter index of composition axis `i`'s tissue-composition fraction, placed AFTER the spawn
    /// parameter so adding a bio axis does not shift the geometry, material, branch, or spawn indices. The
    /// actuator parameters follow the composition block (see below), so the composition indices are UNCHANGED
    /// by the stroke-rate substrate: a grown body's tissue composition is byte-identical to before. Public so a
    /// founder-seeding fixture addresses a composition axis by asking the program (`composition_param(i)`)
    /// rather than recomputing the offset off `param_count()`, which drifts the moment a param category is
    /// appended (the stroke-rate actuator block did exactly that): the accessor is the one source of the layout.
    pub fn composition_param(&self, i: usize) -> usize {
        self.spawn_param() + 1 + i
    }

    /// The parameter index of actuator axis `i`'s root fraction (the stroke-rate substrate), placed at the END,
    /// AFTER the composition parameters, so it shifts NONE of the existing indices (geometry, material, branch,
    /// spawn, composition all unchanged): the two actuator axes are a strictly-additive growth, so every existing
    /// grown value is byte-identical and only the new cross-section and stroke are added. Because the actuator
    /// params are now last, the energy-density and water-fraction composition axes are addressed by
    /// `composition_param(composition_axes.len() - 2)` and `- 1`, not by `param_count() - 2` / `- 1`.
    fn actuator_root_param(&self, i: usize) -> usize {
        self.spawn_param() + 1 + self.composition_axes.len() + i
    }

    /// The parameter index of actuator axis `i`'s per-generation growth fraction (after the actuator roots).
    fn actuator_growth_param(&self, i: usize) -> usize {
        self.spawn_param() + 1 + self.composition_axes.len() + self.actuator_axes.len() + i
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
            // The tissue-composition axes the metabolism and physiology read. RESERVED (the floor's own axis
            // ranges, biology_floor.toml and the mechanical/thermal floors); labelled dev fixtures until a
            // floor registry supplies them. The convective surface, muscle strength, and specific heat lead;
            // energy density and water fraction stay LAST in the composition block (the two the reserves back),
            // addressed by `composition_param(composition_axes.len() - 2)` and `- 1` so a caller stays stable as
            // more physiology axes are added; the actuator params follow the composition block.
            composition_axes: vec![
                geo("bio.convective_surface", dec("0"), dec("500")),
                geo("mat.fracture_strength", dec("0"), dec("200")),
                geo("therm.specific_heat", dec("0"), dec("5000")),
                geo("bio.energy_density", dec("0"), dec("38")),
                geo("bio.water_fraction", dec("0"), dec("1")),
            ],
            // The actuator axes (the stroke-rate substrate): a load-bearing cross-section and a stroke length the
            // `F d` blow reads, grown per-segment so a grown body strikes with its own force and reach. RESERVED
            // ranges: plausible per-segment SUB-ranges WITHIN the floor's declared `mech.cross_section_area` and
            // `mech.stroke_length` bounds (not the full floor bounds), labelled dev fixtures on the same footing
            // as the geometry and composition axes above until a floor registry supplies them.
            actuator_axes: vec![
                geo("mech.cross_section_area", dec("0.00000005"), dec("0.01")),
                geo("mech.stroke_length", dec("0.01"), dec("1")),
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
/// per-kind [`civsim_bio::anatomy::KindDef`] uses, so the function-law dispatch reads a grown segment exactly
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
    /// The segment's accumulated aging damage as a NORMALIZED FRACTION of its own failure tolerance
    /// (R-AGING (c), the first-passage accumulator), in `[0, ONE]`. Zero at growth; advanced each tick
    /// by the load-grounded net damage (insult minus funded repair, normalized once by the tolerance)
    /// where the run arms aging. The fraction store keeps a wound-style additive accumulation exact (no
    /// energy round-trip); its per-tick advance lives in the run loop, this field is the state it writes.
    pub damage: Fixed,
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

    /// This segment's own failure-energy tolerance (the Griffith product `fracture_energy * crack_area`,
    /// in Joules), read from its own grown data: `mat.fracture_energy` (J/m^2) times its cross-sectional
    /// `mech.contact_area` (m^2), the same product the wound and strike fracture tests measure delivered
    /// energy against. A segment that carries no fracture-energy datum reads a zero tolerance: it has no
    /// failure reserve to age against, the absence convention that keeps aging OPT-IN and data-driven
    /// (Principle 11), so a race arms aging by growing tissue that declares a fracture energy, and an
    /// alien whose failure mode is non-mechanical declares its own failure energy on the same axis.
    pub fn failure_tolerance(&self) -> Fixed {
        self.mat("mat.fracture_energy")
            .checked_mul(self.geo("mech.contact_area"))
            .unwrap_or(Fixed::ZERO)
    }

    /// This segment's derived structural integrity in `[0, ONE]`: one minus its accumulated normalized
    /// damage fraction. A segment with no failure tolerance (no fracture-energy datum) reads full
    /// integrity ONE whatever its stored value: it has no aging coupling (the absence convention), NOT a
    /// failed part, the inverse of `insult::derive_integrity`'s zero-tolerance-is-already-failed
    /// convention, so a body whose segments declare no fracture energy keeps full capability and the run
    /// stays byte-identical until a race arms aging.
    pub fn integrity(&self) -> Fixed {
        if self.failure_tolerance() <= Fixed::ZERO {
            return Fixed::ONE;
        }
        Fixed::from_bits(
            Fixed::ONE.to_bits() - self.damage.clamp(Fixed::ZERO, Fixed::ONE).to_bits(),
        )
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

    /// The whole-body viability with per-segment AGING applied (R-AGING (c) route (i), the first-passage
    /// death path): each segment's capability on a function law is scaled by that segment's own derived
    /// integrity before the body takes the greatest, so a body worn down by accumulated damage reads a
    /// falling viability and dies through the SAME reserve-floor cull as an inert one, with no vital-part
    /// predicate (Principle 8: death stays the emergent cull, never a morphology gate). Identical to
    /// [`Structure::whole_body_viability`] when no segment has accrued damage, because a full-integrity
    /// segment scales its capability by ONE (an exact fixed-point identity), so a run with no aging armed
    /// (every segment at zero damage, or carrying no fracture-energy tolerance) reads a byte-identical
    /// viability.
    pub fn whole_body_viability_aged(
        &self,
        fns: &FunctionLawRegistry,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> Fixed {
        let mut best = Fixed::ZERO;
        for def in fns.defs() {
            for i in 0..self.segments.len() {
                let cap = self.segment_capabilities(i, fns, refs, caps).score(def.id);
                let aged = cap.checked_mul(self.segments[i].integrity()).unwrap_or(cap);
                best = best.max(aged);
            }
        }
        best
    }

    /// The whole-body EXTENSIVE maintenance demand (Joules per tick, R-AGING (c) slice 2): the total
    /// repair energy the body would spend to renew every segment's failure reserve at its own turnover
    /// rate this tick, `Σ over segments of turnover_rate * failure_tolerance`. Extensive by construction
    /// (it sums over the whole grown body), so a larger body carries a proportionally larger maintenance
    /// bill: the demand side of the funding gate whose balance against the being's own (mass-neutral)
    /// energy budget lets whatever size-longevity relation emerges be the OUTPUT, never a written law. A
    /// segment with no failure tolerance or no turnover datum contributes nothing (the absence
    /// convention).
    pub fn maintenance_demand(&self) -> Fixed {
        let mut demand = Fixed::ZERO;
        for seg in &self.segments {
            let tol = seg.failure_tolerance();
            if tol <= Fixed::ZERO {
                continue;
            }
            let term = seg
                .mat("mat.turnover_rate")
                .checked_mul(tol)
                .unwrap_or(Fixed::ZERO);
            demand = demand.saturating_add(term);
        }
        demand
    }

    /// Advance every segment's aging damage one tick (R-AGING (c) route (i), the first-passage accrual).
    /// For each segment carrying a failure tolerance: the abrasive Archard WEAR energy from the being's own
    /// locomotor load (`force`, sliding `distance`) against the segment's OWN material, commensurated from
    /// the wear law's kilojoule scale to the segment's Joule failure scale (the `* 1000` the strike path
    /// uses to compare delivered energy against `fracture_energy * crack_area`), MINUS the funded
    /// tissue-turnover REPAIR (`turnover_rate * tolerance * funded_fraction`), the net normalized ONCE by
    /// the segment's own tolerance and added to its damage fraction, floored at zero. The conversion from a
    /// turnover rate to a repaired energy and from an accumulated energy to a damage fraction both go
    /// through the segment's own failure energy, so no free per-segment constant enters. A segment with no
    /// failure tolerance is skipped (the absence convention), so a body whose tissue declares no fracture
    /// energy does not age and the run stays byte-identical. Whatever size-longevity relation emerges is
    /// the OUTPUT: no term is justified by the aging theory it encodes.
    ///
    /// MODELING ASSUMPTIONS, flagged for owner and gate review (not authored values), two of them:
    /// (1) the wear insult drives every load-bearing segment with the WHOLE-BODY locomotor force and
    /// distance, letting each segment's own material and tolerance govern how much it wears and how near
    /// failure that carries it, rather than partitioning the force across the segment graph by an authored
    /// load-path model (a per-segment load partition read from the grown mechanics is a refinement; this
    /// whole-body drive avoids authoring a distribution). (2) The ONLY aging insult wired this slice is the
    /// LOCOMOTOR-mechanical Archard wear (force from muscle, distance from ground slide), so a non-locomoting
    /// or sessile body reads zero distance, accrues no wear, and does not age through THIS insult; the
    /// thermal, chemical, and metabolic/oxidative kernels (each on its own grounded floor physics) are what
    /// will age a non-mechanical-failure or sessile being on its own terms, a future-insult gap, not a
    /// motility-longevity law authored here. A being that declares no fracture-energy tolerance never ages
    /// (the absence convention), so both cases stay clean data rows.
    #[allow(clippy::too_many_arguments)]
    pub fn accrue_aging(
        &mut self,
        force: Fixed,
        distance: Fixed,
        coefficient_scale: Fixed,
        funded_fraction: Fixed,
        wear_max: Fixed,
        energy_max: Fixed,
    ) {
        for seg in &mut self.segments {
            let tol = seg.failure_tolerance();
            if tol <= Fixed::ZERO {
                continue; // no failure reserve to age against: no aging coupling (absence convention)
            }
            // The Archard wear energy from this segment's own tribological material against the load, on the
            // floor's kilojoule cut-work scale. `laws::wear_energy` is ALREADY commensurate with the failure
            // tolerance (`fracture_energy * crack_area`, the same kJ scale, its `C_VOL` bridge lands it there
            // with no free per-insult weight), so no conversion factor is applied: wear, repair, and the
            // tolerance all sit on one currency. A segment with no defined indentation hardness has no
            // Archard wear mode (`laws::wear` would abrade a zero-hardness material without bound); the
            // absence convention skips the wear insult for it rather than reading an unset material as
            // instant destruction.
            let wear = if seg.mat("mat.indentation_hardness") > Fixed::ZERO {
                laws::wear_energy(
                    seg.mat("mat.wear_coefficient"),
                    coefficient_scale,
                    force,
                    distance,
                    seg.mat("mat.indentation_hardness"),
                    seg.mat("mat.specific_cut_energy"),
                    wear_max,
                    energy_max,
                )
            } else {
                Fixed::ZERO
            };
            let repair = insult::repair_energy(seg.mat("mat.turnover_rate"), tol, funded_fraction);
            let net = insult::net_damage_delta(wear, repair); // signed, common kJ currency
                                                              // Normalize once by the segment's own tolerance. An unrepresentable ratio (a catastrophic net
                                                              // wear against a tiny tolerance) saturates SIGN-AWARE toward full damage for a positive net and
                                                              // toward zero for a negative one, matching `derive_integrity`'s None-is-full-damage convention
                                                              // rather than silently zeroing a fragile part's fastest failure.
            let frac_delta = net.checked_div(tol).unwrap_or(if net.to_bits() >= 0 {
                Fixed::ONE
            } else {
                Fixed::ZERO
            });
            // Advance the damage fraction, clamped to [0, ONE] so the stored accumulator stays a clean
            // fraction (integrity() also clamps on read; keeping the store in range means a part driven past
            // failure and later over-funded recovers cleanly rather than climbing down from an unbounded value).
            let advanced =
                Fixed::from_bits(seg.damage.to_bits().saturating_add(frac_delta.to_bits()));
            seg.damage = advanced.clamp(Fixed::ZERO, Fixed::ONE);
        }
    }

    /// The development-weighted SUM of a grown body's tissue composition on an axis (emergent-anatomy Step 3,
    /// the metabolic-tier grow): `Σ over segments of (per-segment development · composition)`. It mirrors how
    /// [`crate::homeostasis::Homeostasis::new`] and the whole-body physiology reads sum a catalog organ set
    /// (`Σ development · composition`), but reads DIRECTLY off the grown segments with no organ kind id and no
    /// registry, the metabolic sibling of the affordance and speed direct reads. Each segment's development is
    /// a uniform fraction of a full body (`1 / MASS_REFERENCE_SEGMENTS`, the same reference the digest's
    /// `body_mass` uses, so the reserve scale and the mass scale agree). An extensive quantity (a reserve
    /// capacity, an exchange surface, a muscle's total strength) reads as this sum; a body whose grown tissue
    /// carries none of the axis reads zero (the absence convention).
    pub fn composition_sum(&self, axis_id: &str) -> Fixed {
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

    /// The mean of a grown body's tissue composition on an axis, over the grown segments (emergent-anatomy
    /// Step 3). An intensive quantity (a per-mass energy density, a specific heat, a density) reads as this
    /// mean rather than a sum; every grown segment carries the same development, so the development-weighted
    /// average is the plain mean. Zero for a body with no segments (which cannot arise, a body has a root).
    pub fn composition_mean(&self, axis_id: &str) -> Fixed {
        let n = self.segments.len();
        if n == 0 {
            return Fixed::ZERO;
        }
        let mut sum = Fixed::ZERO;
        for seg in &self.segments {
            sum = sum.saturating_add(seg.mat(axis_id));
        }
        sum.checked_div(Fixed::from_int(n as i32))
            .unwrap_or(Fixed::ZERO)
    }

    /// The metabolic reserve capacity a grown body's tissue backs on a biology-floor axis: the extensive
    /// [`Structure::composition_sum`] on that axis (energy density, water fraction), which
    /// [`crate::homeostasis::Homeostasis::from_structure`] sums into a reserve. A body whose tissue carries no
    /// energy backs no energy reserve and starves at birth through the ordinary cull.
    pub fn backed_capacity(&self, axis_id: &str) -> Fixed {
        self.composition_sum(axis_id)
    }

    /// The whole-body per-mass energy density a grown body's tissue reads (`bio.energy_density`), the specific
    /// energy the derived metabolic drain reads to bridge the reserve to stored joules: the intensive
    /// [`Structure::composition_mean`] on that axis, mirroring [`crate::physiology::whole_body_energy_density`]
    /// over a catalog organ set but read DIRECTLY off the grown tissue.
    pub fn whole_body_energy_density(&self) -> Fixed {
        self.composition_mean("bio.energy_density")
    }

    /// The whole-body composition VECTOR of a grown body (Arc 6): the per-axis composition the matter cycle
    /// and edibility read, generalizing [`Structure::composition_mean`] across every axis the grown tissue
    /// carries, exactly as [`crate::physiology::whole_body_composition_vector`] generalizes
    /// `whole_body_energy_density`/`body_density` over a catalog organ set. The axis union is taken over every
    /// segment's material map; per axis the value is the mean over the segments that CARRY it (value above
    /// zero), the plain mean because every grown segment shares the same development weight (so the
    /// development-weighted mean the catalog path uses reduces to it here). An axis no segment carries is
    /// absent (the absence convention), so a grown body with no metabolic energy or no water simply omits
    /// those axes rather than reading a Terran default, which lets a silicon or mana-fed body be a data row.
    /// The number of segments the grown body bears (Arc 6), a coarse size read for a reduced-fidelity
    /// description of a grown species (which has no named parts).
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub fn whole_body_composition_vector(&self) -> BTreeMap<String, Fixed> {
        let mut axes: BTreeSet<&str> = BTreeSet::new();
        for seg in &self.segments {
            for key in seg.material.keys() {
                axes.insert(key.as_str());
            }
        }
        let mut vector: BTreeMap<String, Fixed> = BTreeMap::new();
        for axis in axes {
            let mut sum = Fixed::ZERO;
            let mut count = 0i32;
            for seg in &self.segments {
                let v = seg.mat(axis);
                if v > Fixed::ZERO {
                    sum = sum.saturating_add(v);
                    count += 1;
                }
            }
            if count > 0 {
                let mean = sum
                    .checked_div(Fixed::from_int(count))
                    .unwrap_or(Fixed::ZERO);
                if mean > Fixed::ZERO {
                    vector.insert(axis.to_string(), mean);
                }
            }
        }
        vector
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
/// this evolves the body's shape exactly as the controller weights evolve. The morphogen block sits at the
/// FRONT of the gene set (no prefix loci); use [`morphogen_gene_set_with_prefix`] when the pool carries other
/// loci ahead of the morphogen block.
pub fn morphogen_gene_set(program: &MorphogenProgram) -> GeneSet {
    morphogen_gene_set_with_prefix(0, program)
}

/// A morphogen gene set whose parameter loci sit AFTER `prefix_loci` other loci (Arc 6): the gene set is
/// `prefix_loci` no-effect placeholder genes followed by one unit-weight [`Channel::Morphogen`] gene per
/// growth parameter. Because [`GeneSet::express`] indexes alleles by Vec POSITION (not gene id), a pool that
/// carries niche or other bookkeeping loci before its morphogen block must express through this prefixed set
/// so each parameter reads its OWN locus; the placeholder genes carry no channel effect, so express skips
/// them and they contribute nothing. This is the index-alignment fix the biosphere generator (whose species
/// pool carries `p.loci` niche loci ahead of the morphogen block) needs to avoid silently reading the wrong
/// locus for every parameter.
pub fn morphogen_gene_set_with_prefix(prefix_loci: usize, program: &MorphogenProgram) -> GeneSet {
    let mut genes: Vec<GeneDef> = Vec::with_capacity(prefix_loci + program.param_count());
    for k in 0..prefix_loci {
        genes.push(GeneDef {
            id: GeneId(k as u32),
            effects: Vec::new(),
            dominance: DominanceMode::additive(),
        });
    }
    for k in 0..program.param_count() {
        genes.push(GeneDef {
            id: GeneId((prefix_loci + k) as u32),
            effects: vec![GeneEffect {
                channel: Channel::Morphogen(MorphogenParamId(k as u32)),
                weight: Fixed::ONE,
            }],
            dominance: DominanceMode::additive(),
        });
    }
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
    // The actuator axes (the stroke-rate substrate): grown into the same geometry map, so a grown body's
    // cross-section and stroke are its own per-segment data and its `F d` blow is non-zero.
    for (i, spec) in program.actuator_axes.iter().enumerate() {
        let f = frac(params, program.actuator_root_param(i));
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
    for (i, spec) in program.composition_axes.iter().enumerate() {
        let f = frac(params, program.composition_param(i));
        material.insert(spec.axis.clone(), map_range(f, spec.lo, spec.hi));
    }
    let mut segments = vec![Segment {
        parent: None,
        depth: 0,
        geometry,
        material,
        damage: Fixed::ZERO,
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
    // The actuator axes scale from the parent exactly as the geometry axes, by their own growth fraction and a
    // distinct jitter stream (offset past the geometry axes so the draws never collide), so a child's stroke and
    // cross-section stay heritable-plus-individual and deterministic.
    for (i, spec) in program.actuator_axes.iter().enumerate() {
        let f = frac(params, program.actuator_growth_param(i));
        let growth = GROWTH_LO + f.mul(GROWTH_HI - GROWTH_LO);
        let locus = ((parent_idx as u64) << 8) | child as u64;
        let roll = DrawKey::pair(id.0, locus, gen as u64, Phase::MORPHOGEN)
            .slot(SLOT_JITTER)
            .rng(seed)
            .unit_fixed((program.geometry_axes.len() + i) as u64);
        let jitter = Fixed::ONE + (roll - HALF).mul(Fixed::from_int(2)).mul(JITTER_MAG);
        let raw = parent.geo(&spec.axis).mul(growth).mul(jitter);
        geometry.insert(spec.axis.clone(), raw.clamp(spec.lo, spec.hi));
    }
    Segment {
        parent: Some(parent_idx),
        depth: gen + 1,
        geometry,
        material: parent.material.clone(),
        damage: Fixed::ZERO,
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
    use crate::homeostasis::{AffordanceRegistry, Homeostasis, HomeostaticRegistry, MOVE, STRIKE};
    use crate::locomotion::{locomotion_speed_structure, LocomotionParams};
    use civsim_bio::genome::{Allele, Haplotype, SchemeId};

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
    fn the_param_layout_is_a_nonoverlapping_partition_of_param_count() {
        // The determinism guard for the whole positional param layout (root/growth geometry, material, branch,
        // spawn, composition, and the stroke-rate actuator roots and growths): every accessor must map to a
        // DISTINCT index, and together they must cover exactly `[0, param_count())` with no gap and no overlap.
        // This breaks immediately on the exact index regressions the append risks: a dropped offset (two
        // categories collide), an off-by-one (a gap or an overlap), or `actuator_growth_param` losing its
        // `+ actuator_axes.len()` term so a root and its growth share an index. A functional grow test can be
        // blind to these when the colliding loci happen to carry the same seed, so this asserts the arithmetic
        // itself, not an outcome. It runs over a program with a DIFFERENT axis count per category so no
        // coincidental equality hides a swapped `.len()`.
        let mut program = MorphogenProgram::dev_default();
        // Perturb the per-category counts so they are pairwise distinct (2 geometry, 3 material, 4 composition,
        // 1 actuator): a formula that read the wrong category's length would then land on a wrong index.
        program.geometry_axes.truncate(2);
        program.material_axes.truncate(3);
        program.composition_axes.truncate(4);
        program.actuator_axes.truncate(1);

        let mut indices = Vec::new();
        for i in 0..program.geometry_axes.len() {
            indices.push(program.root_geo_param(i));
            indices.push(program.growth_param(i));
        }
        for i in 0..program.material_axes.len() {
            indices.push(program.material_param(i));
        }
        indices.push(program.branch_param());
        indices.push(program.spawn_param());
        for i in 0..program.composition_axes.len() {
            indices.push(program.composition_param(i));
        }
        for i in 0..program.actuator_axes.len() {
            indices.push(program.actuator_root_param(i));
            indices.push(program.actuator_growth_param(i));
        }

        assert_eq!(
            indices.len(),
            program.param_count(),
            "every parameter is addressed exactly once, so the accessors account for the whole count"
        );
        indices.sort_unstable();
        let expected: Vec<usize> = (0..program.param_count()).collect();
        assert_eq!(
            indices, expected,
            "the accessors are a bijection onto [0, param_count()): no collision (a dropped offset), no gap or \
             overlap (an off-by-one), and no root/growth aliasing"
        );
    }

    #[test]
    fn a_grown_body_grows_its_actuator_axes_and_reads_a_positive_impact_blow() {
        // Stroke-rate step-1b, the payoff and its coverage: a grown body carries its own
        // `mech.cross_section_area` and `mech.stroke_length` on every segment (root AND children, so the
        // grow_child actuator branch and its distinct jitter stream are exercised), and reads a POSITIVE
        // IMPACT capability, the `F d` actuator-work blow the strike delivers, derived from its own grown
        // actuator geometry. Before step-1b a grown body carried no such geometry and delivered `F d = 0`.
        // A body whose program declares NO actuator axes (an empty data list, the alien with no actuator)
        // grows no cross-section or stroke and reads a zero blow: the actuator is a data row and the blow
        // emerges from it, never authored. This test breaks on an actuator-index regression (a wrong
        // `actuator_root_param`/`actuator_growth_param` grows the wrong axis, so IMPACT reads zero) that the
        // determinism and composition tests, which only check run-to-run equality and the composition block,
        // would pass blind.
        let program = MorphogenProgram::dev_default();
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = caps();

        // Seed the actuating strength (the muscle tissue, composition index 1), both actuator roots high, a
        // per-generation actuator growth, and a branch plus spawn so a child grows. Address every locus through
        // the program's own accessors so the seed cannot drift if the layout grows again.
        let seeded = grow(
            &program,
            &params(
                &program,
                &[
                    (program.composition_param(1), "1"), // mat.fracture_strength (MUSCLE_STRENGTH)
                    (program.actuator_root_param(0), "1"), // mech.cross_section_area root
                    (program.actuator_root_param(1), "1"), // mech.stroke_length root
                    (program.actuator_growth_param(0), "0.8"),
                    (program.actuator_growth_param(1), "0.8"),
                    (program.branch_param(), "1"),
                    (program.spawn_param(), "1"),
                ],
            ),
            0x1,
            StableId(1),
        );
        // The root grows both actuator axes positive from its own genome.
        let root = &seeded.segments[0];
        assert!(
            root.geo("mech.cross_section_area") > Fixed::ZERO,
            "the root grows a load-bearing cross-section"
        );
        assert!(
            root.geo("mech.stroke_length") > Fixed::ZERO,
            "and a stroke length"
        );
        // A child grows them too (the grow_child actuator branch, scaled from the parent).
        let child = seeded
            .segments
            .iter()
            .find(|s| s.parent.is_some())
            .expect("a child grew so the grow_child actuator path is exercised");
        assert!(
            child.geo("mech.cross_section_area") > Fixed::ZERO,
            "a child inherits and scales its cross-section"
        );
        assert!(
            child.geo("mech.stroke_length") > Fixed::ZERO,
            "and its stroke length"
        );
        // The payoff: a grown body reads a POSITIVE IMPACT (the `F d` blow), the whole point of step-1b.
        assert!(
            seeded.max_capability(FunctionLawRegistry::ID_IMPACT, &fns, &refs, &caps) > Fixed::ZERO,
            "a grown body delivers a non-zero actuator-work blow from its own grown cross-section and stroke"
        );

        // The alien with no actuator: the same strength tissue but an EMPTY actuator-axis list, so no
        // cross-section or stroke is grown and the blow is zero. The actuator is a data row.
        let mut no_actuator = MorphogenProgram::dev_default();
        no_actuator.actuator_axes = Vec::new();
        let unarmed = grow(
            &no_actuator,
            &params(&no_actuator, &[(no_actuator.composition_param(1), "1")]),
            0x1,
            StableId(1),
        );
        let unarmed_root = &unarmed.segments[0];
        assert_eq!(
            unarmed_root.geo("mech.cross_section_area"),
            Fixed::ZERO,
            "a body whose program declares no actuator grows no cross-section"
        );
        assert_eq!(
            unarmed_root.geo("mech.stroke_length"),
            Fixed::ZERO,
            "and no stroke length"
        );
        assert_eq!(
            unarmed.max_capability(FunctionLawRegistry::ID_IMPACT, &fns, &refs, &caps),
            Fixed::ZERO,
            "and so it delivers no blow: the actuator is a data row the blow emerges from"
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
    fn segment_failure_tolerance_and_integrity_absence_and_damage() {
        // R-AGING (c): a segment's failure tolerance is the Griffith product of its own fracture energy
        // and cross-sectional area, and its integrity is one minus its accumulated damage fraction. The
        // absence convention is load-bearing: a segment with no fracture-energy datum has no aging
        // coupling and reads full integrity whatever its stored value (NOT a failed part), so aging is
        // opt-in and byte-neutral until a race grows tissue that declares a fracture energy.
        let seg = |fe: Option<Fixed>, area: Fixed, damage: Fixed| {
            let mut material = BTreeMap::new();
            if let Some(fe) = fe {
                material.insert("mat.fracture_energy".to_string(), fe);
            }
            let mut geometry = BTreeMap::new();
            geometry.insert("mech.contact_area".to_string(), area);
            Segment {
                parent: None,
                depth: 0,
                geometry,
                material,
                damage,
            }
        };
        // No fracture-energy datum: zero tolerance, full integrity even with a stored damage value.
        let bare = seg(None, Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2));
        assert_eq!(bare.failure_tolerance(), Fixed::ZERO);
        assert_eq!(
            bare.integrity(),
            Fixed::ONE,
            "no tolerance means no aging coupling, not a failed part"
        );
        // A tolerance-bearing segment: fracture_energy 2 times area 0.5 gives tolerance 1.0.
        let armed = seg(
            Some(Fixed::from_int(2)),
            Fixed::from_ratio(1, 2),
            Fixed::ZERO,
        );
        assert_eq!(armed.failure_tolerance(), Fixed::ONE);
        assert_eq!(
            armed.integrity(),
            Fixed::ONE,
            "zero damage reads full integrity"
        );
        let worn = seg(
            Some(Fixed::from_int(2)),
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 4),
        );
        assert_eq!(
            worn.integrity(),
            Fixed::from_ratio(3, 4),
            "integrity is one minus the damage fraction"
        );
        let dead = seg(
            Some(Fixed::from_int(2)),
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(3, 2),
        );
        assert_eq!(
            dead.integrity(),
            Fixed::ZERO,
            "damage past the tolerance clamps integrity to zero"
        );
    }

    #[test]
    fn aged_viability_is_identical_at_zero_damage_and_falls_to_the_floor_when_worn() {
        // R-AGING (c) route (i): the aged viability equals the un-aged viability when no segment has
        // accrued damage (the byte-neutrality guarantee: a full-integrity segment scales its capability
        // by an exact ONE), and a body worn to full damage reads a zero viability and dies through the
        // same reserve-floor cull as inert matter, with no vital-part predicate.
        let program = MorphogenProgram::dev_default();
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = caps();
        let mut limb = grow(
            &program,
            &params(&program, &[(0, "1"), (1, "0.5"), (2, "0.4"), (9, "0.75")]),
            0x1,
            StableId(1),
        );
        let v = limb.whole_body_viability(&fns, &refs, &caps);
        assert!(v > Fixed::ZERO);
        assert_eq!(
            limb.whole_body_viability_aged(&fns, &refs, &caps),
            v,
            "aged viability is byte-identical to un-aged at zero damage"
        );
        // Arm aging on every segment (a positive tolerance) and wear each to full damage.
        for s in &mut limb.segments {
            s.material
                .insert("mat.fracture_energy".to_string(), Fixed::from_int(2));
            s.geometry
                .entry("mech.contact_area".to_string())
                .or_insert(Fixed::from_ratio(1, 2));
            s.damage = Fixed::ONE;
        }
        assert_eq!(
            limb.whole_body_viability_aged(&fns, &refs, &caps),
            Fixed::ZERO,
            "a fully-worn body reads no viable function and falls through the cull"
        );
    }

    #[test]
    fn aging_accrual_wears_a_loaded_body_and_repair_offsets_it() {
        // R-AGING (c) route (i), the accrual: a load-bearing segment carrying a failure tolerance and a
        // wear material accrues damage under a locomotor load with no funded repair (wear with nothing to
        // offset it), so its integrity falls; a segment whose funded turnover repair meets or exceeds the
        // wear accrues no net damage (the first-passage balance); and a segment with no fracture-energy
        // tolerance is skipped entirely (the absence convention, the byte-neutrality guarantee).
        let make = |fe: Option<Fixed>, turnover: Fixed| {
            let mut geometry = BTreeMap::new();
            geometry.insert("mech.contact_area".to_string(), Fixed::from_ratio(1, 2));
            let mut material = BTreeMap::new();
            if let Some(fe) = fe {
                material.insert("mat.fracture_energy".to_string(), fe);
            }
            material.insert("mat.wear_coefficient".to_string(), Fixed::ONE);
            material.insert("mat.indentation_hardness".to_string(), Fixed::ONE);
            material.insert("mat.specific_cut_energy".to_string(), Fixed::ONE);
            material.insert("mat.turnover_rate".to_string(), turnover);
            Segment {
                parent: None,
                depth: 0,
                geometry,
                material,
                damage: Fixed::ZERO,
            }
        };
        let force = Fixed::from_int(10);
        let distance = Fixed::from_ratio(1, 10);
        let scale = Fixed::ONE;
        let wear_max = Fixed::from_int(1_000_000);
        let energy_max = Fixed::from_int(1_000_000_000);

        // Wear with no funded repair: damage rises, integrity falls.
        let mut wearing = Structure {
            segments: vec![make(Some(Fixed::from_int(2)), Fixed::ZERO)],
        };
        wearing.accrue_aging(force, distance, scale, Fixed::ZERO, wear_max, energy_max);
        assert!(
            wearing.segments[0].damage > Fixed::ZERO,
            "a loaded body wears"
        );
        assert!(
            wearing.segments[0].integrity() < Fixed::ONE,
            "wear lowers derived integrity"
        );

        // A tolerance-less segment is skipped: no accrual, byte-neutral.
        let mut inert = Structure {
            segments: vec![make(None, Fixed::ZERO)],
        };
        inert.accrue_aging(force, distance, scale, Fixed::ZERO, wear_max, energy_max);
        assert_eq!(
            inert.segments[0].damage,
            Fixed::ZERO,
            "no fracture-energy tolerance means no aging coupling"
        );

        // Funded turnover repair OFFSETS the wear: the same load accrues strictly less net damage when
        // the tissue funds its own maintenance than when it does not (repair is capped at the whole
        // failure reserve per tick, so a heavy single-tick load is not fully cancelled, which is correct:
        // a catastrophic load destroys regardless of repair).
        let mut maintained = Structure {
            segments: vec![make(Some(Fixed::from_int(2)), Fixed::from_int(1_000_000))],
        };
        maintained.accrue_aging(force, distance, scale, Fixed::ONE, wear_max, energy_max);
        assert!(
            maintained.segments[0].damage < wearing.segments[0].damage,
            "funded repair accrues strictly less damage than the unrepaired body under the same load"
        );

        // The extensive maintenance demand sums turnover * tolerance over the segments.
        let demand_body = Structure {
            segments: vec![make(Some(Fixed::from_int(2)), Fixed::from_ratio(1, 4))],
        };
        // tolerance = 2 * 0.5 = 1.0; demand = turnover(0.25) * tolerance(1.0) = 0.25.
        assert_eq!(demand_body.maintenance_demand(), Fixed::from_ratio(1, 4));
    }

    #[test]
    fn aging_overflow_saturates_toward_failure_not_immortality() {
        // R-AGING (c) correctness (audit finding): a catastrophic net wear against a tiny failure tolerance
        // makes the normalizing division unrepresentable; it must saturate SIGN-AWARE toward FULL damage
        // (the fragile part fails fastest), matching derive_integrity's convention, never silently zero the
        // accrual and make the most-worn part immortal by a fixed-point range accident.
        let mut geometry = BTreeMap::new();
        geometry.insert("mech.contact_area".to_string(), Fixed::from_ratio(1, 10));
        let mut material = BTreeMap::new();
        // A small (but representable) tolerance against a heavy load, a large wear coefficient, and a soft
        // material, so the wear energy saturates and wear / tolerance overflows the representable range.
        material.insert("mat.fracture_energy".to_string(), Fixed::ONE);
        material.insert("mat.wear_coefficient".to_string(), Fixed::from_int(1000));
        material.insert(
            "mat.indentation_hardness".to_string(),
            Fixed::from_ratio(1, 1000),
        );
        material.insert("mat.specific_cut_energy".to_string(), Fixed::from_int(1000));
        material.insert("mat.turnover_rate".to_string(), Fixed::ZERO);
        let mut s = Structure {
            segments: vec![Segment {
                parent: None,
                depth: 0,
                geometry,
                material,
                damage: Fixed::ZERO,
            }],
        };
        assert!(
            s.segments[0].failure_tolerance() > Fixed::ZERO,
            "the segment is aging-armed"
        );
        s.accrue_aging(
            Fixed::from_int(1000),
            Fixed::from_int(1000),
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::from_int(1_000_000),
            Fixed::from_int(1_000_000_000),
        );
        assert_eq!(
            s.segments[0].integrity(),
            Fixed::ZERO,
            "an overwhelming wear against a tiny tolerance reaches first passage, not immortality"
        );
    }

    #[test]
    fn aging_reaches_first_passage_and_the_longevity_relation_is_an_output() {
        // R-AGING (c) FUNCTIONAL CHECK: a loaded body accrues damage tick by tick until its damage
        // crosses its own failure tolerance (first passage: integrity reaches zero), and a body with a
        // greater failure tolerance takes more ticks to reach it. What this checks precisely: the
        // first-passage TIME is a pure OUTPUT of the tolerance-versus-wear balance (monotone in the
        // segment's own tolerance under a fixed load), with no authored lifespan or size-to-duration law in
        // the accrual; the time falls out of the arithmetic. It does NOT exercise a full size-covariance (a
        // body scaling area and load together), which the gate's functional check on a grown race covers;
        // here only the tolerance varies. The magnitudes are test fixtures spacing the passage over many
        // ticks; the real per-tissue wear and tolerance are the owner's reserved data.
        let run_to_first_passage = |fracture_energy: Fixed| -> u32 {
            let mut geometry = BTreeMap::new();
            geometry.insert("mech.contact_area".to_string(), Fixed::ONE);
            let mut material = BTreeMap::new();
            material.insert("mat.fracture_energy".to_string(), fracture_energy);
            material.insert("mat.wear_coefficient".to_string(), Fixed::ONE);
            material.insert("mat.indentation_hardness".to_string(), Fixed::ONE);
            material.insert("mat.specific_cut_energy".to_string(), Fixed::ONE);
            material.insert("mat.turnover_rate".to_string(), Fixed::ZERO);
            let mut s = Structure {
                segments: vec![Segment {
                    parent: None,
                    depth: 0,
                    geometry,
                    material,
                    damage: Fixed::ZERO,
                }],
            };
            let mut ticks = 0u32;
            while s.segments[0].integrity() > Fixed::ZERO && ticks < 500_000 {
                s.accrue_aging(
                    Fixed::from_int(10),
                    Fixed::from_int(10),
                    Fixed::ONE,
                    Fixed::ZERO,
                    Fixed::from_int(1_000_000),
                    Fixed::from_int(1_000_000_000),
                );
                ticks += 1;
            }
            ticks
        };
        let short = run_to_first_passage(Fixed::from_int(1_000_000));
        let long = run_to_first_passage(Fixed::from_int(2_000_000));
        assert!(
            short > 1 && short < 500_000,
            "a loaded body reaches first passage over many ticks (got {short})"
        );
        assert!(
            long > short,
            "a tougher body lives longer: the longevity relation is an emergent output, never authored \
             (short {short}, long {long})"
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
        // The bio parameters sit after spawn: energy_density then water_fraction are the last two composition
        // axes (addressed via `composition_param`, since the actuator params now follow composition).
        let energy = program.composition_param(program.composition_axes.len() - 2);
        let water = program.composition_param(program.composition_axes.len() - 1);

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
    fn a_grown_body_sums_and_averages_its_physiology_composition() {
        // Emergent-anatomy Step 3, the derived-physiology grow: a grown body reads its exchange surface and
        // muscle strength as an extensive SUM over its tissue (`composition_sum`) and its specific heat as an
        // intensive MEAN (`composition_mean`), directly off the grown segments with no organ kind id, exactly
        // as the reserve capacity and energy density do. A body whose tissue carries none reads zero.
        let program = MorphogenProgram::dev_default();
        // The physiology composition params lead the composition block (convective surface, fracture
        // strength, specific heat), before the energy density and water fraction the reserves back.
        // Addressed via `composition_param` (not `param_count()`-relative), since the actuator params now
        // follow the composition block and the physiology axes lead it.
        let surface = program.composition_param(0);
        let fracture = program.composition_param(1);
        let specific_heat = program.composition_param(2);
        let grown = grow(
            &program,
            &params(
                &program,
                &[(surface, "0.5"), (fracture, "0.5"), (specific_heat, "0.5")],
            ),
            0x1,
            StableId(1),
        );
        assert!(
            grown.composition_sum("bio.convective_surface") > Fixed::ZERO,
            "a grown body sums its exchange surface off its tissue"
        );
        assert!(
            grown.composition_sum("mat.fracture_strength") > Fixed::ZERO,
            "and its muscle strength"
        );
        assert!(
            grown.composition_mean("therm.specific_heat") > Fixed::ZERO,
            "and averages its specific heat"
        );

        let bare = grow(
            &program,
            &vec![Fixed::ZERO; program.param_count()],
            0x1,
            StableId(1),
        );
        assert_eq!(
            bare.composition_sum("bio.convective_surface"),
            Fixed::ZERO,
            "a body with no such tissue reads zero"
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
