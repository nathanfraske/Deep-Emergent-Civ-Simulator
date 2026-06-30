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

//! The chunk grid and the level-of-detail quadtree (design Part 6).
//!
//! Part 6 makes the overworld glyph grid, the spatial-partition quadtree, and the
//! fidelity tiers one hierarchy rather than three. This module is that one tree over the
//! generated [`crate::TileMap`]: a [`QuadTree`] whose root covers the whole map and which
//! quarters down to per-tile leaves, with a [`NodeSummary`] (the dominant biome) at every
//! node. The camera reads a level of it to draw a zoomed view (see [`crate::view`]), and
//! the later in-world-significance scheduler will read the same tree to decide where to
//! spend fidelity, so it is built canonical from the start: every summary is computed in a
//! fixed walk with a lowest-id tiebreak, and [`QuadTree::state_hash`] folds the whole tree
//! in canonical order so the same map yields a bit-identical tree on any machine.
//!
//! A [`ChunkCoord`] names a `CHUNK`-by-`CHUNK` tile block, the Part 6 chunk, which is the
//! addressing the located-identity join (a tile to its occupants) will key on.

use crate::terrain::BiomeId;
use crate::topology::Coord3;
use crate::worldgen::TileMap;
use civsim_core::StateHasher;
use std::collections::BTreeMap;

/// Tiles per chunk side (the Part 6 chunk). A chunk is the natural block the located
/// identity index and the focus layer address; the quadtree itself quarters all the way to
/// single tiles, so a chunk is one mid-level node.
pub const CHUNK: i32 = 16;

/// A chunk coordinate: a `CHUNK`-by-`CHUNK` tile block at `(cx, cy)`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct ChunkCoord {
    pub cx: i32,
    pub cy: i32,
}

impl ChunkCoord {
    /// The chunk a ground coordinate falls in (floor division, so negative coordinates
    /// land in the correct chunk).
    #[inline]
    pub fn of(c: Coord3) -> ChunkCoord {
        ChunkCoord {
            cx: c.x.div_euclid(CHUNK),
            cy: c.y.div_euclid(CHUNK),
        }
    }

    /// The ground coordinate of this chunk's lower corner.
    #[inline]
    pub fn origin(self) -> Coord3 {
        Coord3::ground(self.cx * CHUNK, self.cy * CHUNK)
    }
}

/// The summary of a quadtree node: the dominant biome over the world tiles it covers, and
/// how many world tiles that is. A node covering no world tile (off the map) has no
/// summary.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeSummary {
    /// The most common biome among the node's world tiles (lowest id on a tie).
    pub dominant: BiomeId,
    /// The number of world tiles the node covers.
    pub tile_count: u32,
}

/// The level-of-detail quadtree over a [`TileMap`]: one tree whose root covers a square of
/// side `root_side` (a power of two at or above the larger map dimension) and which
/// quarters down through `depth` levels to per-tile leaves. Level 0 is the root; level
/// `depth` is per tile.
#[derive(Clone, Debug)]
pub struct QuadTree {
    root_side: i32,
    depth: u32,
    width: i32,
    height: i32,
    /// `levels[L]` is the row-major `2^L` by `2^L` grid of node summaries at level `L`; a
    /// node covering no world tile is `None`.
    levels: Vec<Vec<Option<NodeSummary>>>,
}

impl QuadTree {
    /// Build the quadtree over a generated map, summarising each node by the dominant biome
    /// of the world tiles it covers. The build reads only the tiles' biome ids (the
    /// [`crate::BiomeSet`] is needed to turn an id into a glyph, which is the view's job, not
    /// the tree's), and is a fixed canonical walk, so the same map yields a bit-identical
    /// tree.
    pub fn build(map: &TileMap) -> QuadTree {
        let topo = map.topo();
        let width = topo.width.max(0);
        let height = topo.height.max(0);
        let root_side = next_pow2(width.max(height).max(1));
        let depth = root_side.trailing_zeros();
        let mut levels: Vec<Vec<Option<NodeSummary>>> = Vec::with_capacity(depth as usize + 1);
        for level in 0..=depth {
            let nodes_per_side = 1i32 << level;
            let side = root_side >> level;
            let mut grid = Vec::with_capacity((nodes_per_side * nodes_per_side) as usize);
            for ny in 0..nodes_per_side {
                for nx in 0..nodes_per_side {
                    grid.push(summarize(map, nx * side, ny * side, side, width, height));
                }
            }
            levels.push(grid);
        }
        QuadTree {
            root_side,
            depth,
            width,
            height,
            levels,
        }
    }

