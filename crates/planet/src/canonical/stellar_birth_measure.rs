//! Fail-closed contract for the first physical realization measure.
//!
//! The requirement graph is deliberately smaller than a world-seed schema. A
//! single joint physical measure preserves every correlation in its support;
//! this contract does not split it into authored scalar roots, independent
//! marginals, environment classes, or familiar-system labels. A separate
//! coordinate law is required before generated contingency can sample that
//! measure. Each node names a derive-first, admit-after-exhaustion, or refuse
//! obligation. None of the names admits a value.

use super::floor_magnitudes::AuditedFloorView;
use std::fmt;

/// The Stage 1 capability required by the canonical pipeline.
pub(crate) const STELLAR_BIRTH_REALIZATION_MEASURE: &str = "stellar_birth.realization_measure";

const STELLAR_BIRTH_JOINT_PHYSICAL_MEASURE: &str = "stellar_birth.joint_physical_measure";
const STELLAR_BIRTH_REALIZATION_COORDINATE_LAW: &str = "stellar_birth.realization_coordinate_law";

const NO_REQUIREMENTS: &[&str] = &[];
const REALIZATION_REQUIREMENTS: &[&str] = &[
    STELLAR_BIRTH_JOINT_PHYSICAL_MEASURE,
    STELLAR_BIRTH_REALIZATION_COORDINATE_LAW,
];

/// The physical role of one node in the fixed Stage 1 requirement graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequirementKind {
    JointPhysicalMeasure,
    RealizationCoordinateLaw,
    RealizationMeasure,
}

/// Every node follows the canonical two-route rule and fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequirementObligation {
    DeriveFirstAdmitAfterExhaustionOrRefuse,
}

/// One obligation in the static stellar-birth requirement graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MeasureRequirement {
    id: &'static str,
    kind: RequirementKind,
    requires: &'static [&'static str],
    obligation: RequirementObligation,
}

const DERIVE_ADMIT_OR_REFUSE: RequirementObligation =
    RequirementObligation::DeriveFirstAdmitAfterExhaustionOrRefuse;

const REQUIREMENT_GRAPH: [MeasureRequirement; 3] = [
    MeasureRequirement {
        id: STELLAR_BIRTH_JOINT_PHYSICAL_MEASURE,
        kind: RequirementKind::JointPhysicalMeasure,
        requires: NO_REQUIREMENTS,
        obligation: DERIVE_ADMIT_OR_REFUSE,
    },
    MeasureRequirement {
        id: STELLAR_BIRTH_REALIZATION_COORDINATE_LAW,
        kind: RequirementKind::RealizationCoordinateLaw,
        requires: NO_REQUIREMENTS,
        obligation: DERIVE_ADMIT_OR_REFUSE,
    },
    MeasureRequirement {
        id: STELLAR_BIRTH_REALIZATION_MEASURE,
        kind: RequirementKind::RealizationMeasure,
        requires: REALIZATION_REQUIREMENTS,
        obligation: DERIVE_ADMIT_OR_REFUSE,
    },
];

const ROOT_INDEX: usize = 2;

fn root_requirement() -> &'static MeasureRequirement {
    &REQUIREMENT_GRAPH[ROOT_INDEX]
}

/// Opaque proof that the complete stellar-birth measure contract has closed.
///
/// This type has no constructor. A future repository-owned constructor belongs
/// here only after the joint measure and coordinate law have each derived or
/// completed every required floor-admission receipt. Ledger identity, evidence
/// custody, and a caller-provided coordinate cannot construct the capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StellarBirthMeasureCapability {
    _sealed: CapabilitySeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CapabilitySeal;

/// Typed reason Stage 1 cannot construct its stellar-birth measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StellarBirthMeasureRefusal {
    requirement_id: &'static str,
}

impl StellarBirthMeasureRefusal {
    pub(crate) const fn requirement_id(self) -> &'static str {
        self.requirement_id
    }
}

impl fmt::Display for StellarBirthMeasureRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "derived or admitted absolute-floor measure '{}'",
            self.requirement_id()
        )
    }
}

