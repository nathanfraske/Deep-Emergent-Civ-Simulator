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

//! Value profiles and the distance between them (design Part 21, the resolved R-VALUE-METRIC
//! work, record 62.3).
//!
//! There is no alignment axis and no evil flag. A race or culture carries a value profile
//! over the world's data-defined value axes, and how opposed two profiles are is a structural
//! distance, because value axes are not independent: some are near-synonyms and some are
//! opposites, and different races organize value space differently (a ring, a set of
//! foundations, a hierarchy, a lattice). The structure is therefore per-race data and the
//! distance respects it; the mechanism is fixed Rust and the topology is chosen by the data
//! (Principle 11).
//!
//! This brick builds the parts the record pins exactly:
//!
//! - the per-race [`ValueStructure`] (independent axes, a weighted graph, or a relationship
//!   matrix, with a ring or tree a special graph);
//! - the offline [`GroundMetric`] compiler, an exact integer all-pairs shortest-path table
//!   over the structure (Floyd-Warshall over integer edge weights), so the runtime is table
//!   lookups and determinism is automatic;
//! - the pinned independent-axes distance, plain Euclidean over the shared axes, which is
//!   what the structural distance reduces to when axes are independent, computed with the
//!   deterministic [`Fixed::sqrt`];
//! - the cross-race plumbing: a shared [`EticSubstrate`], per-race [`EmicProjection`]s onto
//!   it, and a [`cross_race_distance`] that projects both profiles and folds the measured
//!   projection loss (the squared stance of every untranslatable emic axis, from the reusable
//!   [`project_to_etic_with_loss`]) into the sum-of-squares before its single root, so the
//!   per-pair distance rises by that pair's own incommensurability rather than a flat floor,
//!   treating an untranslatable value as a theory-of-mind blind spot.
//!
//! The one part deliberately left as a flagged seam (the owner's call): the exact weighted
//! distance for a structured (graph or relationship) space. Record 62.3 pins the ground
//! metric and the independent reduction, but reserves the structured weighted form as a
//! design choice and lists "the weighted form and the transport form agree where both apply"
//! as still to be proven, so its closed algebra is not pinned. Rather than fabricate it,
//! [`value_distance`] returns `None` for a structured metric, so no caller runs on an
//! invented metric; the [`GroundMetric`] it would consume is built and proven here, ready
//! for that formula when it is settled. The candidate forms are a tree or graph Wasserstein
//! (Le et al., NeurIPS 2019) and a Mahalanobis quadratic over a positive-semidefinite
//! relationship matrix; the latter is the family that reduces to Euclidean for the identity.
//!
//! The numeric calibrations (the default compatibility and opposition weights, the
//! distance-to-pressure coefficient, and which axes populate the etic substrate) are reserved
//! owner values (the `value_metric.*` entries of the calibration manifest), supplied as data,
//! never invented here. The cross-race incommensurability is no longer a reserved flat floor:
//! it is measured per race-pair as the etic projection loss ([`cross_race_distance`] through
//! [`project_to_etic_with_loss`]), so `value_metric.incommensurability_floor` is retired as a
//! constant in favour of that derived quantity.

use std::collections::BTreeMap;

use civsim_core::Fixed;

/// A data-defined value-axis identifier (the emic axes a race carries, Part 40).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ValueAxisId(pub u32);

/// A data-defined shared-substrate axis identifier (the etic axes cross-race comparison
/// passes through, Part 40).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EticAxisId(pub u32);

/// A data-defined race identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RaceId(pub u32);

/// A value profile: a stance, a small signed magnitude, on each value axis the holder has an
/// opinion on. Kept in a sorted map so any canonical walk is deterministic and a partial
/// profile (an agent who knows only some of another's values, Part 37) is a profile with
/// fewer axes. The same type is used on a culture, a deity, and inside a mental model as a
/// believed profile, which is why the distance over it is a pure function (it runs on a
/// believed profile exactly as on a true one).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ValueProfile {
    axes: BTreeMap<ValueAxisId, i8>,
}

impl ValueProfile {
    /// An empty profile (no stance on any axis).
    pub fn new() -> Self {
        ValueProfile {
            axes: BTreeMap::new(),
        }
    }

    /// A profile from axis-stance pairs.
    pub fn with(pairs: impl IntoIterator<Item = (ValueAxisId, i8)>) -> Self {
        ValueProfile {
            axes: pairs.into_iter().collect(),
        }
    }