    /// The number of quadtree levels below the root; level `depth` is per tile.
    #[inline]
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// The side of the root region in tiles (a power of two).
    #[inline]
    pub fn root_side(&self) -> i32 {
        self.root_side
    }

    /// The number of nodes per axis at a level (`2^level`).
    #[inline]
    pub fn nodes_per_side(&self, level: u32) -> i32 {
        1i32 << level.min(self.depth)
    }

    /// The tile side of a node at a level (`root_side >> level`).
    #[inline]
    pub fn node_side(&self, level: u32) -> i32 {
        self.root_side >> level.min(self.depth)
    }

    /// The map width in tiles.
    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// The map height in tiles.
    #[inline]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// The summary at a node, or `None` if the level or the node index is out of range or
    /// the node covers no world tile.
    pub fn node(&self, level: u32, nx: i32, ny: i32) -> Option<NodeSummary> {
        if level > self.depth {
            return None;
        }
        let per_side = 1i32 << level;
        if nx < 0 || ny < 0 || nx >= per_side || ny >= per_side {
            return None;
        }
        self.levels[level as usize][(ny * per_side + nx) as usize]
    }

    /// A deterministic 128-bit hash of the whole tree, walked level by level in row-major
    /// order, so the same map hashes identically on any machine.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        h.write_u32(self.root_side as u32);
        h.write_u32(self.depth);
        h.write_u32(self.width as u32);
        h.write_u32(self.height as u32);
        for level in &self.levels {
            for node in level {
                match node {
                    Some(s) => {
                        h.write_u32(1);
                        h.write_u32(s.dominant.0 as u32);
                        h.write_u32(s.tile_count);
                    }
                    None => h.write_u32(0),
                }
            }
        }
        h.finish()
    }
}

/// Summarise the world tiles in the square `[x0, x0+side) x [y0, y0+side)`, clipped to the
/// map, by their dominant biome (lowest id on a tie). Returns `None` if the square covers
/// no world tile.
fn summarize(
    map: &TileMap,
    x0: i32,
    y0: i32,
    side: i32,
    width: i32,
    height: i32,
) -> Option<NodeSummary> {
    if x0 >= width || y0 >= height {
        return None;
    }
    let x1 = (x0 + side).min(width);
    let y1 = (y0 + side).min(height);
    let mut counts: BTreeMap<BiomeId, u32> = BTreeMap::new();
    let mut total = 0u32;
    for y in y0..y1 {
        for x in x0..x1 {
            if let Some(t) = map.tile(Coord3::ground(x, y)) {
                *counts.entry(t.biome).or_insert(0) += 1;
                total += 1;
            }
        }
    }
    if total == 0 {
        return None;
    }
    // BTreeMap iterates by ascending BiomeId, so taking the first strict maximum gives the
    // lowest-id tiebreak, a fixed canonical choice.
    let mut best: Option<(BiomeId, u32)> = None;
    for (&id, &n) in &counts {
        match best {
            Some((_, bn)) if n <= bn => {}
            _ => best = Some((id, n)),
        }
    }
    let (dominant, _) = best.expect("a non-empty count map has a maximum");
    Some(NodeSummary {
        dominant,
        tile_count: total,
    })
}

