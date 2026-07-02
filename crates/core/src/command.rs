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

//! Deterministic command ordering and the event-queue substrate (design Part 4.3 and
//! Part 57; R-CMD-ORDER).
//!
//! A parallel tick lets many workers emit commands (spawn an entity, promote a region,
//! emit an event) at once, and a future-event list lets a region or an agent schedule
//! work at a later tick. Both are determinism traps unless the order in which commands
//! apply, and events fire, is a pure function of the data rather than of the thread that
//! produced them or the moment they were buffered. This module provides the two pieces
//! that close that trap:
//!
//! - [`CommandKey`], the total order applied to every command and scheduled event
//!   (R-CMD-ORDER): the tick, then the primary entity, then the kind, then an emission
//!   ordinal that breaks any remaining tie. With a total key, any ordering keyed by it
//!   is a pure function of the command set, independent of insertion order and worker
//!   count.
//! - [`EventQueue`], the deterministic future-event list both engine levers drain: the
//!   agent wake queue of event-driven execution (R-AGENT-EXEC, Part 57) and the
//!   cross-region synchronization core of temporal level of detail (R-TEMPORAL-LOD,
//!   Part 32) are the same substrate at agent and region granularity.
//!
//! The companion mechanism, [`content_id`], derives an identifier from canonical content
//! rather than from emission order, so an event or a spawned entity gets the same id on
//! every replay whatever worker produced it. Spawn ids are then assigned at the
//! single-threaded barrier by walking the commands in [`CommandKey`] order (see
//! [`crate::canonical_sorted`]), never in the parallel stage.
//!
//! This is the shared substrate prototyped in isolation, as Part 57 asks, before it is
//! wired into the tick. It writes no canonical state on its own; it orders what the
//! caller feeds it.

use crate::canonical::canonical_sorted;
use crate::hash::StateHasher;
use crate::id::StableId;
use std::collections::BTreeMap;

/// The total order applied to every command and scheduled event so application never
/// depends on the thread that emitted it or the order it was buffered in (R-CMD-ORDER,
/// design Part 4.3 and Part 57).
///
/// The fields compare lexicographically in declaration order: the `tick` first (so an
/// event queue drains in time order), then the `primary` entity the command concerns,
/// then the command or event `kind`, then the emission `ordinal` that breaks any
/// remaining tie. The ordinal is the caller's contract: it must make the key unique per
/// `(tick, primary, kind)`, so the key is a total order and any sequence keyed by it is
/// a pure function of the command set rather than of insertion order or worker count.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CommandKey {
    /// The tick the command applies at, or the event fires at.
    pub tick: u64,
    /// The primary entity the command concerns; the first tie-break after the tick.
    pub primary: StableId,
    /// The command or event kind discriminant; the second tie-break.
    pub kind: u32,
    /// The emission ordinal, which must make the key unique per `(tick, primary, kind)`.
    pub ordinal: u64,
}

impl CommandKey {
    /// A key from its four fields.
    pub fn new(tick: u64, primary: StableId, kind: u32, ordinal: u64) -> Self {
        CommandKey {
            tick,
            primary,
            kind,
            ordinal,
        }
    }

    /// Fold the key into a hasher in canonical field order, for content-addressed ids
    /// and the state hash.
    #[inline]
    pub fn write(&self, h: &mut StateHasher) {
        h.write_u64(self.tick);
        h.write_stable(self.primary);
        h.write_u32(self.kind);
        h.write_u64(self.ordinal);
    }
}

/// Derive a never-reused identifier from canonical content rather than from emission
/// order (R-CMD-ORDER: "make the id a deterministic function of canonical content"), so
/// an event or a spawned entity gets the same id on every replay whatever worker
/// produced it and whenever it was produced. The caller feeds the content in a fixed
/// canonical order through the hasher; the id is the low 64 bits of the 128-bit
/// state hash of that content, so two distinct contents collide only on a 64-bit hash
/// collision, and identical content always yields the identical id.
pub fn content_id(feed: impl FnOnce(&mut StateHasher)) -> u64 {
    let mut h = StateHasher::new();
    feed(&mut h);
    h.finish() as u64
}