    /// Set the stance on an axis.
    pub fn set(&mut self, axis: ValueAxisId, stance: i8) {
        self.axes.insert(axis, stance);
    }

    /// The stance on an axis, or `None` if the holder has no opinion on it.
    pub fn get(&self, axis: ValueAxisId) -> Option<i8> {
        self.axes.get(&axis).copied()
    }

    /// The axes the holder has a stance on, in canonical order.
    pub fn axes(&self) -> impl Iterator<Item = (ValueAxisId, i8)> + '_ {
        self.axes.iter().map(|(&a, &s)| (a, s))
    }

    /// How many axes the holder has a stance on.
    pub fn len(&self) -> usize {
        self.axes.len()
    }

    /// Whether the holder has no stance on any axis.
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }
}

/// One weighted edge of a value-structure graph: two axis indices and the integer cost
/// between them. Undirected. A ring is a cycle of such edges, a tree an acyclic set; the
/// compiler treats both as graphs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GraphEdge {
    /// The first axis index (in `0..k`).
    pub a: u32,
    /// The second axis index (in `0..k`).
    pub b: u32,
    /// The integer edge cost (a small non-negative weight; near-synonym axes sit close,
    /// opposing axes far). Must fit `i32`.
    pub weight: u32,
}

/// The per-race value-space structure (design Part 21). The human value circle is one
/// structure among many, so the structure is data and the engine fixes only how distance is
/// read over it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ValueStructure {
    /// Axes are orthogonal; the distance reduces to plain Euclidean over the shared axes.
    Independent {
        /// The number of axes.
        k: usize,
    },
    /// A weighted graph over the axes (a ring or tree is a special graph). The ground metric
    /// is the all-pairs shortest paths over its integer edge costs.
    Graph {
        /// The number of axes.
        k: usize,
        /// The weighted edges.
        edges: Vec<GraphEdge>,
    },
    /// A relationship matrix: a `k*k` row-major positive-semidefinite weight matrix of
    /// compatible and opposing axis pairs. Carried as data; the quadratic-form distance over
    /// it is part of the flagged structured-distance seam.
    Relationship {
        /// The number of axes.
        k: usize,
        /// The `k*k` row-major weight matrix.
        matrix: Vec<Fixed>,
    },
}

impl ValueStructure {
    /// The number of axes the structure spans.
    pub fn k(&self) -> usize {
        match self {
            ValueStructure::Independent { k }
            | ValueStructure::Graph { k, .. }
            | ValueStructure::Relationship { k, .. } => *k,
        }
    }
}

/// Which structure a [`GroundMetric`] was compiled from, so the distance dispatches to the
/// pinned independent form or the flagged structured seam without re-reading the structure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StructureKind {
    /// Independent axes (the pinned Euclidean form).
    Independent,
    /// A weighted graph (structured seam).
    Graph,
    /// A relationship matrix (structured seam).
    Relationship,
}

/// The compiled ground metric: the exact integer structural distance between every pair of
/// axes, `dist[i*k + j]` (design Part 21). Compiled offline from a [`ValueStructure`] so the
/// runtime is table lookups. For a graph it is the all-pairs shortest paths over the integer
/// edge costs (exact, since shortest paths over integer weights are integers); for a
/// relationship matrix it carries the matrix; for independent axes it is the trivial metric
/// (zero on the diagonal, [`GroundMetric::UNREACHABLE`] off it, since orthogonal axes do not
/// connect). Determinism is automatic: the compile is a fixed-order integer computation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GroundMetric {
    kind: StructureKind,
    k: usize,
    dist: Box<[Fixed]>,
}

impl GroundMetric {
    /// The sentinel for a pair of axes with no path between them (orthogonal or disconnected).
    pub const UNREACHABLE: Fixed = Fixed::MAX;

    /// Compile a structure into its ground metric.
    pub fn compile(structure: &ValueStructure) -> GroundMetric {
        match structure {
            ValueStructure::Independent { k } => GroundMetric::independent(*k),
            ValueStructure::Graph { k, edges } => GroundMetric::from_graph(*k, edges),
            ValueStructure::Relationship { k, matrix } => GroundMetric {
                kind: StructureKind::Relationship,
                k: *k,
                dist: matrix.clone().into_boxed_slice(),
            },
        }
    }

