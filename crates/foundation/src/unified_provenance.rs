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

//! The joined provenance register (provenance register Phase 2 slice 4, the floor unification;
//! `docs/PROVENANCE_LEDGER.md`).
//!
//! Phase 1 made the seven-tag register mandatory over `calibration/reserved.toml` (the calibration honesty
//! surface). Phase 2 refined the physics floor's two-tag `real`/`fantasy` provenance into the same seven tags
//! (`civsim_physics::floor_provenance`, the floor honesty surface). This module folds the two into ONE node
//! set so the honesty number is a SINGLE query spanning both registers, under the identical worst-case DAG
//! join Phase 1 built: a value is only as pinned as its least-pinned transitive input, so a derived value is
//! authoring-tainted the moment its DAG touches a single closure or authored value, in EITHER register.
//!
//! The cross-register taint is the point of the fold. A calibration derived value names its full source list
//! in `derived_from`; a source that is a reserved-value id is a joinable edge in its own register, but a
//! source that is a FLOOR quantity was disclosed-yet-un-joinable there (the manifest DAG traces only its own
//! register's ancestry). Here the joinable edge set is recomputed against the UNIFIED namespace, so a
//! calibration value deriving from a floor quantity (or a floor value deriving from a calibration one) joins
//! across the seam. Today the one live cross-register edge is benign (`langmod.perceptual_geometry` derives
//! from two measured acoustic floor axes, and a measured input never taints toward the surface), so the
//! unified surface is the two independent surfaces combined with no flip. The mechanism carries the taint
//! regardless, so a future closure edge across the seam surfaces correctly rather than hiding.
//!
//! Byte-neutral by construction: this is an accounting query with no run-path caller (the sibling of
//! `CalibrationManifest::authoring_surface` and `FloorProvenance::authoring_surface`), so it cannot move a
//! fixed-point pin.

use crate::calibration::{CalibrationError, CalibrationManifest, Provenance};
use civsim_physics::floor_provenance::FloorProvenance;
use std::collections::{BTreeMap, BTreeSet};

/// Which source register a joined node came from, so a cross-register edge (a taint path spanning the seam)
/// is identifiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    /// A `calibration/reserved.toml` entry.
    Calibration,
    /// A physics-floor grade (`crates/physics/data/floor_provenance.toml`).
    Floor,
}

/// One node in the joined register: its source register, its declared provenance, and its joinable input
/// edges (the declared sources that resolve to another node in the UNIFIED node set, in either register).
#[derive(Debug, Clone)]
pub struct JoinedNode {
    /// The register this node came from.
    pub register: Register,
    /// The DECLARED seven-tag provenance (not the effective, DAG-joined one, which [`JoinedRegister`] computes).
    pub provenance: Provenance,
    /// The joinable input edges: the declared sources that resolve to a node id in the joined register,
    /// recomputed against the unified namespace so a cross-register source joins here.
    pub inputs: Vec<String>,
}

/// The joined provenance register: the calibration manifest and the physics floor as ONE node set, so the
/// worst-case DAG join and the honesty surface are a single query spanning both. The join descends only from
/// a `Derived` node (a leaf's effective provenance is its own tag); the effective provenance of a value is
/// the minimum-rank (least-pinned) member of its declared tag joined with every transitive input, so an
/// authored or closure ancestor anywhere in the DAG surfaces the derived value onto the authoring surface.
#[derive(Debug)]
pub struct JoinedRegister {
    /// Every node id, deterministic: the calibration entries in file order, then the floor grades in register
    /// order.
    order: Vec<String>,
    /// The per-id nodes.
    nodes: BTreeMap<String, JoinedNode>,
}