/// A deterministic future-event list: the substrate both event-driven agent execution
/// (R-AGENT-EXEC, design Part 57) and temporal level of detail's cross-region
/// synchronization (R-TEMPORAL-LOD, design Part 32) drain, one structure at agent and
/// region granularity.
///
/// Events are ordered by [`CommandKey`], so draining yields them in total order
/// independent of the order they were scheduled and the worker that scheduled them: the
/// drained sequence is a pure function of the scheduled set. It is backed by a
/// `BTreeMap`, so scheduling and popping the earliest event are logarithmic, ordered
/// iteration is by key, and the key uniqueness the total order requires is enforced by
/// the map (a colliding schedule is surfaced, not silently reordered).
pub struct EventQueue<P> {
    scheduled: BTreeMap<CommandKey, P>,
}

impl<P> EventQueue<P> {
    /// An empty queue.
    pub fn new() -> Self {
        EventQueue {
            scheduled: BTreeMap::new(),
        }
    }

    /// The number of scheduled events.
    pub fn len(&self) -> usize {
        self.scheduled.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.scheduled.is_empty()
    }

    /// Schedule `payload` to fire at `key`. Returns the displaced payload if `key` was
    /// already present, which is a determinism-contract violation the caller must not
    /// cause: the emission ordinal must make each key unique, so a well-formed caller
    /// always gets `None`. A `Some` return is a defect to surface, not to ignore, since
    /// two events sharing a key would otherwise have no defined order.
    pub fn schedule(&mut self, key: CommandKey, payload: P) -> Option<P> {
        self.scheduled.insert(key, payload)
    }

    /// The earliest scheduled event in total order, without removing it.
    pub fn peek(&self) -> Option<(&CommandKey, &P)> {
        self.scheduled.first_key_value()
    }

    /// Remove and return the earliest scheduled event in total order.
    pub fn pop(&mut self) -> Option<(CommandKey, P)> {
        self.scheduled.pop_first()
    }

    /// Remove and return, in total order, every event whose tick is at or before
    /// `tick`: the events due to fire by a given point on the timeline. A woken agent
    /// draining its due wakes, or a promoted region catching up to the global clock,
    /// calls this. Events scheduled for a later tick stay in the queue.
    pub fn drain_due(&mut self, tick: u64) -> Vec<(CommandKey, P)> {
        let mut out = Vec::new();
        while self
            .scheduled
            .first_key_value()
            .is_some_and(|(k, _)| k.tick <= tick)
        {
            out.push(self.scheduled.pop_first().expect("non-empty by the guard"));
        }
        out
    }
}

impl<P> Default for EventQueue<P> {
    fn default() -> Self {
        EventQueue::new()
    }
}

/// The single-threaded barrier that applies a tick's buffered commands in total order:
/// the `ActionApply` phase of the tick (design Part 4.1; R-CMD-ORDER, Part 4.3).
///
/// During the parallel `ActionStage`, workers push commands (a spawn, a promotion, an
/// event emission, any structural change) into the buffer without coordinating, in
/// whatever order and interleaving the scheduler gives them. At the barrier the buffer
/// is drained in [`CommandKey`] order, so structural change applies as a pure function
/// of the command set rather than of the worker that produced each command or the moment
/// it arrived. That is the determinism the total order buys by construction: the applied
/// sequence, and any id minted while draining, are independent of the thread count. A
/// spawn command carrying no id yet is assigned one inside the drain, in canonical order,
/// so ids are minted at the barrier and never in the parallel stage.
pub struct CommandBuffer<C> {
    pending: Vec<(CommandKey, C)>,
}

impl<C> CommandBuffer<C> {
    /// An empty buffer.
    pub fn new() -> Self {
        CommandBuffer {
            pending: Vec::new(),
        }
    }

    /// The number of buffered commands.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Buffer a command, as a worker does during the parallel stage. The order of pushes
    /// carries no meaning: the barrier re-orders by [`CommandKey`], so two runs that push
    /// the same commands in different orders apply them identically.
    pub fn push(&mut self, key: CommandKey, command: C) {
        self.pending.push((key, command));
    }

    /// Consume the buffer and return its commands in total [`CommandKey`] order, the
    /// canonical order the barrier applies them in. Reuses the R-CANON-WALK canonical
    /// walk (design Part 3.5), so the result is a pure function of the buffered set given
    /// unique keys (the emission ordinal is the caller's uniqueness contract).
    pub fn into_ordered(self) -> Vec<(CommandKey, C)> {
        canonical_sorted(self.pending, |pair: &(CommandKey, C)| pair.0)
    }

