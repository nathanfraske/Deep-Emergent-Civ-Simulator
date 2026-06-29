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

//! Arena and slab allocation (design Part 2.3).
//!
//! [`Arena`] gives contiguous layout, cheap bulk reset, and a stable index per
//! pushed item: it grows in fixed-size blocks, so a pushed item keeps its address
//! for its lifetime even as the arena grows, which is what SIMD passes and stable
//! indices rely on. [`Slab`] is for slots freed and reused individually, with a
//! generational guard so a stale handle fails a generation check rather than
//! aliasing a reused slot.

/// A block-allocated arena. Items keep a stable index and a stable address until
/// [`Arena::reset`]. Indices are assigned densely in push order.
pub struct Arena<T> {
    blocks: Vec<Vec<T>>,
    block_size: usize,
    len: usize,
}

impl<T> Arena<T> {
    /// A new arena with the given block size (rounded up to at least 1).
    pub fn with_block_size(block_size: usize) -> Self {
        Arena {
            blocks: Vec::new(),
            block_size: block_size.max(1),
            len: 0,
        }
    }

    /// A new arena with a reasonable default block size.
    pub fn new() -> Self {
        Arena::with_block_size(1024)
    }

    /// Number of live items.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the arena holds no items.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push a value, returning its stable index.
    pub fn push(&mut self, value: T) -> u32 {
        let idx = self.len;
        let b = idx / self.block_size;
        if b >= self.blocks.len() {
            self.blocks.push(Vec::with_capacity(self.block_size));
        }
        self.blocks[b].push(value);
        self.len += 1;
        u32::try_from(idx).expect("arena index overflow")
    }

    /// Borrow an item by index. Panics if out of range.
    pub fn get(&self, idx: u32) -> &T {
        let i = idx as usize;
        assert!(i < self.len, "arena index {i} out of range {}", self.len);
        &self.blocks[i / self.block_size][i % self.block_size]
    }

    /// Mutably borrow an item by index. Panics if out of range.
    pub fn get_mut(&mut self, idx: u32) -> &mut T {
        let i = idx as usize;
        assert!(i < self.len, "arena index {i} out of range {}", self.len);
        &mut self.blocks[i / self.block_size][i % self.block_size]
    }

    /// Borrow an item if the index is in range.
    pub fn try_get(&self, idx: u32) -> Option<&T> {
        let i = idx as usize;
        if i < self.len {
            Some(&self.blocks[i / self.block_size][i % self.block_size])
        } else {
            None
        }
    }

    /// Free everything at once, keeping block capacity for reuse (end of a
    /// transient scope).
    pub fn reset(&mut self) {
        for b in &mut self.blocks {
            b.clear();
        }
        self.len = 0;
    }

    /// Iterate items in index order.
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.blocks.iter().flat_map(|b| b.iter())
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Arena::new()
    }
}

/// A handle into a [`Slab`], carrying the generation that was current when the
/// slot was allocated.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SlabHandle {
    index: u32,
    generation: u32,
}

enum Slot<T> {
    Occupied(T),
    Vacant(Option<u32>),
}

struct Entry<T> {
    generation: u32,
    slot: Slot<T>,
}

/// A slab with a free list and a generational guard. Removing a slot bumps its
/// generation, so a handle from before the removal fails its generation check and
/// reads as absent rather than aliasing whatever now occupies the slot.
pub struct Slab<T> {
    entries: Vec<Entry<T>>,
    free_head: Option<u32>,
    len: usize,
}

impl<T> Slab<T> {
    /// An empty slab.
    pub fn new() -> Self {
        Slab {
            entries: Vec::new(),
            free_head: None,
            len: 0,
        }
    }

    /// Number of occupied slots.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the slab holds no occupied slots.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert a value, reusing a free slot if one exists.
    pub fn insert(&mut self, value: T) -> SlabHandle {
        self.len += 1;
        match self.free_head.take() {
            Some(index) => {
                let entry = &mut self.entries[index as usize];
                let next_free = match entry.slot {
                    Slot::Vacant(next) => next,
                    Slot::Occupied(_) => unreachable!("free list pointed at an occupied slot"),
                };
                self.free_head = next_free;
                entry.slot = Slot::Occupied(value);
                SlabHandle {
                    index,
                    generation: entry.generation,
                }
            }
            None => {
                let index = u32::try_from(self.entries.len()).expect("slab index overflow");
                self.entries.push(Entry {
                    generation: 0,
                    slot: Slot::Occupied(value),
                });
                SlabHandle {
                    index,
                    generation: 0,
                }
            }
        }
    }

