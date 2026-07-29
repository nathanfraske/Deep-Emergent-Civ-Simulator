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

//! The typed canonical-state boundary (design Part 58, Part 3.4).
//!
//! Determinism is enforced at compile time rather than asked of contributors to
//! remember. The [`Canonical`] marker is implemented for the fixed-point type and
//! the integer types, and deliberately not for `f32` or `f64`. A container that
//! holds authoritative state bounds its element on `Canonical`, so a float in
//! canonical state is a compile error rather than a latent nondeterminism bug.
//!
//! Floating-point presentation data has no sanctioned crossing into canonical
//! state. A caller must supply an integer or exact fixed-point representation
//! whose custody and rounding contract were established before this boundary.

use crate::fixed::Fixed;

/// A type permitted in canonical (authoritative, replayable) state.
///
/// Implemented for [`Fixed`] and the integer and boolean primitives. It is
/// deliberately not implemented for `f32` or `f64`, so a generic over `Canonical`
/// cannot be instantiated with a floating-point type.
pub trait Canonical: Copy {}

impl Canonical for Fixed {}
impl Canonical for bool {}
impl Canonical for i8 {}
impl Canonical for i16 {}
impl Canonical for i32 {}
impl Canonical for i64 {}
impl Canonical for i128 {}
impl Canonical for u8 {}
impl Canonical for u16 {}
impl Canonical for u32 {}
impl Canonical for u64 {}
impl Canonical for u128 {}

/// A wrapper that marks its contents as non-authoritative. Whatever it holds can
/// never satisfy [`Canonical`], so it cannot be placed where canonical state is
/// required. Use it for render fields and view-time elaboration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NonCanonical<T>(pub T);

impl<T> NonCanonical<T> {
    /// Wrap a non-authoritative value.
    pub const fn new(value: T) -> Self {
        NonCanonical(value)
    }
}

/// A cell that can only hold canonical state. The bound is the compile-time
/// boundary: `CanonicalCell::<f64>::new(..)` does not type-check, because `f64`
/// does not implement [`Canonical`].
#[derive(Clone, Copy, Debug, Default)]
pub struct CanonicalCell<T: Canonical>(T);

impl<T: Canonical> CanonicalCell<T> {
    /// Wrap a canonical value.
    pub const fn new(value: T) -> Self {
        CanonicalCell(value)
    }

    /// Read the canonical value.
    pub fn get(self) -> T {
        self.0
    }
}

// --- Canonical iteration and reduction (design Part 3.5, Part 57; R-CANON-WALK, R-REDUCE-ORDER) ---
//
// Determinism over a collection has one requirement: the walk order is a function of the data, not
// of insertion, hashing, or thread schedule. Canonical containers are ordered maps or carry a
// sorted accessor (the `Registry::entries_sorted` model), so their own walk is already canonical;
// these two helpers are the sanctioned path for the harder cases the red-team named, a walk over an
// unordered source that will be hashed or saved, and a non-associative combine whose result would
// otherwise depend on arrival order. The key must be a total order (an id or a content key, unique
// per element); with a total key the result is a pure function of the item set, independent of the
// order the items arrived in.

/// Materialise items in canonical key order: the single sanctioned way to turn an unordered source
/// into an ordered sequence for a hash, a save, a selection, or any order-sensitive walk
/// (R-CANON-WALK, design Part 3.5). The key must be a total order, unique per element; with a total
/// key the output is a pure function of the item set. The sort is stable, so a non-total key
/// degrades to input order on ties rather than to hash order, but a caller relying on
/// order-independence must supply a total key.
#[inline]
pub fn canonical_sorted<T, K, F>(items: impl IntoIterator<Item = T>, key: F) -> Vec<T>
where
    K: Ord,
    F: Fn(&T) -> K,
{
    let mut v: Vec<T> = items.into_iter().collect();
    // sort_by_cached_key computes each key once (rather than twice per comparison) and is stable.
    v.sort_by_cached_key(|x| key(x));
    v
}

