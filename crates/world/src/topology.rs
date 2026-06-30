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

//! The spatial coordinate and topology layer (design Part 56, Part 6), the first brick
//! of the generated map (roadmap M1).
//!
//! A world coordinate is a [`Coord3`] over the 2.5D stacked model: x and y on the plane,
//! z the stacked layer. All spatial math routes through the [`TopologySpace`] trait so a
//! flat, cylindrical, or spherical world stays selectable without changing the systems
//! above it; [`FlatBounded`] is the simplest concrete space and the only one the first
//! slice needs. Distance is the exact squared planar distance, an integer, so no square
//! root and no float enters canonical state (which keeps the R-GPU-CANON-PIN square-root
//! question off this path): it is a comparison key, not a rendered length.

/// A world coordinate over the 2.5D stacked model (Part 56): x and y on the plane, z the
/// stacked layer. An `i32` per axis (design Part 56) gives a large-scale world room to
/// grow (about plus or minus two billion tiles per axis) while staying a small, cheap
/// coordinate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Coord3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Coord3 {
    /// A coordinate at `(x, y, z)`.
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Coord3 { x, y, z }
    }

    /// A coordinate on the ground layer (z = 0).
    #[inline]
    pub const fn ground(x: i32, y: i32) -> Self {
        Coord3 { x, y, z: 0 }
    }

    /// A deterministic, well-distributed fold of this coordinate into a single value, so a
    /// cell can serve as a locus or region in a [`civsim_core::DrawKey`] (the canonical
    /// draw-keying schema). With `i32` axes a lossless 64-bit pack is impossible, so this
    /// mixes the three axes by FNV-1a instead: distinct cells fold to distinct keys with
    /// overwhelming probability, which is what a draw-key locus needs (it is a hash
    /// coordinate, not a stored length).
    #[inline]
    pub const fn key(self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        h = (h ^ (self.x as u32 as u64)).wrapping_mul(0x0000_0100_0000_01b3);
        h = (h ^ (self.y as u32 as u64)).wrapping_mul(0x0000_0100_0000_01b3);
        h = (h ^ (self.z as u32 as u64)).wrapping_mul(0x0000_0100_0000_01b3);
        h
    }
}

/// How space wraps at its edges. The membership is open: a cylindrical and a spherical
/// space are the next entries, added without changing any consumer because they route
/// through [`TopologySpace`]. [`FlatBounded`] is the simplest concrete space.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Topology {
    /// A finite, non-wrapping rectangle (see [`FlatBounded`]).
    FlatBounded,
}

/// The spatial behaviour every system above routes through, so the world's shape stays
/// selectable without changing them (Part 56).
pub trait TopologySpace {
    /// Whether a coordinate lies on the world.
    fn contains(&self, c: Coord3) -> bool;

    /// Bring a coordinate onto the world, wrapping it where the topology wraps and
    /// rejecting it where the topology is bounded. Returns `None` for an off-world
    /// coordinate that cannot be wrapped onto the world.
    fn normalize(&self, c: Coord3) -> Option<Coord3>;

    /// The in-world planar neighbours of a coordinate: the eight-neighbourhood on its
    /// own layer, walked in a fixed (row, then column) order so the result is canonical.
    fn neighbours(&self, c: Coord3) -> Vec<Coord3>;

    /// The exact squared planar distance between two coordinates. It is an integer, so
    /// no square root enters canonical state; it is a comparison key, not a length.
    fn distance2(&self, a: Coord3, b: Coord3) -> i64;
}

/// A finite, non-wrapping rectangular world of `width` by `height` tiles across `layers`
/// stacked planes. The simplest concrete [`TopologySpace`], and all the first map slice
/// needs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FlatBounded {
    pub width: i32,
    pub height: i32,
    pub layers: i32,
}

impl FlatBounded {
    /// A flat bounded world of `width` by `height` tiles over `layers` stacked planes.
    #[inline]
    pub const fn new(width: i32, height: i32, layers: i32) -> Self {
        FlatBounded {
            width,
            height,
            layers,
        }
    }
}

impl TopologySpace for FlatBounded {
    #[inline]
    fn contains(&self, c: Coord3) -> bool {
        c.x >= 0
            && c.x < self.width
            && c.y >= 0
            && c.y < self.height
            && c.z >= 0
            && c.z < self.layers
    }

    #[inline]
    fn normalize(&self, c: Coord3) -> Option<Coord3> {
        // Bounded: a coordinate is either on the world or off it; nothing wraps.
        if self.contains(c) {
            Some(c)
        } else {
            None
        }
    }