    /// The trivial metric for independent axes: zero on the diagonal, unreachable off it.
    fn independent(k: usize) -> GroundMetric {
        let mut dist = vec![GroundMetric::UNREACHABLE; k * k];
        for i in 0..k {
            dist[i * k + i] = Fixed::ZERO;
        }
        GroundMetric {
            kind: StructureKind::Independent,
            k,
            dist: dist.into_boxed_slice(),
        }
    }

    /// The all-pairs shortest-path metric over a weighted graph (Floyd-Warshall). Exact
    /// integers, fixed order, so it is bit-identical on every machine.
    fn from_graph(k: usize, edges: &[GraphEdge]) -> GroundMetric {
        let mut dist = vec![GroundMetric::UNREACHABLE; k * k];
        for i in 0..k {
            dist[i * k + i] = Fixed::ZERO;
        }
        for e in edges {
            let (a, b) = (e.a as usize, e.b as usize);
            if a >= k || b >= k {
                continue;
            }
            let w = Fixed::from_int(e.weight.min(i32::MAX as u32) as i32);
            // Keep the lightest parallel edge.
            if w < dist[a * k + b] {
                dist[a * k + b] = w;
                dist[b * k + a] = w;
            }
        }
        for via in 0..k {
            for i in 0..k {
                let d_iv = dist[i * k + via];
                if d_iv == GroundMetric::UNREACHABLE {
                    continue;
                }
                for j in 0..k {
                    let d_vj = dist[via * k + j];
                    if d_vj == GroundMetric::UNREACHABLE {
                        continue;
                    }
                    let through = d_iv + d_vj;
                    if through < dist[i * k + j] {
                        dist[i * k + j] = through;
                    }
                }
            }
        }
        GroundMetric {
            kind: StructureKind::Graph,
            k,
            dist: dist.into_boxed_slice(),
        }
    }

    /// The number of axes.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Which structure this was compiled from.
    pub fn kind(&self) -> StructureKind {
        self.kind
    }

    /// The structural distance between axes `i` and `j`, or [`GroundMetric::UNREACHABLE`] if
    /// they do not connect. Out-of-range indices return unreachable.
    pub fn between(&self, i: usize, j: usize) -> Fixed {
        if i >= self.k || j >= self.k {
            return GroundMetric::UNREACHABLE;
        }
        self.dist[i * self.k + j]
    }

    /// The grid's separability floor: the smallest positive finite structural distance the metric
    /// resolves, the granularity below which two positions are not held apart (design Part 21, the
    /// ground-metric granularity the semantic-substrate quantization keys off, Part 33.1). For a
    /// graph metric it is the lightest positive off-diagonal distance in the compiled table (the
    /// finest hop the integer edge weights lay down); for the independent (orthogonal) metric, whose
    /// distinct axes do not connect, it is [`Fixed::ONE`], the unit step the integer weights sit on,
    /// so a quantization taken consistent with it never collapses to zero. A pure function of the
    /// compiled table, so it is bit-identical on every machine.
    pub fn resolution(&self) -> Fixed {
        let mut best: Option<Fixed> = None;
        for i in 0..self.k {
            for j in 0..self.k {
                if i == j {
                    continue;
                }
                let d = self.dist[i * self.k + j];
                if d > Fixed::ZERO && d != GroundMetric::UNREACHABLE {
                    best = Some(match best {
                        Some(b) => b.min(d),
                        None => d,
                    });
                }
            }
        }
        best.unwrap_or(Fixed::ONE)
    }
}

/// The pinned independent-axes distance: plain Euclidean over the axes both profiles have a
/// stance on (design Part 21, record 62.3). Partial profiles are handled by summing over the
/// shared axes, the primary runtime path. A pure function of the two profiles, so it runs on
/// a believed profile inside a mental model exactly as on a true one.
pub fn euclidean_distance(a: &ValueProfile, b: &ValueProfile) -> Fixed {
    let mut acc = Fixed::ZERO;
    for (axis, av) in a.axes() {
        if let Some(bv) = b.get(axis) {
            let d = Fixed::from_int(av as i32 - bv as i32);
            acc += d.mul(d);
        }
    }
    acc.sqrt()
}

