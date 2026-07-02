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

//! The deterministic scheduler (design Part 57, `docs/deterministic_scheduler_design.md`), the
//! keystone of the determinism cluster.
//!
//! Each system (a tick phase, or a finer unit of work) declares an [`Access`] over named data
//! resources: the resources it reads and the resources it writes. The scheduler derives, as a pure
//! function of those declarations, a deterministic execution order plus conflict-free parallel
//! batches. Two systems conflict when one writes a resource the other reads or writes; conflicting
//! systems land in different batches and run in the order of their stable [`SystemId`], while
//! non-conflicting systems may share a batch and run concurrently.
//!
//! Because the schedule is a pure function of the sorted declarations, it is deterministic and
//! observer-independent (Principles 3 and 10), and it is cheap to tune: changing a declaration
//! re-derives the schedule with no rewrite (Principle 11, the access is data a reviewer reads, not a
//! borrow signature inferred by a compiler). The [`SystemId`] must be a stable canonical id assigned
//! from a sorted declaration, never from registration order, or the schedule would depend on load
//! order and cease to be deterministic.
//!
//! The scheduler is storage-agnostic: a [`ResourceId`] names a resource whether it is a `BTreeMap`
//! store today or a component column later, so it does not force the storage decision. And it is
//! parallelism-ready without requiring parallelism: [`run_serial`] flattens the batches and runs each
//! system in one thread, so the schedule is exercised and proven against the current serial tick
//! before any parallel executor is switched on. A parallel executor is the same shape with the
//! systems of one batch run concurrently, which is sound precisely because a batch holds no
//! conflicting pair.
//!
//! The honest limit: the scheduler proves the schedule conflict-free and deterministic, not that any
//! system declared its access correctly. A mis-declared read or write is a correctness bug the
//! declaration review, and the tick-hash equivalence test, must catch.

use std::collections::{BTreeMap, BTreeSet};

/// A named canonical data resource a system reads or writes (a per-being store, the event log, an RNG
/// stream, a field). A data-defined id, so a new resource is an id rather than a code change.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ResourceId(pub u32);

/// A stable canonical id for a system (a tick phase or a finer unit of work). Assigned from a sorted
/// declaration like the phase registry, never from registration order, so the schedule is a pure
/// function of the declarations and not of load order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SystemId(pub u32);

/// A system's declared read-set and write-set over [`ResourceId`]s. Ordered sets, so a declaration is
/// canonical and two declarations of the same access compare equal regardless of insertion order.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Access {
    /// The resources the system reads.
    pub reads: BTreeSet<ResourceId>,
    /// The resources the system writes.
    pub writes: BTreeSet<ResourceId>,
}

impl Access {
    /// A declaration from its read and write resource ids.
    pub fn new(
        reads: impl IntoIterator<Item = ResourceId>,
        writes: impl IntoIterator<Item = ResourceId>,
    ) -> Self {
        Access {
            reads: reads.into_iter().collect(),
            writes: writes.into_iter().collect(),
        }
    }

    /// Whether this system conflicts with `other`: one writes a resource the other reads or writes.
    /// Two pure readers of the same resource do not conflict; a writer conflicts with any reader or
    /// writer of the same resource. The relation is symmetric.
    pub fn conflicts_with(&self, other: &Access) -> bool {
        self.writes
            .iter()
            .any(|r| other.writes.contains(r) || other.reads.contains(r))
            || self.reads.iter().any(|r| other.writes.contains(r))
    }
}

