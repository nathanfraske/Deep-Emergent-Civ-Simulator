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

//! The 64-byte alignment wrapper that prevents false sharing (design Part 2.5).
//!
//! Per-thread accumulators, per-chunk dirty flags, and any other write-hot
//! per-worker scalar are wrapped this way so two compute domains never contend on
//! one cache line.

/// Pads and aligns its contents to a 64-byte cache line.
#[repr(align(64))]
#[derive(Clone, Copy, Default, Debug)]
pub struct CacheLine<T>(pub T);

impl<T> CacheLine<T> {
    /// Wrap a value.
    pub const fn new(value: T) -> Self {
        CacheLine(value)
    }

    /// Unwrap to the contained value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> core::ops::Deref for CacheLine<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> core::ops::DerefMut for CacheLine<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_is_a_cache_line() {
        assert_eq!(core::mem::align_of::<CacheLine<u8>>(), 64);
        assert_eq!(core::mem::align_of::<CacheLine<u64>>(), 64);
        // Two adjacent wrapped scalars sit on separate lines.
        assert!(core::mem::size_of::<[CacheLine<u64>; 2]>() >= 128);
    }

    #[test]
    fn deref_round_trips() {
        let mut c = CacheLine::new(41u32);
        *c += 1;
        assert_eq!(*c, 42);
        assert_eq!(c.into_inner(), 42);
    }
}
