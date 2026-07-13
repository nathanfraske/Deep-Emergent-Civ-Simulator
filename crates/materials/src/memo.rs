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

//! The kernel-provided memo: the one canonically-iterated cache for the pure proposer and disposer.
//!
//! Backed by a `BTreeMap` so iteration is in KEY order (canonical, R-CANON-WALK), never insertion order, so an
//! instantiation cannot roll its own insertion-order cache and smuggle a nondeterminism landmine one level
//! down (seam 5). Keys are the quantized composition-class and environment-bucket the owner's contract names;
//! a memo is a cache, never hashed into world state, so it is a pure performance structure.

use std::collections::BTreeMap;

/// A canonical memo: a cache from a quantized key to a computed value, iterated in key order.
#[derive(Debug, Clone, Default)]
pub struct Memo<K: Ord, V> {
    entries: BTreeMap<K, V>,
}

impl<K: Ord, V> Memo<K, V> {
    /// An empty memo.
    pub fn new() -> Self {
        Memo {
            entries: BTreeMap::new(),
        }
    }

    /// The cached value for a key, if present.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    /// Insert a value, returning the previous one if the key was present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.entries.insert(key, value)
    }

    /// The cached value for the key, computing and caching it with the pure closure on a miss. The closure
    /// must be a pure function of the key (the memoization contract), so a hit and a miss are indistinguishable
    /// to the caller.
    pub fn get_or_insert_with(&mut self, key: K, compute: impl FnOnce() -> V) -> &V {
        self.entries.entry(key).or_insert_with(compute)
    }

    /// The number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the memo is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Canonical (key-ordered) iteration, the sanctioned walk (R-CANON-WALK).
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn get_or_insert_with_computes_once_per_key() {
        let mut memo: Memo<u32, u32> = Memo::new();
        let calls = Cell::new(0);
        let compute = |k: u32| {
            calls.set(calls.get() + 1);
            k * 10
        };
        assert_eq!(*memo.get_or_insert_with(2, || compute(2)), 20);
        assert_eq!(*memo.get_or_insert_with(2, || compute(2)), 20);
        assert_eq!(calls.get(), 1, "a hit does not recompute");
        assert_eq!(*memo.get_or_insert_with(5, || compute(5)), 50);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn iteration_is_in_canonical_key_order_not_insertion_order() {
        let mut memo: Memo<u32, &str> = Memo::new();
        // Insert out of order; iterate in key order regardless.
        memo.insert(30, "c");
        memo.insert(10, "a");
        memo.insert(20, "b");
        let keys: Vec<u32> = memo.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            keys,
            vec![10, 20, 30],
            "iteration is key-ordered, never insertion-ordered"
        );
    }
}