    fn neighbours(&self, c: Coord3) -> Vec<Coord3> {
        let mut out = Vec::with_capacity(8);
        // Row-then-column walk gives a fixed, canonical order (no hash, no thread order).
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                // Compute in i64 so an i32 edge cell does not overflow before the bounds test.
                let nx = c.x as i64 + dx;
                let ny = c.y as i64 + dy;
                if (0..self.width as i64).contains(&nx)
                    && (0..self.height as i64).contains(&ny)
                    && c.z >= 0
                    && c.z < self.layers
                {
                    out.push(Coord3::new(nx as i32, ny as i32, c.z));
                }
            }
        }
        out
    }

    #[inline]
    fn distance2(&self, a: Coord3, b: Coord3) -> i64 {
        // Compute in i128 so an i32-scale separation cannot overflow, then saturate to the
        // i64 comparison key (a far separation stays far); no float, no square root.
        let dx = a.x as i128 - b.x as i128;
        let dy = a.y as i128 - b.y as i128;
        (dx * dx + dy * dy).min(i64::MAX as i128) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_contains_and_normalize() {
        let w = FlatBounded::new(10, 8, 1);
        assert!(w.contains(Coord3::ground(0, 0)));
        assert!(w.contains(Coord3::ground(9, 7)));
        assert!(!w.contains(Coord3::ground(10, 0)));
        assert!(!w.contains(Coord3::ground(-1, 0)));
        assert!(!w.contains(Coord3::new(0, 0, 1)));
        assert_eq!(
            w.normalize(Coord3::ground(3, 3)),
            Some(Coord3::ground(3, 3))
        );
        assert_eq!(w.normalize(Coord3::ground(10, 0)), None, "bounded: no wrap");
    }

    #[test]
    fn neighbours_count_by_position() {
        let w = FlatBounded::new(10, 8, 1);
        assert_eq!(w.neighbours(Coord3::ground(5, 5)).len(), 8, "interior cell");
        assert_eq!(w.neighbours(Coord3::ground(0, 0)).len(), 3, "corner cell");
        assert_eq!(w.neighbours(Coord3::ground(5, 0)).len(), 5, "edge cell");
    }

    #[test]
    fn neighbours_are_all_in_world_and_canonically_ordered() {
        let w = FlatBounded::new(10, 8, 1);
        let n = w.neighbours(Coord3::ground(5, 5));
        assert!(n.iter().all(|&c| w.contains(c)), "neighbours stay in world");
        // The row-then-column walk is a fixed order, so two calls agree exactly.
        assert_eq!(n, w.neighbours(Coord3::ground(5, 5)));
        // The canonical order is row-major (y outer, x inner), pinned here so a
        // downstream canonical walk over neighbours is reproducible.
        assert_eq!(
            n,
            vec![
                Coord3::ground(4, 4),
                Coord3::ground(5, 4),
                Coord3::ground(6, 4),
                Coord3::ground(4, 5),
                Coord3::ground(6, 5),
                Coord3::ground(4, 6),
                Coord3::ground(5, 6),
                Coord3::ground(6, 6),
            ]
        );
    }

    #[test]
    fn edge_neighbours_do_not_overflow() {
        // An i32 max-edge cell must not overflow when its neighbour is computed (the i64
        // compute in neighbours is what guards this).
        let w = FlatBounded::new(i32::MAX, i32::MAX, 1);
        let c = Coord3::ground(i32::MAX - 1, i32::MAX - 1);
        let n = w.neighbours(c);
        assert!(n.iter().all(|&c| w.contains(c)));
    }

    #[test]
    fn distance2_is_exact_and_symmetric() {
        let w = FlatBounded::new(100, 100, 1);
        let a = Coord3::ground(1, 2);
        let b = Coord3::ground(4, 6);
        assert_eq!(w.distance2(a, b), 9 + 16, "3-4-5: 3^2 + 4^2");
        assert_eq!(w.distance2(a, b), w.distance2(b, a), "symmetric");
        assert_eq!(w.distance2(a, a), 0);
    }

    #[test]
    fn key_is_distinct_per_cell() {
        assert_ne!(Coord3::ground(1, 0).key(), Coord3::ground(0, 1).key());
        assert_ne!(Coord3::new(0, 0, 0).key(), Coord3::new(0, 0, 1).key());
        assert_eq!(Coord3::ground(7, 3).key(), Coord3::ground(7, 3).key());
    }
}