/// Derive the deterministic layered schedule from the declared accesses: a sequence of batches, each
/// a set of systems with no pairwise conflict, such that every conflicting pair runs in [`SystemId`]
/// order across batches. A pure function of the sorted declarations.
///
/// The layering walks systems in ascending [`SystemId`] order and places each in the earliest batch
/// after every lower-id system it conflicts with. That batch never already holds a conflicting
/// member (a conflicting lower-id system sits in a strictly earlier batch by construction), so the
/// batch is conflict-free and the assignment is unambiguous.
pub fn schedule(systems: &BTreeMap<SystemId, Access>) -> Vec<Vec<SystemId>> {
    let mut batches: Vec<Vec<SystemId>> = Vec::new();
    let mut batch_of: BTreeMap<SystemId, usize> = BTreeMap::new();
    // BTreeMap iterates in ascending SystemId order, the canonical processing order.
    for (&sid, access) in systems {
        let mut earliest = 0usize;
        for (&other, other_access) in systems {
            if other >= sid {
                break; // only lower ids are already placed
            }
            if access.conflicts_with(other_access) {
                earliest = earliest.max(batch_of[&other] + 1);
            }
        }
        while batches.len() <= earliest {
            batches.push(Vec::new());
        }
        batches[earliest].push(sid);
        batch_of.insert(sid, earliest);
    }
    batches
}

/// Flatten a schedule into the single canonical execution order (batch order, then ascending
/// [`SystemId`] within a batch): the order the serial executor runs, and the order a determinism
/// proof compares against.
pub fn flatten(schedule: &[Vec<SystemId>]) -> Vec<SystemId> {
    schedule.iter().flat_map(|b| b.iter().copied()).collect()
}