/// The structure-aware distance between two value profiles. Pinned for the independent
/// structure (it reduces to [`euclidean_distance`]); for a structured metric (graph or
/// relationship) the closed weighted form is a flagged seam (record 62.3 reserves it as a
/// design choice), so this returns `None` until that formula is settled rather than running
/// on a fabricated metric. The compiled `g` is the precomputation the structured form will
/// consume.
pub fn value_distance(a: &ValueProfile, b: &ValueProfile, g: &GroundMetric) -> Option<Fixed> {
    match g.kind {
        StructureKind::Independent => Some(euclidean_distance(a, b)),
        StructureKind::Graph | StructureKind::Relationship => None,
    }
}

/// The shared substrate cross-race comparison passes through (design Part 21): two races'
/// emic axes are not directly comparable, so each race projects its profile onto these
/// common axes and distance is read there.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct EticSubstrate {
    /// The substrate axes, in canonical order.
    pub axes: Vec<EticAxisId>,
}

/// How one emic value axis projects onto the etic substrate: a weight per substrate axis it
/// contributes to. An emic axis with an empty projection is untranslatable, which is also a
/// theory-of-mind blind spot (an agent cannot model a value it has no axis for).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct EmicProjection {
    /// The substrate axes this emic axis feeds, with weights.
    pub onto: Vec<(EticAxisId, Fixed)>,
}

/// A race's projection of its whole value space onto the etic substrate: one
/// [`EmicProjection`] per emic value axis.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RaceProjection {
    /// Per emic axis, how it projects onto the substrate.
    pub per_axis: BTreeMap<ValueAxisId, EmicProjection>,
}

/// A profile expressed on the etic substrate: a magnitude per substrate axis.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct EticProfile {
    axes: BTreeMap<EticAxisId, Fixed>,
}

impl EticProfile {
    /// The magnitude on a substrate axis, or `None`.
    pub fn get(&self, axis: EticAxisId) -> Option<Fixed> {
        self.axes.get(&axis).copied()
    }

    /// The substrate axes carried, in canonical order.
    pub fn axes(&self) -> impl Iterator<Item = (EticAxisId, Fixed)> + '_ {
        self.axes.iter().map(|(&a, &v)| (a, v))
    }
}

/// Project an emic value profile onto the etic substrate through a race's projections. Each
/// emic stance is spread onto the substrate axes its projection names, weighted; an emic
/// axis with no projection (or an empty one) contributes nothing (the untranslatable,
/// blind-spot case). A pure deterministic function: the accumulation walks the profile and
/// each projection in canonical order. This is [`project_to_etic_with_loss`] with the measured
/// projection loss discarded.
pub fn project_to_etic(profile: &ValueProfile, race: &RaceProjection) -> EticProfile {
    project_to_etic_with_loss(profile, race).0
}

/// Project an emic value profile onto the etic substrate and, alongside the projection, measure
/// its projection loss: the sum of squared stances of every emic axis the projection drops (an
/// untranslatable axis with no etic image, whether it has no projection entry or an empty one).
/// The etic profile is exactly what [`project_to_etic`] yields; the second value is the measured
/// incommensurability of this one profile under this one race's projections, zero when every
/// carried axis translates and the squared magnitude of each untranslatable stance otherwise.
/// A caller folds this loss into its sum-of-squares before a single root, so a cross-race
/// distance rises by exactly the untranslated magnitude rather than a flat constant. This
/// measurement is the reusable primitive the cross-race value distance and the later
/// language-distance incommensurability ceiling both read. A pure deterministic function of the
/// profile and the race projection: it reads only whether each carried axis has an etic image,
/// never a race identifier.
pub fn project_to_etic_with_loss(
    profile: &ValueProfile,
    race: &RaceProjection,
) -> (EticProfile, Fixed) {
    let mut axes: BTreeMap<EticAxisId, Fixed> = BTreeMap::new();
    let mut loss = Fixed::ZERO;
    for (axis, stance) in profile.axes() {
        let s = Fixed::from_int(stance as i32);
        let onto = race
            .per_axis
            .get(&axis)
            .map(|p| p.onto.as_slice())
            .unwrap_or(&[]);
        if onto.is_empty() {
            // Untranslatable: the axis drops out of the etic frame, and its squared stance is
            // the projection loss it contributes (the theory-of-mind blind spot, measured).
            loss += s.mul(s);
            continue;
        }
        for &(etic, weight) in onto {
            *axes.entry(etic).or_insert(Fixed::ZERO) += s.mul(weight);
        }
    }
    (EticProfile { axes }, loss)
}

