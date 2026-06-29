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

//! Event sourcing and the append-only history (design Part 7).
//!
//! History is not simulated continuously; it is logged and then queried. Every
//! meaningful occurrence becomes an immutable event carrying who, where, when, and
//! which entities it concerns. An entity's life story is a query over the log
//! filtered by [`StableId`].
//!
//! The event *mechanism* is fixed here: append-only storage, a never-reused
//! [`EventId`], and a `StableId`-keyed provenance index. The event *schema* is
//! left open. Design Part 7.1 carries the R-EVENT backlog flag, whose open question
//! is whether events should have a hardcoded type taxonomy at all. Per Principle 4,
//! the bedrock does not author a closed `EventKind` enum: a kind is a data-defined
//! [`EventKindId`], and the kind-specific fields ride in an opaque payload whose
//! schema is data. When R-EVENT is resolved, the resolution decides the payload
//! schema without changing this storage mechanism.

use crate::id::StableId;
use std::collections::HashMap;

/// A never-reused identifier for a logged event.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EventId(pub u64);

/// A data-defined event kind. The set of kinds lives in world data, not in a Rust
/// enum, pending the R-EVENT resolution (design Part 7.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EventKindId(pub u32);

/// An immutable logged occurrence.
///
/// `actors` are who did it; `subjects` are what it concerns (an artifact, a
/// building, a victim, a culture). Both are [`StableId`] references so they survive
/// promotion, demotion, and load. `payload` is opaque kind-specific data; its
/// layout is defined by the kind's data schema, not by this type.
#[derive(Clone, Debug)]
pub struct Event {
    /// Assigned by the log on append; the value passed in is ignored.
    pub id: EventId,
    /// The tick at which the event occurred.
    pub tick: u64,
    /// The data-defined kind.
    pub kind: EventKindId,
    /// Subtile location, as raw integer coordinates (kept canonical-friendly).
    pub location: (i32, i32, i16),
    /// Who acted.
    pub actors: Vec<StableId>,
    /// What the event concerns.
    pub subjects: Vec<StableId>,
    /// Kind-specific data, schema defined in data (R-EVENT).
    pub payload: Vec<u8>,
}

impl Event {
    /// A minimal event with no payload, for the common case and for tests.
    pub fn new(
        tick: u64,
        kind: EventKindId,
        actors: Vec<StableId>,
        subjects: Vec<StableId>,
    ) -> Self {
        Event {
            id: EventId(u64::MAX),
            tick,
            kind,
            location: (0, 0, 0),
            actors,
            subjects,
            payload: Vec::new(),
        }
    }

    /// Every entity the event references, actors then subjects, in order.
    pub fn referenced(&self) -> impl Iterator<Item = StableId> + '_ {
        self.actors
            .iter()
            .copied()
            .chain(self.subjects.iter().copied())
    }
}

/// An append-only event log with a provenance index from [`StableId`] to the events
/// that reference it. Appends happen only in the single-threaded events phase
/// (design Part 4.1), so the log needs no internal locking.
#[derive(Default)]
pub struct EventLog {
    events: Vec<Event>,
    by_entity: HashMap<StableId, Vec<EventId>>,
    next: u64,
}

impl EventLog {
    /// An empty log.
    pub fn new() -> Self {
        EventLog::default()
    }

    /// Append an event, assigning it the next id, and index it under every entity
    /// it references. Returns the assigned id.
    pub fn append(&mut self, mut e: Event) -> EventId {
        let id = EventId(self.next);
        self.next += 1;
        e.id = id;
        // Index the event once per distinct entity it references. An entity that is
        // both an actor and a subject (a self-directed act) must not be listed twice,
        // or any multiplicity-based fold over history over-counts (audit C-09).
        let mut refs: Vec<StableId> = e.referenced().collect();
        refs.sort();
        refs.dedup();
        for s in refs {
            self.by_entity.entry(s).or_default().push(id);
        }
        self.events.push(e);
        id
    }

    /// Total number of logged events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Fetch an event by id. Ids are dense and assigned in append order, so the id
    /// is the storage index. Panics if out of range.
    pub fn get(&self, id: EventId) -> &Event {
        &self.events[id.0 as usize]
    }

    /// Every event that references the given entity, in append order. Returns an
    /// empty iterator for an entity the log has never seen.
    pub fn history_of(&self, id: StableId) -> impl Iterator<Item = &Event> + '_ {
        self.by_entity
            .get(&id)
            .into_iter()
            .flatten()
            .map(move |eid| self.get(*eid))
    }

    /// All events in append order.
    pub fn iter(&self) -> impl Iterator<Item = &Event> + '_ {
        self.events.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_directed_event_is_indexed_once() {
        // Regression for the determinism audit C-09: when a StableId is both actor
        // and subject, the provenance index must list the event once, not twice.
        let mut log = EventLog::new();
        log.append(Event::new(
            1,
            EventKindId(0),
            vec![StableId(1)],
            vec![StableId(1)],
        ));
        assert_eq!(log.history_of(StableId(1)).count(), 1);
    }

    #[test]
    fn ids_are_dense_and_never_reused() {
        let mut log = EventLog::new();
        let a = log.append(Event::new(1, EventKindId(0), vec![StableId(1)], vec![]));
        let b = log.append(Event::new(2, EventKindId(0), vec![StableId(2)], vec![]));
        let c = log.append(Event::new(
            3,
            EventKindId(1),
            vec![StableId(1)],
            vec![StableId(2)],
        ));
        assert_eq!((a, b, c), (EventId(0), EventId(1), EventId(2)));
        assert_eq!(log.len(), 3);
        assert_eq!(log.get(a).tick, 1);
    }

    #[test]
    fn provenance_index_answers_history_queries() {
        let mut log = EventLog::new();
        // entity 1 is born, forges with entity 2, then dies.
        log.append(Event::new(10, EventKindId(0), vec![StableId(1)], vec![]));
        log.append(Event::new(
            20,
            EventKindId(5),
            vec![StableId(1)],
            vec![StableId(2)],
        ));
        log.append(Event::new(
            30,
            EventKindId(9),
            vec![StableId(3)],
            vec![StableId(1)],
        ));

        let ticks_of_1: Vec<u64> = log.history_of(StableId(1)).map(|e| e.tick).collect();
        assert_eq!(ticks_of_1, vec![10, 20, 30], "all three reference entity 1");

        let ticks_of_2: Vec<u64> = log.history_of(StableId(2)).map(|e| e.tick).collect();
        assert_eq!(
            ticks_of_2,
            vec![20],
            "only the forge references entity 2 as a subject"
        );

        assert_eq!(
            log.history_of(StableId(404)).count(),
            0,
            "unknown entity has no history"
        );
    }

    #[test]
    fn append_assigns_id_ignoring_provided_value() {
        let mut log = EventLog::new();
        let mut e = Event::new(1, EventKindId(0), vec![], vec![]);
        e.id = EventId(99);
        let id = log.append(e);
        assert_eq!(id, EventId(0), "the log assigns the id");
    }
}