impl JoinedRegister {
    /// Fold the two registers into one node set. The joinable edge set of each node is recomputed against the
    /// UNIFIED namespace (`derived_from` intersected with the full key set), so a cross-register source joins.
    /// Fails loud on a calibration/floor id collision (a duplicate id has no well-defined owner) or a floor
    /// grade string that is not one of the seven tags.
    pub fn build(
        cal: &CalibrationManifest,
        floor: &FloorProvenance,
    ) -> Result<Self, CalibrationError> {
        // Pass 1: the full key set (both registers), so a joinable edge can resolve across the seam. A
        // collision between a calibration id and a floor id is a defect, not a silent overwrite.
        let mut order = Vec::new();
        let mut keys: BTreeSet<String> = BTreeSet::new();
        for v in cal.iter() {
            if !keys.insert(v.id.clone()) {
                return Err(CalibrationError::Duplicate(v.id.clone()));
            }
            order.push(v.id.clone());
        }
        for g in &floor.grades {
            if !keys.insert(g.id.clone()) {
                return Err(CalibrationError::Duplicate(g.id.clone()));
            }
            order.push(g.id.clone());
        }

        // Pass 2: the nodes. The joinable inputs are the declared sources that resolve to a node in the
        // unified key set: for a calibration value that is its intra-calibration `inputs` PLUS any floor
        // quantity it names (un-joinable in calibration alone, joinable here); for a floor grade it is the
        // constituent ids that are themselves floor entries. Code-level and prose sources do not resolve and
        // stay disclosed-but-un-joinable, exactly as in each register alone.
        let mut nodes = BTreeMap::new();
        for v in cal.iter() {
            let provenance = v.provenance()?;
            let inputs = v
                .derived_from
                .iter()
                .filter(|s| keys.contains(s.as_str()))
                .cloned()
                .collect();
            nodes.insert(
                v.id.clone(),
                JoinedNode {
                    register: Register::Calibration,
                    provenance,
                    inputs,
                },
            );
        }
        for g in &floor.grades {
            let provenance = Provenance::from_tag(&g.grade).ok_or_else(|| {
                CalibrationError::BadValue {
                    id: g.id.clone(),
                    detail: format!(
                        "floor grade '{}' is not one of the seven tags: derived, measured, estimator, closure, authored, written_state, contingency",
                        g.grade
                    ),
                }
            })?;
            let inputs = g
                .derived_from
                .iter()
                .filter(|s| keys.contains(s.as_str()))
                .cloned()
                .collect();
            nodes.insert(
                g.id.clone(),
                JoinedNode {
                    register: Register::Floor,
                    provenance,
                    inputs,
                },
            );
        }

        // CYCLES FAIL CONSTRUCTION. A cycle is a malformed graph, not a provenance grade. Resolved
        // lazily it became `Unclassified`, and the authoring-surface query selects only `Closure` and
        // `Authored`, so a cyclic derivation was OMITTED from the honesty surface rather than surfaced as
        // suspect: two floor rows naming each other in `derived_from` passed every structural check and
        // then vanished from the count that is supposed to make authoring visible.
        //
        // Three-colour depth-first search over the resolved value-to-value edges. It convicts nothing in
        // the current tree, which is the point: it is a trap set before the thing it catches exists,
        // rather than a fix applied after one shipped.
        let register = JoinedRegister { order, nodes };
        register.assert_acyclic()?;
        Ok(register)
    }