/// Fold a non-associative canonical combine in a pinned order (R-REDUCE-ORDER, design Part 57):
/// sort the items by their total key, then fold left. The result is a pure function of the item set
/// rather than of arrival or thread order, which pins weighted selection over an unordered candidate
/// list and any other non-associative reduction. The
/// key must be a total order for the guarantee to hold. This is the general form of what
/// [`Fixed::sum_bits`](crate::Fixed::sum_bits) does for the associative sum: order the inputs, then
/// combine.
#[inline]
pub fn canonical_reduce<T, K, A, KF, FF>(
    items: impl IntoIterator<Item = T>,
    key: KF,
    init: A,
    fold: FF,
) -> A
where
    K: Ord,
    KF: Fn(&T) -> K,
    FF: Fn(A, T) -> A,
{
    canonical_sorted(items, key).into_iter().fold(init, fold)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A generic that only accepts canonical state. Calling it with f64 would not
    // compile, which is the boundary in action.
    fn store_canonical<T: Canonical>(v: T) -> CanonicalCell<T> {
        CanonicalCell::new(v)
    }

    #[test]
    fn canonical_types_are_accepted() {
        assert_eq!(store_canonical(Fixed::ONE).get(), Fixed::ONE);
        assert_eq!(store_canonical(7i64).get(), 7);
        assert!(store_canonical(true).get());
    }

    #[test]
    fn canonical_sorted_is_input_order_independent() {
        // Items keyed by a total (unique) id. Two different arrival orders sort identically.
        let a = vec![(3u32, "c"), (1, "a"), (2, "b")];
        let b = vec![(1u32, "a"), (2, "b"), (3, "c")];
        let sa = canonical_sorted(a, |&(k, _)| k);
        let sb = canonical_sorted(b, |&(k, _)| k);
        assert_eq!(
            sa, sb,
            "the same set materialises in the same canonical order"
        );
        assert_eq!(sa, vec![(1, "a"), (2, "b"), (3, "c")]);
    }

    #[test]
    fn canonical_reduce_pins_a_non_associative_combine() {
        // A deliberately non-commutative fold (acc*3 + value): the result depends on the order the
        // values are combined in. canonical_reduce sorts by the total key first, so it yields the
        // same result whatever order the inputs arrived in, and that result is the one the sorted
        // order gives, not the arrival order.
        let shuffled = vec![(3u32, 7i64), (1, 2), (2, 5)];
        let other_order = vec![(2u32, 5i64), (3, 7), (1, 2)];
        let fold = |acc: i64, (_, v): (u32, i64)| acc * 3 + v;

        let r1 = canonical_reduce(shuffled.clone(), |&(k, _)| k, 0i64, fold);
        let r2 = canonical_reduce(other_order, |&(k, _)| k, 0i64, fold);
        assert_eq!(
            r1, r2,
            "the combine is a function of the set, not the arrival order"
        );

        // It equals the fold over the key-sorted sequence [2, 5, 7]: ((0*3+2)*3+5)*3+7 = 40.
        assert_eq!(r1, 40);
        // And it differs from a naive fold over the shuffled arrival order [7, 2, 5]: 74, which is
        // exactly the nondeterminism the helper removes.
        let naive = shuffled.into_iter().fold(0i64, fold);
        assert_ne!(
            naive, r1,
            "a naive fold over arrival order would differ (the bug being pinned)"
        );
        assert_eq!(naive, 74);
    }

    #[test]
    fn canonical_reduce_matches_a_manual_sorted_fold() {
        // Belt-and-braces: canonical_reduce is exactly sort-by-key-then-fold.
        let items = vec![(5u32, 1i64), (1, 2), (9, 3), (4, 4)];
        let via_helper = canonical_reduce(items.clone(), |&(k, _)| k, 100i64, |a, (_, v)| a - v);
        let mut sorted = items;
        sorted.sort_by_key(|&(k, _)| k);
        let manual = sorted.into_iter().fold(100i64, |a, (_, v)| a - v);
        assert_eq!(via_helper, manual);
    }
}