    /// Borrow a value if the handle is still valid (live slot and matching
    /// generation).
    pub fn get(&self, h: SlabHandle) -> Option<&T> {
        let entry = self.entries.get(h.index as usize)?;
        if entry.generation != h.generation {
            return None;
        }
        match &entry.slot {
            Slot::Occupied(v) => Some(v),
            Slot::Vacant(_) => None,
        }
    }

    /// Mutably borrow a value if the handle is still valid.
    pub fn get_mut(&mut self, h: SlabHandle) -> Option<&mut T> {
        let entry = self.entries.get_mut(h.index as usize)?;
        if entry.generation != h.generation {
            return None;
        }
        match &mut entry.slot {
            Slot::Occupied(v) => Some(v),
            Slot::Vacant(_) => None,
        }
    }

    /// Whether a handle is still valid.
    pub fn contains(&self, h: SlabHandle) -> bool {
        self.get(h).is_some()
    }

    /// Remove a value, bumping the slot's generation so older handles are invalid.
    /// Returns the value if the handle was valid.
    pub fn remove(&mut self, h: SlabHandle) -> Option<T> {
        let entry = self.entries.get_mut(h.index as usize)?;
        if entry.generation != h.generation {
            return None;
        }
        if let Slot::Vacant(_) = entry.slot {
            return None;
        }
        entry.generation = entry.generation.wrapping_add(1);
        let old = std::mem::replace(&mut entry.slot, Slot::Vacant(self.free_head));
        self.free_head = Some(h.index);
        self.len -= 1;
        match old {
            Slot::Occupied(v) => Some(v),
            Slot::Vacant(_) => None,
        }
    }
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Slab::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_stable_indices_across_growth() {
        let mut a: Arena<u32> = Arena::with_block_size(4);
        let mut handles = Vec::new();
        for v in 0..100u32 {
            handles.push(a.push(v));
        }
        assert_eq!(a.len(), 100);
        for (i, h) in handles.iter().enumerate() {
            assert_eq!(
                *a.get(*h),
                i as u32,
                "index stayed valid across many blocks"
            );
        }
        assert_eq!(handles[0], 0);
        assert_eq!(handles[99], 99);
    }

    #[test]
    fn arena_addresses_are_stable_across_growth() {
        let mut a: Arena<u64> = Arena::with_block_size(2);
        let h0 = a.push(10);
        let p0 = a.get(h0) as *const u64 as usize;
        for v in 1..1000u64 {
            a.push(v);
        }
        let p0_after = a.get(h0) as *const u64 as usize;
        assert_eq!(
            p0, p0_after,
            "an item's address does not move as the arena grows"
        );
    }

    #[test]
    fn arena_reset_clears_but_keeps_capacity() {
        let mut a: Arena<u32> = Arena::with_block_size(8);
        for v in 0..20 {
            a.push(v);
        }
        a.reset();
        assert!(a.is_empty());
        let h = a.push(7);
        assert_eq!(h, 0, "indices restart after reset");
        assert_eq!(*a.get(h), 7);
    }

    #[test]
    fn slab_generation_guard_rejects_stale_handle() {
        let mut s: Slab<&str> = Slab::new();
        let h = s.insert("first");
        assert_eq!(s.get(h), Some(&"first"));
        let removed = s.remove(h);
        assert_eq!(removed, Some("first"));
        assert_eq!(s.get(h), None, "stale handle reads as absent after removal");

        // The freed slot is reused, but the new handle has a fresh generation.
        let h2 = s.insert("second");
        assert_eq!(s.get(h2), Some(&"second"));
        assert_eq!(
            s.get(h),
            None,
            "the old handle never aliases the reused slot"
        );
        assert_ne!(h, h2);
    }

    #[test]
    fn slab_free_list_reuses_slots() {
        let mut s: Slab<u32> = Slab::new();
        let a = s.insert(1);
        let b = s.insert(2);
        let c = s.insert(3);
        assert_eq!(s.len(), 3);
        s.remove(b);
        assert_eq!(s.len(), 2);
        let d = s.insert(4); // should reuse b's slot
        assert_eq!(s.len(), 3);
        assert_eq!(s.get(d), Some(&4));
        assert_eq!(s.get(a), Some(&1));
        assert_eq!(s.get(c), Some(&3));
    }
}