    /// Refuse a cyclic derivation graph, naming the exact cycle path.
    ///
    /// `A -> B -> A` is not a value with unknown provenance; it is two values each claiming the other as
    /// its ground, which grounds neither. Reported as the full path rather than as a boolean, because
    /// "there is a cycle somewhere" is not actionable and the path is.
    fn assert_acyclic(&self) -> Result<(), CalibrationError> {
        // 0 unvisited, 1 on the current stack (grey), 2 finished (black).
        let mut colour: BTreeMap<&str, u8> = BTreeMap::new();
        for key in &self.order {
            if colour.get(key.as_str()).copied().unwrap_or(0) != 0 {
                continue;
            }
            let mut stack: Vec<(&str, usize)> = vec![(key.as_str(), 0)];
            let mut path: Vec<&str> = vec![key.as_str()];
            colour.insert(key.as_str(), 1);
            while let Some((node, idx)) = stack.pop() {
                let inputs = self.nodes.get(node).map(|n| &n.inputs);
                let next = inputs.and_then(|v| v.get(idx));
                match next {
                    Some(child) => {
                        stack.push((node, idx + 1));
                        let c = colour.get(child.as_str()).copied().unwrap_or(0);
                        if c == 1 {
                            // Grey means it is on the current path: splice the cycle out of it.
                            let start = path.iter().position(|p| *p == child.as_str()).unwrap_or(0);
                            let mut cycle: Vec<&str> = path[start..].to_vec();
                            cycle.push(child.as_str());
                            return Err(CalibrationError::BadValue {
                                id: child.clone(),
                                detail: format!(
                                    "provenance cycle: {}. Each of these names the next as its ground, so \
                                     none of them is grounded. A cycle is a malformed graph rather than a \
                                     grade, and it is not baseline-waivable.",
                                    cycle.join(" -> ")
                                ),
                            });
                        }
                        if c == 0 {
                            let child_key = self
                                .nodes
                                .get_key_value(child.as_str())
                                .map(|(k, _)| k.as_str())
                                .unwrap_or(child.as_str());
                            colour.insert(child_key, 1);
                            path.push(child_key);
                            stack.push((child_key, 0));
                        }
                    }
                    None => {
                        colour.insert(node, 2);
                        if path.last() == Some(&node) {
                            path.pop();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// The number of nodes in the joined register (calibration entries plus floor grades).
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether the joined register is empty (never true for the real registers; present for lint parity).
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// A node by id.
    pub fn node(&self, id: &str) -> Option<&JoinedNode> {
        self.nodes.get(id)
    }

    /// The EFFECTIVE provenance of a node: the worst-case join of its declared provenance with the effective
    /// provenance of every joinable input, transitively up the unified DAG. Only a `Derived` node joins over
    /// inputs; a leaf returns its own tag. Cycle-safe (an id already on the resolution path is not re-entered)
    /// and total (an input naming an unknown id contributes `Unclassified`, a suspect leaf, rather than
    /// panicking). The provenance strings were parsed at build time, so this is infallible.
    pub fn effective_provenance(&self, id: &str) -> Provenance {
        let mut on_path = BTreeSet::new();
        self.effective_provenance_inner(id, &mut on_path)
    }

    fn effective_provenance_inner(&self, id: &str, on_path: &mut BTreeSet<String>) -> Provenance {
        let Some(node) = self.nodes.get(id) else {
            return Provenance::Unclassified;
        };
        if node.provenance != Provenance::Derived {
            return node.provenance;
        }
        if !on_path.insert(id.to_string()) {
            return Provenance::Unclassified;
        }
        let mut worst = node.provenance;
        for input in &node.inputs {
            let eff = self.effective_provenance_inner(input, on_path);
            if eff.rank() < worst.rank() {
                worst = eff;
            }
        }
        on_path.remove(id);
        worst
    }

    /// The UNIFIED HONESTY SURFACE: every node whose EFFECTIVE provenance is on the authoring surface,
    /// `Closure` or `Authored`, whether declared at the root or inherited through a derived chain that touches
    /// one, across BOTH registers. This is the single honest count of world-content values resting on
    /// set-points a laboratory could not refute without running the sim, the calibration surface and the floor
    /// surface joined with any cross-register taint applied. Returned in node order, deterministic.
    pub fn authoring_surface(&self) -> Vec<&str> {
        self.order
            .iter()
            .filter(|id| {
                let eff = self.effective_provenance(id);
                eff == Provenance::Closure || eff == Provenance::Authored
            })
            .map(|s| s.as_str())
            .collect()
    }

    /// The edges that SPAN the two registers: a joinable input of a node in one register that resolves to a
    /// node contributed by the other. These are the taint paths the fold makes joinable (un-joinable in either
    /// register alone). Returned as `(consumer, source)` pairs in node order, deterministic. Today this is the
    /// two benign `langmod.perceptual_geometry` acoustic edges (a calibration derived value reaching two
    /// measured floor axes); the list is the audit surface for a future closure edge across the seam.
    pub fn cross_register_edges(&self) -> Vec<(&str, &str)> {
        let mut edges = Vec::new();
        for id in &self.order {
            let node = &self.nodes[id];
            for input in &node.inputs {
                if let Some(src) = self.nodes.get(input) {
                    if src.register != node.register {
                        edges.push((id.as_str(), input.as_str()));
                    }
                }
            }
        }
        edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_calibration() -> CalibrationManifest {
        CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/reserved.toml"
        ))
        .expect("the real calibration manifest loads")
    }

    fn real_floor() -> FloorProvenance {
        FloorProvenance::embedded().expect("the embedded floor grade register parses")
    }

    /// THE CYCLE TRAP, set before the thing it catches exists.
    ///
    /// A cycle used to resolve lazily to `Unclassified`, and the authoring-surface query selects only
    /// `Closure` and `Authored`, so a cyclic derivation was OMITTED from the honesty surface rather than
    /// surfaced as suspect. Two rows naming each other passed every structural check and then vanished
    /// from the count that exists to make authoring visible. The current tree has no such cycle, so this
    /// builds one directly against the detector: a detector with nothing to catch is one nobody has
    /// proven, and this repository has already shipped two gates whose failure path had never run.
    #[test]
    fn a_provenance_cycle_fails_construction_and_names_its_path() {
        let mut nodes: BTreeMap<String, JoinedNode> = BTreeMap::new();
        let mk = |inputs: Vec<&str>| JoinedNode {
            register: Register::Floor,
            provenance: Provenance::Derived,
            inputs: inputs.into_iter().map(String::from).collect(),
        };
        nodes.insert("floor.a".into(), mk(vec!["floor.b"]));
        nodes.insert("floor.b".into(), mk(vec!["floor.a"]));
        let register = JoinedRegister {
            order: vec!["floor.a".into(), "floor.b".into()],
            nodes,
        };
        let err = register
            .assert_acyclic()
            .expect_err("a cycle must refuse rather than resolve to Unclassified");
        let text = format!("{err:?}");
        assert!(
            text.contains("provenance cycle"),
            "names the defect: {text}"
        );
        assert!(
            text.contains("floor.a") && text.contains("floor.b"),
            "names the PATH, because 'there is a cycle somewhere' is not actionable: {text}"
        );

        // And an acyclic graph passes, so the detector is not simply always-on.
        let mut ok: BTreeMap<String, JoinedNode> = BTreeMap::new();
        ok.insert("floor.a".into(), mk(vec!["floor.b"]));
        ok.insert("floor.b".into(), mk(vec![]));
        let good = JoinedRegister {
            order: vec!["floor.a".into(), "floor.b".into()],
            nodes: ok,
        };
        assert!(good.assert_acyclic().is_ok(), "an acyclic graph must pass");
    }

    #[test]
    fn the_joined_register_is_the_two_registers_as_one_node_set() {
        let cal = real_calibration();
        let floor = real_floor();
        let joined =
            JoinedRegister::build(&cal, &floor).expect("the fold succeeds, no id collision");
        // 229 calibration entries + 243 floor grades, no calibration/floor id collision.
        assert_eq!(
            cal.iter().count(),
            229,
            "the calibration manifest has 229 entries"
        );
        assert_eq!(floor.grades.len(), 243, "the floor register has 243 grades");
        assert_eq!(
            joined.len(),
            472,
            "the joined register is the two node sets with no id collision (229 + 243)"
        );
    }

    #[test]
    fn the_unified_honesty_number_is_the_two_surfaces_joined_with_no_cross_register_flip() {
        let cal = real_calibration();
        let floor = real_floor();
        let joined = JoinedRegister::build(&cal, &floor).expect("the fold succeeds");

        // The two independent surfaces, each computed by its own register's query (no literal).
        let cal_surface: BTreeSet<String> = cal
            .authoring_surface()
            .expect("the calibration surface query succeeds")
            .into_iter()
            .map(str::to_string)
            .collect();
        let floor_surface: BTreeSet<String> = floor
            .authoring_surface()
            .into_iter()
            .map(str::to_string)
            .collect();
        let unified: BTreeSet<String> = joined
            .authoring_surface()
            .into_iter()
            .map(str::to_string)
            .collect();

        // The floor surface is the 6 reserved bio/chem couplings; the calibration surface is the Phase-1 205.
        assert_eq!(floor_surface.len(), 6, "the floor authoring surface is 6");
        assert_eq!(
            cal_surface.len(),
            205,
            "the calibration authoring surface is 205"
        );

        // The join adds NO cross-register flip: the unified surface is exactly the disjoint union of the two,
        // because no value in either register derives from an authoring-surface value in the other (verified at
        // source: the one live cross-register edge lands on measured floor axes). The count is a query, and it
        // is 205 + 6 = 211.
        let expected: BTreeSet<String> = cal_surface.union(&floor_surface).cloned().collect();
        assert_eq!(
            unified, expected,
            "the unified surface is exactly the two surfaces joined, no cross-register flip"
        );
        assert_eq!(
            unified.len(),
            cal_surface.len() + floor_surface.len(),
            "no id off both surfaces is tainted onto the unified surface"
        );
        assert_eq!(
            unified.len(),
            211,
            "the unified honesty number is 211 (205 + 6)"
        );
    }

    #[test]
    fn the_cross_register_acoustic_edge_is_traversed_and_benign() {
        let cal = real_calibration();
        let floor = real_floor();
        let joined = JoinedRegister::build(&cal, &floor).expect("the fold succeeds");

        // langmod.perceptual_geometry is a calibration derived value whose derived_from names two floor axes.
        // Those edges are un-joinable in the calibration register alone (they are not reserved values); the
        // fold recomputes joinable inputs against the unified namespace, so they join HERE.
        let node = joined
            .node("langmod.perceptual_geometry")
            .expect("the node is present");
        assert_eq!(node.provenance, Provenance::Derived);
        assert!(
            node.inputs
                .iter()
                .any(|s| s == "acoustic.absorption_reference"),
            "the cross-register acoustic edge is joinable in the unified namespace"
        );
        assert!(
            node.inputs.iter().any(|s| s == "acoustic.resonator_length"),
            "the second cross-register acoustic edge joins too"
        );
        // THE ASSERTION MOVED WITH THE TRUTH, 2026-07-19, and the test's INTENT is unchanged. Both
        // acoustic axes used to be labelled `measured`; an audit found neither carried machine-checkable
        // evidence, and all 244 such labels were downgraded to `unverified_measurement_candidate`. The
        // derived value therefore now INHERITS that weakness, which is the join working correctly: a
        // value derived from an unverified measurement is not better evidenced than its input.
        //
        // What this test exists to prove is untouched: the join does not taint toward the AUTHORING
        // surface. `Closure` and `Authored` are what that surface means, and an unverified measurement is
        // a weaker evidence claim rather than a free knob, so the property still holds and is asserted
        // directly rather than implied by an equality that only happened to hold under the old ranking.
        let joined_prov = joined.effective_provenance("langmod.perceptual_geometry");
        assert_eq!(
            joined_prov,
            Provenance::UnverifiedMeasurementCandidate,
            "a derived value inherits the weakest evidence among its inputs"
        );
        assert!(
            !matches!(joined_prov, Provenance::Closure | Provenance::Authored),
            "and it still never taints onto the AUTHORING surface, which is what this test is for"
        );
        // The floor leaf itself: labelled measured before the 2026-07-19 evidence audit, now honestly an
        // unverified candidate, because no machine-checkable source backs it. This is the value the
        // derived node above inherits from, so the two assertions move together or one of them is lying.
        assert_eq!(
            joined.effective_provenance("acoustic.absorption_reference"),
            Provenance::UnverifiedMeasurementCandidate
        );

        // The cross-register edge list surfaces exactly these acoustic edges (the audit surface for a future
        // closure edge across the seam).
        let edges = joined.cross_register_edges();
        assert!(
            edges.contains(&(
                "langmod.perceptual_geometry",
                "acoustic.absorption_reference"
            )),
            "the cross-register edge is on the audit surface"
        );
        assert!(
            edges
                .iter()
                .all(|(consumer, _)| *consumer == "langmod.perceptual_geometry"),
            "langmod.perceptual_geometry is the only live cross-register consumer today"
        );
    }

    #[test]
    fn a_closure_edge_across_registers_taints_onto_the_unified_surface() {
        // The mechanism proof: the join is a real worst-case DAG traversal, not the sum of two counts. A
        // synthetic calibration derived value deriving from a floor closure inherits the closure taint across
        // the seam and surfaces onto the authoring number.
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "cal.consumer".to_string(),
            JoinedNode {
                register: Register::Calibration,
                provenance: Provenance::Derived,
                inputs: vec!["floor.free_knob".to_string()],
            },
        );
        nodes.insert(
            "floor.free_knob".to_string(),
            JoinedNode {
                register: Register::Floor,
                provenance: Provenance::Closure,
                inputs: vec![],
            },
        );
        let joined = JoinedRegister {
            order: vec!["cal.consumer".to_string(), "floor.free_knob".to_string()],
            nodes,
        };
        assert_eq!(
            joined.effective_provenance("cal.consumer"),
            Provenance::Closure,
            "a derived value deriving from a floor closure is tainted onto the surface across the seam"
        );
        let surface = joined.authoring_surface();
        assert!(
            surface.contains(&"cal.consumer"),
            "the tainted consumer is counted"
        );
        assert!(
            surface.contains(&"floor.free_knob"),
            "the declared closure is counted"
        );
        assert_eq!(
            surface.len(),
            2,
            "both the closure and its tainted consumer are on the surface"
        );
    }

    #[test]
    fn the_phase_pin_survives_the_fold() {
        let cal = real_calibration();
        let floor = real_floor();
        let joined = JoinedRegister::build(&cal, &floor).expect("the fold succeeds");

        // The 7 candidate phases are measured + derive_first_defect. They must stay OFF the unified authoring
        // surface (a measured value is not authored), and the derive-first-defect punch-list (the
        // materials-buildout worklist) is unchanged by the fold.
        for id in joined.authoring_surface() {
            assert!(
                !id.starts_with("phase."),
                "no candidate phase is on the unified authoring surface (phases are measured), found {id}"
            );
        }
        assert_eq!(
            floor.derive_first_defects().len(),
            22,
            "the derive-first-defect count (13 bulk substances, the elastic-modulus axis the materials \
             modulus route targets, and 8 phases, enstatite added) survives the fold unchanged"
        );
    }
}