/// The sum of squared differences over the etic substrate axes both projected profiles carry
/// (the Euclidean sum before the root). Kept apart from the root so the cross-race distance can
/// fold each side's measured projection loss into the same sum-of-squares under one final
/// [`Fixed::sqrt`].
fn etic_sq_distance(a: &EticProfile, b: &EticProfile) -> Fixed {
    let mut acc = Fixed::ZERO;
    for (axis, av) in a.axes() {
        if let Some(bv) = b.get(axis) {
            let d = av - bv;
            acc += d.mul(d);
        }
    }
    acc
}

/// The cross-race distance between two value profiles (design Part 21): project both onto the
/// shared etic substrate and read the distance there, with each profile's untranslatable emic
/// axes folded in as measured projection loss (even an aligned alien unsettles, by exactly the
/// magnitude neither race can carry into the shared frame). Pinned when the etic substrate is
/// independent: the Euclidean sum-of-squares over the shared substrate axes plus each side's
/// squared projection loss, under one final [`Fixed::sqrt`]. For a structured etic substrate the
/// closed form is the same flagged seam, so this returns `None`. The incommensurability is
/// measured per race-pair from [`project_to_etic_with_loss`], not a flat reserved floor: full
/// projections add nothing, and one extra untranslatable axis of stance `s` raises the distance
/// by exactly `|s|`. A pure function, symmetric under swapping the two sides; it never branches
/// on a race identifier.
pub fn cross_race_distance(
    a: &ValueProfile,
    ra: &RaceProjection,
    b: &ValueProfile,
    rb: &RaceProjection,
    etic_metric: &GroundMetric,
) -> Option<Fixed> {
    match etic_metric.kind {
        StructureKind::Independent => {
            let (ea, loss_a) = project_to_etic_with_loss(a, ra);
            let (eb, loss_b) = project_to_etic_with_loss(b, rb);
            let sum_of_squares = etic_sq_distance(&ea, &eb) + loss_a + loss_b;
            Some(sum_of_squares.sqrt())
        }
        StructureKind::Graph | StructureKind::Relationship => None,
    }
}

/// The incommensurability ceiling of a race-pair: the summed measured etic projection loss of the
/// two sides, the untranslatable magnitude neither race carries into the shared frame (design Part
/// 21). A pure ALIAS of what [`project_to_etic_with_loss`] already measures (Wave 1): the same two
/// loss terms [`cross_race_distance`] folds into its sum-of-squares, surfaced on their own so a
/// caller that needs the ceiling reads the measured loss rather than a separately authored constant.
/// It is the value-space primitive the language-distance layer's no-shared-form ceiling and per-cell
/// one-sided-coverage contribution are the normalised image of (Part 33.5): a wholly untranslatable
/// pair is the ceiling, exactly as a wholly non-cognate lexicon is the top of the lexical share.
/// Symmetric under swapping the two sides (the sum of the two losses is order-free), and it never
/// branches on a race identifier: it reads only whether each carried axis has an etic image.
pub fn incommensurability_ceiling(
    a: &ValueProfile,
    ra: &RaceProjection,
    b: &ValueProfile,
    rb: &RaceProjection,
) -> Fixed {
    let (_, loss_a) = project_to_etic_with_loss(a, ra);
    let (_, loss_b) = project_to_etic_with_loss(b, rb);
    loss_a + loss_b
}