/// Run the schedule serially: every system in batch order, in ascending [`SystemId`] within a batch,
/// on one thread. This exercises and proves the schedule against the current serial tick without yet
/// parallelising. A parallel executor runs the systems of one batch concurrently, which is sound
/// because a batch holds no conflicting pair.
pub fn run_serial(schedule: &[Vec<SystemId>], mut run_system: impl FnMut(SystemId)) {
    for batch in schedule {
        for &sid in batch {
            run_system(sid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(ids: impl IntoIterator<Item = u32>) -> impl Iterator<Item = ResourceId> {
        ids.into_iter().map(ResourceId)
    }

    fn systems(defs: &[(u32, &[u32], &[u32])]) -> BTreeMap<SystemId, Access> {
        defs.iter()
            .map(|&(s, r, w)| (SystemId(s), Access::new(res(r.to_vec()), res(w.to_vec()))))
            .collect()
    }

    #[test]
    fn conflict_is_write_versus_any_and_symmetric() {
        let writer = Access::new(res([]), res([1]));
        let reader = Access::new(res([1]), res([]));
        let other_reader = Access::new(res([1]), res([]));
        assert!(writer.conflicts_with(&reader), "write vs read conflicts");
        assert!(reader.conflicts_with(&writer), "and it is symmetric");
        assert!(
            writer.conflicts_with(&writer.clone()),
            "write vs write conflicts"
        );
        assert!(
            !reader.conflicts_with(&other_reader),
            "two pure readers of one resource do not conflict"
        );
    }

    #[test]
    fn independent_systems_share_the_first_batch() {
        // Three systems touching disjoint resources have no conflict, so they parallelise into one
        // batch (the maximum parallelism the declarations allow).
        let s = systems(&[(0, &[], &[10]), (1, &[], &[20]), (2, &[], &[30])]);
        let sch = schedule(&s);
        assert_eq!(sch.len(), 1, "no conflicts means one batch");
        assert_eq!(sch[0], vec![SystemId(0), SystemId(1), SystemId(2)]);
    }

    #[test]
    fn conflicting_systems_layer_and_order_by_id() {
        // A chain: 0 writes r1, 1 reads r1 (conflict, must follow 0), 2 writes r1 (conflict with
        // both, must follow 1). A fourth system 3 on a disjoint resource joins the earliest batch.
        let s = systems(&[
            (0, &[], &[1]),
            (1, &[1], &[]),
            (2, &[], &[1]),
            (3, &[], &[99]),
        ]);
        let sch = schedule(&s);
        assert_eq!(sch.len(), 3, "the r1 chain forces three layers");
        assert_eq!(
            sch[0],
            vec![SystemId(0), SystemId(3)],
            "0 and the disjoint 3 lead"
        );
        assert_eq!(sch[1], vec![SystemId(1)]);
        assert_eq!(sch[2], vec![SystemId(2)]);
    }

    #[test]
    fn schedule_is_a_pure_function_of_the_declarations() {
        // The input is a BTreeMap, so there is no insertion order to leak; the schedule is a
        // function of the (sorted) declarations alone. Building the same set of systems by two
        // different insertion sequences yields the identical schedule.
        let defs: [(u32, &[u32], &[u32]); 5] = [
            (3, &[1], &[2]),
            (1, &[], &[1]),
            (4, &[2], &[]),
            (2, &[1], &[1]),
            (0, &[3], &[3]),
        ];
        let forward: BTreeMap<SystemId, Access> = defs
            .iter()
            .map(|&(s, r, w)| (SystemId(s), Access::new(res(r.to_vec()), res(w.to_vec()))))
            .collect();
        let mut rev = defs;
        rev.reverse();
        let reversed: BTreeMap<SystemId, Access> = rev
            .iter()
            .map(|&(s, r, w)| (SystemId(s), Access::new(res(r.to_vec()), res(w.to_vec()))))
            .collect();
        assert_eq!(
            schedule(&forward),
            schedule(&reversed),
            "insertion order must not matter"
        );
    }

    #[test]
    fn every_batch_is_conflict_free_and_conflicts_respect_id_order() {
        // A larger mixed set. Assert the two invariants a parallel executor relies on: no batch holds
        // a conflicting pair, and every conflicting pair runs in ascending SystemId order.
        let s = systems(&[
            (0, &[1], &[2]),
            (1, &[2], &[3]),
            (2, &[], &[2]),
            (3, &[3], &[]),
            (4, &[9], &[9]),
            (5, &[9], &[]),
        ]);
        let sch = schedule(&s);
        let mut batch_of: BTreeMap<SystemId, usize> = BTreeMap::new();
        for (b, batch) in sch.iter().enumerate() {
            for &sid in batch {
                batch_of.insert(sid, b);
            }
        }
        for (&a, aacc) in &s {
            for (&c, cacc) in &s {
                if a < c && aacc.conflicts_with(cacc) {
                    assert!(
                        batch_of[&a] < batch_of[&c],
                        "conflicting {a:?} and {c:?} must run in id order across batches"
                    );
                }
            }
        }
        for batch in &sch {
            for (i, &a) in batch.iter().enumerate() {
                for &c in &batch[i + 1..] {
                    assert!(
                        !s[&a].conflicts_with(&s[&c]),
                        "a batch must hold no conflicting pair ({a:?}, {c:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn the_serial_executor_respects_declared_dependencies() {
        // The determinism the scheduler buys: running systems in the flattened schedule order gives
        // the same result as any dependency-respecting serialization, because conflicting systems run
        // in id order. Here systems mutate a shared cell through a declared resource; the schedule's
        // order must reproduce the id-order result.
        let s = systems(&[
            (0, &[], &[1]),  // set the cell to 10
            (1, &[1], &[1]), // add 5
            (2, &[1], &[1]), // double it
        ]);
        let sch = schedule(&s);
        // Three writers of resource 1 must fully serialise in id order.
        assert_eq!(flatten(&sch), vec![SystemId(0), SystemId(1), SystemId(2)]);
        let mut cell = 0i64;
        run_serial(&sch, |sid| match sid {
            SystemId(0) => cell = 10,
            SystemId(1) => cell += 5,
            SystemId(2) => cell *= 2,
            _ => unreachable!(),
        });
        assert_eq!(
            cell, 30,
            "the schedule reproduced the id-order computation ((10 + 5) * 2)"
        );
    }

    #[test]
    fn an_empty_system_set_schedules_to_no_batches() {
        assert!(schedule(&BTreeMap::new()).is_empty());
    }
}