    /// Drain the buffer at the barrier, applying each command in total [`CommandKey`]
    /// order. A closure that mints ids or appends events sees the commands in the
    /// canonical order, so whatever it produces is independent of the push order and the
    /// worker count.
    pub fn apply_ordered(self, mut apply: impl FnMut(CommandKey, C)) {
        for (key, command) in self.into_ordered() {
            apply(key, command);
        }
    }
}

impl<C> Default for CommandBuffer<C> {
    fn default() -> Self {
        CommandBuffer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(tick: u64, primary: u64, kind: u32, ordinal: u64) -> CommandKey {
        CommandKey::new(tick, StableId(primary), kind, ordinal)
    }

    #[test]
    fn command_key_orders_lexicographically() {
        // tick dominates, then primary, then kind, then ordinal.
        assert!(key(1, 9, 9, 9) < key(2, 0, 0, 0), "earlier tick is first");
        assert!(key(5, 1, 9, 9) < key(5, 2, 0, 0), "then lower primary");
        assert!(key(5, 2, 1, 9) < key(5, 2, 2, 0), "then lower kind");
        assert!(key(5, 2, 2, 1) < key(5, 2, 2, 2), "then lower ordinal");
    }

    #[test]
    fn queue_drains_in_key_order_regardless_of_insertion() {
        // The determinism proof: insertion-order-independence is what makes the drained
        // sequence independent of worker count, since threads differ only in the order
        // and interleaving of their inserts.
        let items = [
            (key(3, 1, 0, 0), "c"),
            (key(1, 5, 0, 0), "a"),
            (key(1, 5, 0, 1), "b"),
            (key(9, 0, 2, 0), "e"),
            (key(3, 1, 1, 0), "d"),
        ];
        let expected = {
            let mut q = EventQueue::new();
            for (k, v) in items {
                assert!(q.schedule(k, v).is_none());
            }
            let mut out = Vec::new();
            while let Some(e) = q.pop() {
                out.push(e);
            }
            out
        };

        // Two different insertion orders (reversed, and a rotation) must produce the
        // identical drained sequence.
        for permute in [
            |xs: &[(CommandKey, &'static str)]| xs.iter().rev().copied().collect::<Vec<_>>(),
            |xs: &[(CommandKey, &'static str)]| {
                let mut v: Vec<_> = xs.to_vec();
                v.rotate_left(2);
                v
            },
        ] {
            let mut q = EventQueue::new();
            for (k, v) in permute(&items) {
                q.schedule(k, v);
            }
            let mut out = Vec::new();
            while let Some(e) = q.pop() {
                out.push(e);
            }
            assert_eq!(out, expected, "drain order is a pure function of the set");
        }

        // And it is the sorted order.
        let mut sorted: Vec<_> = items.to_vec();
        sorted.sort_by_key(|(k, _)| *k);
        assert_eq!(expected, sorted);
    }

    #[test]
    fn drain_due_returns_events_through_a_tick_in_order() {
        let mut q = EventQueue::new();
        q.schedule(key(10, 1, 0, 0), "a");
        q.schedule(key(10, 2, 0, 0), "b");
        q.schedule(key(20, 1, 0, 0), "c");
        q.schedule(key(5, 3, 0, 0), "d");

        let due = q.drain_due(10);
        let got: Vec<&str> = due.iter().map(|(_, v)| *v).collect();
        assert_eq!(
            got,
            vec!["d", "a", "b"],
            "everything through tick 10, in order"
        );
        assert_eq!(q.len(), 1, "the tick-20 event remains");
        assert_eq!(q.pop().unwrap().1, "c");
    }

    #[test]
    fn schedule_signals_a_key_collision() {
        // A colliding key is a determinism-contract violation (the ordinal must
        // disambiguate); the queue surfaces it as a Some return rather than reordering
        // silently.
        let mut q = EventQueue::new();
        assert!(q.schedule(key(1, 1, 0, 0), "first").is_none());
        assert_eq!(
            q.schedule(key(1, 1, 0, 0), "second"),
            Some("first"),
            "the displaced payload is returned so the caller can fail loud"
        );
    }

    #[test]
    fn content_id_is_a_pure_function_of_content() {
        let make = |tick: u64, actor: u64| {
            content_id(|h| {
                h.write_u64(tick);
                h.write_stable(StableId(actor));
            })
        };
        assert_eq!(make(7, 3), make(7, 3), "same content, same id, any time");
        assert_ne!(make(7, 3), make(7, 4), "different content, different id");
        assert_ne!(make(7, 3), make(8, 3), "order and value both matter");

        // It agrees with feeding the hasher directly and truncating to 64 bits.
        let mut h = StateHasher::new();
        h.write_u64(7);
        h.write_stable(StableId(3));
        assert_eq!(make(7, 3), h.finish() as u64);
    }

    #[test]
    fn command_buffer_applies_in_key_order_independent_of_push_order() {
        // The ActionApply barrier: whatever order workers push commands in, the barrier
        // drains them in CommandKey order, so both the applied sequence and any id minted
        // while draining are a pure function of the command set. This is the
        // thread-count-independence R-CMD-ORDER guarantees by construction, proven by
        // pushing the same commands in two different interleavings (as two different
        // worker counts would) and asserting identical output.
        let commands = [
            (key(2, 3, 0, 0), "promote"),
            (key(1, 7, 5, 0), "spawn-a"),
            (key(1, 7, 5, 1), "spawn-b"),
            (key(2, 3, 0, 1), "emit"),
            (key(1, 4, 5, 0), "spawn-c"),
        ];

        // Drain a buffer built from a given push order, minting a sequential id for each
        // "spawn" command as the barrier reaches it, and record (command, minted id).
        let run = |push_order: Vec<(CommandKey, &'static str)>| {
            let mut buf = CommandBuffer::new();
            for (k, c) in push_order {
                buf.push(k, c);
            }
            let mut applied: Vec<(&'static str, Option<StableId>)> = Vec::new();
            let mut next_spawn_id = 500u64;
            buf.apply_ordered(|_key, command| {
                let minted = if command.starts_with("spawn") {
                    let id = StableId(next_spawn_id);
                    next_spawn_id += 1;
                    Some(id)
                } else {
                    None
                };
                applied.push((command, minted));
            });
            applied
        };

        let forward = run(commands.to_vec());
        let reversed = run(commands.iter().rev().copied().collect());
        let mut rotated = commands.to_vec();
        rotated.rotate_left(3);
        let rotated = run(rotated);

        assert_eq!(forward, reversed, "push order does not change application");
        assert_eq!(forward, rotated, "nor does a different interleaving");
        // And the minted spawn ids follow the canonical order: spawn-c (tick 1, primary 4)
        // before spawn-a and spawn-b (tick 1, primary 7).
        let spawn_ids: Vec<_> = forward
            .iter()
            .filter(|(c, _)| c.starts_with("spawn"))
            .map(|(c, id)| (*c, id.unwrap()))
            .collect();
        assert_eq!(
            spawn_ids,
            vec![
                ("spawn-c", StableId(500)),
                ("spawn-a", StableId(501)),
                ("spawn-b", StableId(502)),
            ],
            "spawn ids minted at the barrier in CommandKey order"
        );
    }

    #[test]
    fn spawn_ids_minted_in_key_order_are_insertion_independent() {
        // "Mint spawn ids at the single-threaded barrier": the barrier sorts the spawn
        // commands by CommandKey (reusing the R-CANON-WALK helper) and assigns ids in
        // that order from the registry high-water mark, so the id a spawn receives does
        // not depend on which worker emitted it.
        let spawns_a = [key(4, 2, 7, 0), key(4, 1, 7, 0), key(4, 2, 7, 1)];
        let spawns_b = [key(4, 2, 7, 1), key(4, 2, 7, 0), key(4, 1, 7, 0)]; // different order

        let assign = |spawns: &[CommandKey], base: u64| -> Vec<(CommandKey, StableId)> {
            canonical_sorted(spawns.iter().copied(), |k| *k)
                .into_iter()
                .enumerate()
                .map(|(i, k)| (k, StableId(base + i as u64)))
                .collect()
        };

        assert_eq!(
            assign(&spawns_a, 100),
            assign(&spawns_b, 100),
            "each spawn key gets the same id regardless of emission order"
        );
    }
}