/// Conflict pressure between two groups from their value distance and relationship state
/// (design Part 21): `value_dist * conflict_coefficient * (1 + niche_overlap) + grievance -
/// trust`. None of the four inputs belongs to either group alone; pressure is a property of
/// the pair. The `conflict_coefficient` is the reserved `value_metric.conflict_coefficient`,
/// supplied by the caller, so the distance-to-pressure mapping is calibrated, not asserted
/// here. A pure function.
pub fn conflict_pressure(
    value_dist: Fixed,
    conflict_coefficient: Fixed,
    niche_overlap: Fixed,
    grievance: Fixed,
    trust: Fixed,
) -> Fixed {
    value_dist
        .mul(conflict_coefficient)
        .mul(Fixed::ONE + niche_overlap)
        + grievance
        - trust
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(n: u32) -> ValueAxisId {
        ValueAxisId(n)
    }

    #[test]
    fn ground_metric_is_exact_shortest_paths_on_a_ring() {
        // A 4-cycle with unit edges: opposite axes are distance 2, adjacent distance 1.
        let edges = vec![
            GraphEdge {
                a: 0,
                b: 1,
                weight: 1,
            },
            GraphEdge {
                a: 1,
                b: 2,
                weight: 1,
            },
            GraphEdge {
                a: 2,
                b: 3,
                weight: 1,
            },
            GraphEdge {
                a: 3,
                b: 0,
                weight: 1,
            },
        ];
        let g = GroundMetric::compile(&ValueStructure::Graph { k: 4, edges });
        assert_eq!(g.between(0, 0), Fixed::ZERO);
        assert_eq!(g.between(0, 1), Fixed::from_int(1));
        assert_eq!(g.between(0, 3), Fixed::from_int(1), "wraps around the ring");
        assert_eq!(
            g.between(0, 2),
            Fixed::from_int(2),
            "opposite axes are two hops"
        );
        // Symmetric.
        assert_eq!(g.between(2, 0), g.between(0, 2));
    }

    #[test]
    fn ground_metric_relaxes_a_long_edge_through_a_short_path() {
        // 0-1 weight 10, but 0-2-1 costs 1+1: the shortest path wins.
        let edges = vec![
            GraphEdge {
                a: 0,
                b: 1,
                weight: 10,
            },
            GraphEdge {
                a: 0,
                b: 2,
                weight: 1,
            },
            GraphEdge {
                a: 2,
                b: 1,
                weight: 1,
            },
        ];
        let g = GroundMetric::compile(&ValueStructure::Graph { k: 3, edges });
        assert_eq!(
            g.between(0, 1),
            Fixed::from_int(2),
            "the two-hop path beats the direct edge"
        );
    }

    #[test]
    fn disconnected_axes_are_unreachable() {
        let edges = vec![GraphEdge {
            a: 0,
            b: 1,
            weight: 1,
        }];
        let g = GroundMetric::compile(&ValueStructure::Graph { k: 3, edges });
        assert_eq!(g.between(0, 2), GroundMetric::UNREACHABLE);
        assert_eq!(g.between(2, 2), Fixed::ZERO);
    }

    #[test]
    fn independent_distance_is_euclidean_over_shared_axes() {
        // Stances (3, 0) vs (0, 4): a 3-4-5 right triangle, distance 5.
        let a = ValueProfile::with([(axis(0), 3), (axis(1), 0)]);
        let b = ValueProfile::with([(axis(0), 0), (axis(1), 4)]);
        let g = GroundMetric::compile(&ValueStructure::Independent { k: 2 });
        assert_eq!(value_distance(&a, &b, &g), Some(Fixed::from_int(5)));
        assert_eq!(euclidean_distance(&a, &b), Fixed::from_int(5));
    }

    #[test]
    fn partial_profiles_sum_over_shared_axes_only() {
        // b has no stance on axis 1, so only axis 0 (difference 3) contributes.
        let a = ValueProfile::with([(axis(0), 3), (axis(1), 9)]);
        let b = ValueProfile::with([(axis(0), 0)]);
        assert_eq!(euclidean_distance(&a, &b), Fixed::from_int(3));
    }

    #[test]
    fn structured_distance_is_a_flagged_seam_returning_none() {
        let a = ValueProfile::with([(axis(0), 1)]);
        let b = ValueProfile::with([(axis(0), -1)]);
        let graph = GroundMetric::compile(&ValueStructure::Graph {
            k: 1,
            edges: vec![],
        });
        let rel = GroundMetric::compile(&ValueStructure::Relationship {
            k: 1,
            matrix: vec![Fixed::ONE],
        });
        assert_eq!(value_distance(&a, &b, &graph), None);
        assert_eq!(value_distance(&a, &b, &rel), None);
    }

    /// A race projection mapping the given emic axis onto one shared etic axis with unit weight.
    fn onto_shared(a: ValueAxisId, etic: EticAxisId) -> RaceProjection {
        RaceProjection {
            per_axis: [(
                a,
                EmicProjection {
                    onto: vec![(etic, Fixed::ONE)],
                },
            )]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn cross_race_distance_adds_no_floor_when_every_axis_translates() {
        // Two races, one shared etic axis, both profiles translating fully: the distance is the
        // plain etic distance with nothing added. The flat incommensurability floor is retired.
        let etic = EticAxisId(100);
        let race = onto_shared(axis(0), etic);
        let a = ValueProfile::with([(axis(0), 5)]);
        let b = ValueProfile::with([(axis(0), 2)]);
        let etic_metric = GroundMetric::compile(&ValueStructure::Independent { k: 1 });
        // Projected magnitudes 5 and 2: distance 3, no projection loss, so exactly 3.
        assert_eq!(
            cross_race_distance(&a, &race, &b, &race, &etic_metric),
            Some(Fixed::from_int(3))
        );
    }

    #[test]
    fn one_untranslatable_axis_raises_the_distance_by_exactly_its_magnitude() {
        // Both profiles identical in the etic frame (distance zero). One extra untranslatable
        // emic axis of stance s on one side raises the pair distance to exactly |s|, the measured
        // projection loss under one root, not a flat constant.
        let etic = EticAxisId(100);
        let ra = onto_shared(axis(0), etic); // axis 1 has no projection: untranslatable
        let rb = onto_shared(axis(0), etic);
        let etic_metric = GroundMetric::compile(&ValueStructure::Independent { k: 1 });
        let base = ValueProfile::with([(axis(0), 4)]);
        // Baseline: identical translatable stance, distance zero (nothing added).
        assert_eq!(
            cross_race_distance(&base, &ra, &base, &rb, &etic_metric),
            Some(Fixed::ZERO)
        );
        // Add one untranslatable axis of stance 3: the distance is exactly |3|.
        let a_pos = ValueProfile::with([(axis(0), 4), (axis(1), 3)]);
        assert_eq!(
            cross_race_distance(&a_pos, &ra, &base, &rb, &etic_metric),
            Some(Fixed::from_int(3))
        );
        // The rise depends only on the magnitude: stance -3 gives the same distance, |-3| = 3.
        let a_neg = ValueProfile::with([(axis(0), 4), (axis(1), -3)]);
        assert_eq!(
            cross_race_distance(&a_neg, &ra, &base, &rb, &etic_metric),
            Some(Fixed::from_int(3))
        );
    }

    #[test]
    fn cross_race_distance_is_symmetric_under_swap() {
        // Distance(a, b) equals Distance(b, a), including the projection-loss term: the etic
        // sum-of-squares is symmetric and loss_a + loss_b is order-free.
        let etic = EticAxisId(100);
        let ra = onto_shared(axis(0), etic); // axis 1 untranslatable for race a
        let rb = onto_shared(axis(0), etic);
        // a carries an untranslatable axis under ra; b a different translatable stance under rb.
        let a = ValueProfile::with([(axis(0), 5), (axis(1), 2)]);
        let b = ValueProfile::with([(axis(0), 1)]);
        let etic_metric = GroundMetric::compile(&ValueStructure::Independent { k: 1 });
        let forward = cross_race_distance(&a, &ra, &b, &rb, &etic_metric);
        let backward = cross_race_distance(&b, &rb, &a, &ra, &etic_metric);
        assert!(forward.is_some());
        assert_eq!(forward, backward, "the pair distance is symmetric");
    }

    #[test]
    fn project_to_etic_with_loss_measures_the_untranslatable_magnitude() {
        // The reusable projection-loss measurement (shared with the language-distance
        // incommensurability ceiling): the etic image matches project_to_etic, and the loss is
        // the sum of squared stances of every dropped axis.
        let etic = EticAxisId(100);
        let race = onto_shared(axis(0), etic);
        // Axis 0 translates (stance 4 -> etic 4); axes 1 and 2 are untranslatable.
        let profile = ValueProfile::with([(axis(0), 4), (axis(1), 3), (axis(2), -2)]);
        let (image, loss) = project_to_etic_with_loss(&profile, &race);
        assert_eq!(
            image,
            project_to_etic(&profile, &race),
            "the etic image is exactly project_to_etic's"
        );
        assert_eq!(image.get(etic), Some(Fixed::from_int(4)));
        // Loss is 3^2 + (-2)^2 = 13.
        assert_eq!(loss, Fixed::from_int(13));
        // A fully translatable profile has zero loss.
        let full = ValueProfile::with([(axis(0), 7)]);
        let (_, none_lost) = project_to_etic_with_loss(&full, &race);
        assert_eq!(none_lost, Fixed::ZERO);
        // An explicitly empty projection is untranslatable too, and contributes to the loss.
        let empty = RaceProjection {
            per_axis: [(axis(0), EmicProjection { onto: vec![] })]
                .into_iter()
                .collect(),
        };
        let (empty_image, empty_loss) =
            project_to_etic_with_loss(&ValueProfile::with([(axis(0), 5)]), &empty);
        assert_eq!(
            empty_image.axes().count(),
            0,
            "an empty projection carries nothing into the etic frame"
        );
        assert_eq!(empty_loss, Fixed::from_int(25));
    }

    #[test]
    fn an_untranslatable_emic_axis_is_a_blind_spot() {
        // Race a has no projection for axis 1, so its stance there vanishes in the etic frame.
        let etic = EticAxisId(100);
        let race = RaceProjection {
            per_axis: [(
                axis(0),
                EmicProjection {
                    onto: vec![(etic, Fixed::ONE)],
                },
            )]
            .into_iter()
            .collect(),
        };
        let profile = ValueProfile::with([(axis(0), 4), (axis(1), 9)]);
        let projected = project_to_etic(&profile, &race);
        assert_eq!(projected.get(etic), Some(Fixed::from_int(4)));
        assert_eq!(
            projected.axes().count(),
            1,
            "the untranslatable axis does not appear"
        );
    }

    #[test]
    fn ground_metric_resolution_is_the_grid_separability_floor() {
        // A graph with a lightest edge of 1 and a heavier of 3: the finest resolvable hop is 1.
        let edges = vec![
            GraphEdge {
                a: 0,
                b: 1,
                weight: 1,
            },
            GraphEdge {
                a: 1,
                b: 2,
                weight: 3,
            },
        ];
        let g = GroundMetric::compile(&ValueStructure::Graph { k: 3, edges });
        assert_eq!(
            g.resolution(),
            Fixed::from_int(1),
            "the lightest positive hop is the granularity"
        );
        // Independent axes do not connect, so the resolution defaults to the unit grid step, never
        // zero: a quantization taken consistent with it cannot collapse distinct regions.
        let ind = GroundMetric::compile(&ValueStructure::Independent { k: 3 });
        assert_eq!(ind.resolution(), Fixed::ONE);
    }

    #[test]
    fn incommensurability_ceiling_is_the_measured_projection_loss_of_the_pair() {
        // The ceiling equals the sum of the two sides' measured projection losses, the same terms
        // cross_race_distance folds in, surfaced on their own. Symmetric, and no RaceId enters.
        let etic = EticAxisId(100);
        let ra = onto_shared(axis(0), etic); // axes 1, 2 untranslatable under ra
        let rb = onto_shared(axis(0), etic);
        let a = ValueProfile::with([(axis(0), 4), (axis(1), 3)]); // loss_a = 3^2 = 9
        let b = ValueProfile::with([(axis(0), 2), (axis(2), -2), (axis(3), 4)]); // loss_b = 4 + 16 = 20
        let (_, loss_a) = project_to_etic_with_loss(&a, &ra);
        let (_, loss_b) = project_to_etic_with_loss(&b, &rb);
        assert_eq!(
            incommensurability_ceiling(&a, &ra, &b, &rb),
            loss_a + loss_b,
            "the ceiling is a pure alias of the measured projection loss"
        );
        assert_eq!(
            incommensurability_ceiling(&a, &ra, &b, &rb),
            Fixed::from_int(29)
        );
        // Symmetric under swapping the two sides.
        assert_eq!(
            incommensurability_ceiling(&a, &ra, &b, &rb),
            incommensurability_ceiling(&b, &rb, &a, &ra),
            "the pair ceiling is symmetric"
        );
        // A fully-translatable pair has a zero ceiling: nothing is untranslatable.
        let full = ValueProfile::with([(axis(0), 7)]);
        assert_eq!(
            incommensurability_ceiling(&full, &ra, &full, &rb),
            Fixed::ZERO
        );
    }

    #[test]
    fn conflict_pressure_combines_distance_niche_grievance_and_trust() {
        // dist 2, coefficient 1, niche 0.5 -> 2 * 1 * 1.5 = 3; +grievance 4 -trust 1 = 6.
        let p = conflict_pressure(
            Fixed::from_int(2),
            Fixed::ONE,
            Fixed::from_ratio(1, 2),
            Fixed::from_int(4),
            Fixed::from_int(1),
        );
        assert_eq!(p, Fixed::from_int(6));
    }
}