/// The smallest power of two at or above `n` (for `n >= 1`).
fn next_pow2(n: i32) -> i32 {
    let mut s = 1i32;
    while s < n {
        s <<= 1;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::BiomeSet;
    use crate::topology::FlatBounded;
    use crate::worldgen::WorldgenParams;

    fn map(seed: u64, w: i32, h: i32) -> TileMap {
        TileMap::generate(
            seed,
            FlatBounded::new(w, h, 1),
            &BiomeSet::dev_default(),
            &WorldgenParams::dev_default(),
        )
    }

    #[test]
    fn chunk_of_floors_into_the_right_block() {
        assert_eq!(ChunkCoord::of(Coord3::ground(0, 0)), ChunkCoord { cx: 0, cy: 0 });
        assert_eq!(ChunkCoord::of(Coord3::ground(CHUNK - 1, 0)), ChunkCoord { cx: 0, cy: 0 });
        assert_eq!(ChunkCoord::of(Coord3::ground(CHUNK, 0)), ChunkCoord { cx: 1, cy: 0 });
        assert_eq!(ChunkCoord::of(Coord3::ground(-1, 0)), ChunkCoord { cx: -1, cy: 0 });
        assert_eq!(ChunkCoord { cx: 2, cy: 3 }.origin(), Coord3::ground(2 * CHUNK, 3 * CHUNK));
    }

    #[test]
    fn root_covers_the_map_and_depth_reaches_per_tile() {
        let t = QuadTree::build(&map(0xEA27, 48, 24));
        assert_eq!(t.root_side(), 64, "next power of two at or above 48");
        assert_eq!(t.depth(), 6, "64 = 2^6, so the deepest level is per tile");
        assert_eq!(t.node_side(t.depth()), 1, "a leaf is one tile");
        // The root summarises the whole map.
        let root = t.node(0, 0, 0).expect("the root covers world tiles");
        assert_eq!(root.tile_count, 48 * 24, "the root covers every world tile");
    }

    #[test]
    fn the_tree_is_bit_identical_for_the_same_seed() {
        let a = QuadTree::build(&map(0xEA27, 48, 24));
        let b = QuadTree::build(&map(0xEA27, 48, 24));
        assert_eq!(a.state_hash(), b.state_hash());
        let c = QuadTree::build(&map(0x1234, 48, 24));
        assert_ne!(a.state_hash(), c.state_hash(), "a different seed, a different tree");
    }

    #[test]
    fn a_leaf_summary_is_its_tile_biome() {
        let m = map(0xEA27, 48, 24);
        let t = QuadTree::build(&m);
        // At the deepest level a node is one tile, so its dominant is that tile's biome.
        for &(x, y) in &[(0, 0), (10, 5), (47, 23)] {
            let leaf = t.node(t.depth(), x, y).expect("an in-world leaf");
            assert_eq!(leaf.tile_count, 1);
            assert_eq!(leaf.dominant, m.tile(Coord3::ground(x, y)).unwrap().biome);
        }
    }

    #[test]
    fn off_world_nodes_have_no_summary() {
        // 48x24 in a 64x64 root: the node at (1,1) on level 1 covers [32,64)x[32,64), which
        // is partly off the 24-high world; the node covering only off-world tiles is None.
        let t = QuadTree::build(&map(0xEA27, 48, 24));
        // Level 1 node (0,1) covers y in [32,64): entirely below the 24-tall world.
        assert!(t.node(1, 0, 1).is_none(), "a fully off-world node has no summary");
        assert!(t.node(1, 0, 0).is_some(), "an in-world node is summarised");
    }

    #[test]
    fn a_node_dominant_matches_a_hand_count() {
        let m = map(0xEA27, 32, 32);
        let t = QuadTree::build(&m);
        // Level 1 node (0,0) covers [0,32)x... no: root_side for 32 is 32, depth 5, level 1
        // side 16, node (0,0) covers [0,16)x[0,16). Hand-count its dominant.
        let side = t.node_side(1);
        let mut counts: BTreeMap<BiomeId, u32> = BTreeMap::new();
        for y in 0..side {
            for x in 0..side {
                *counts.entry(m.tile(Coord3::ground(x, y)).unwrap().biome).or_insert(0) += 1;
            }
        }
        let hand = counts.iter().fold(None, |acc: Option<(BiomeId, u32)>, (&id, &n)| match acc {
            Some((_, bn)) if n <= bn => acc,
            _ => Some((id, n)),
        });
        assert_eq!(t.node(1, 0, 0).unwrap().dominant, hand.unwrap().0);
    }
}