/// Require the closed measure contract before Stage 1 can generate a draw.
///
/// The current three-invariant floor closes neither leaf. The audited floor view
/// is accepted only to keep this check downstream of exact floor validation;
/// it is not a generic lookup or a value-binding surface.
pub(crate) fn require_birth_measure(
    _floor: &AuditedFloorView<'_>,
) -> Result<StellarBirthMeasureCapability, StellarBirthMeasureRefusal> {
    let root = root_requirement();
    debug_assert_eq!(root.id, STELLAR_BIRTH_REALIZATION_MEASURE);
    debug_assert_eq!(root.kind, RequirementKind::RealizationMeasure);
    debug_assert_eq!(root.obligation, DERIVE_ADMIT_OR_REFUSE);
    Err(StellarBirthMeasureRefusal {
        requirement_id: root.id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::sealed_absolute_physics_floor;
    use civsim_ledger::AbsolutePhysicsFloor;
    use std::collections::BTreeSet;

    fn audited_floor() -> AbsolutePhysicsFloor {
        sealed_absolute_physics_floor().expect("the audited physical catalog is admissible")
    }

    #[test]
    fn the_requirement_graph_is_exact_and_rooted_at_the_present_refusal() {
        assert_eq!(REQUIREMENT_GRAPH.len(), 3);
        assert_eq!(
            REQUIREMENT_GRAPH,
            [
                MeasureRequirement {
                    id: "stellar_birth.joint_physical_measure",
                    kind: RequirementKind::JointPhysicalMeasure,
                    requires: &[],
                    obligation: DERIVE_ADMIT_OR_REFUSE,
                },
                MeasureRequirement {
                    id: "stellar_birth.realization_coordinate_law",
                    kind: RequirementKind::RealizationCoordinateLaw,
                    requires: &[],
                    obligation: DERIVE_ADMIT_OR_REFUSE,
                },
                MeasureRequirement {
                    id: "stellar_birth.realization_measure",
                    kind: RequirementKind::RealizationMeasure,
                    requires: &[
                        "stellar_birth.joint_physical_measure",
                        "stellar_birth.realization_coordinate_law",
                    ],
                    obligation: DERIVE_ADMIT_OR_REFUSE,
                },
            ]
        );
        assert_eq!(root_requirement().id, STELLAR_BIRTH_REALIZATION_MEASURE);
    }

    #[test]
    fn every_dependency_resolves_and_the_requirement_graph_is_acyclic() {
        let ids: BTreeSet<_> = REQUIREMENT_GRAPH.iter().map(|node| node.id).collect();
        assert_eq!(ids.len(), REQUIREMENT_GRAPH.len());
        assert!(REQUIREMENT_GRAPH
            .iter()
            .flat_map(|node| node.requires.iter())
            .all(|dependency| ids.contains(dependency)));

        fn visit(
            id: &'static str,
            visiting: &mut BTreeSet<&'static str>,
            visited: &mut BTreeSet<&'static str>,
        ) {
            assert!(visiting.insert(id), "cycle reaches {id}");
            let node = REQUIREMENT_GRAPH
                .iter()
                .find(|node| node.id == id)
                .expect("every dependency resolves to one graph node");
            for dependency in node.requires {
                if !visited.contains(dependency) {
                    visit(dependency, visiting, visited);
                }
            }
            assert!(visiting.remove(id));
            visited.insert(id);
        }

        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        visit(root_requirement().id, &mut visiting, &mut visited);
        assert_eq!(visited, ids);
    }

    #[test]
    fn the_joint_measure_remains_one_correlation_preserving_obligation() {
        let joint = &REQUIREMENT_GRAPH[0];
        assert_eq!(joint.kind, RequirementKind::JointPhysicalMeasure);
        assert!(joint.requires.is_empty());
        assert_eq!(joint.obligation, DERIVE_ADMIT_OR_REFUSE);
        assert!(REQUIREMENT_GRAPH.iter().all(|node| {
            !node.id.contains("epoch")
                && !node.id.contains("environment")
                && !node.id.contains("metallicity")
                && !node.id.contains("scalar")
        }));
    }

    #[test]
    fn the_current_floor_refuses_at_the_graph_root() {
        let floor = audited_floor();
        let floor_view =
            AuditedFloorView::from_floor(&floor).expect("the audited floor has typed magnitudes");
        let refusal = require_birth_measure(&floor_view)
            .expect_err("the three-invariant floor cannot close the birth measure contract");

        assert_eq!(refusal.requirement_id(), STELLAR_BIRTH_REALIZATION_MEASURE);
        assert_eq!(
            refusal.to_string(),
            "derived or admitted absolute-floor measure 'stellar_birth.realization_measure'"
        );
    }
}
