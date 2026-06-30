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

//! End-to-end proof of the zoomable map over the public API: a seed becomes a generated
//! world, the canonical quadtree summarises it, and the camera draws it at every zoom, all
//! bit-identical on replay. This is the worked proof that the large-scale-to-fine view
//! reads canon and never perturbs it (design Parts 6, 14, 54).

use civsim_world::view::{whole_map_frame, Camera};
use civsim_world::{BiomeSet, Coord3, FlatBounded, QuadTree, TileMap, WorldgenParams};

fn world(seed: u64, w: i32, h: i32) -> (TileMap, QuadTree, BiomeSet) {
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(
        seed,
        FlatBounded::new(w, h, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );
    let tree = QuadTree::build(&map);
    (map, tree, biomes)
}

#[test]
fn the_same_seed_reproduces_the_whole_pipeline() {
    let (map_a, tree_a, _b) = world(0xC117, 96, 64);
    let (map_b, tree_b, _b2) = world(0xC117, 96, 64);
    assert_eq!(map_a.state_hash(), map_b.state_hash(), "the map is deterministic");
    assert_eq!(tree_a.state_hash(), tree_b.state_hash(), "the tree is deterministic");
}

#[test]
fn a_different_seed_is_a_different_world_and_tree() {
    let (map_a, tree_a, _b) = world(0xC117, 96, 64);
    let (map_c, tree_c, _b2) = world(0xD00D, 96, 64);
    assert_ne!(map_a.state_hash(), map_c.state_hash());
    assert_ne!(tree_a.state_hash(), tree_c.state_hash());
}

#[test]
fn the_zoom_ladder_runs_from_one_glyph_to_per_tile() {
    let (_m, tree, biomes) = world(0xC117, 96, 64);
    // Zoom 0 is a single glyph: the whole world's dominant biome.
    let top = whole_map_frame(&tree, &biomes, 0);
    assert_eq!(top.lines().count(), 1);
    assert_eq!(top.trim_end().chars().count(), 1);
    // Each rung down never shrinks, and the deepest rung is per tile.
    let mut prev = 0usize;
    for z in 0..=tree.depth() {
        let rows = whole_map_frame(&tree, &biomes, z).lines().count();
        assert!(rows >= prev, "zooming in never shrinks the overview");
        prev = rows;
    }
    let deepest = whole_map_frame(&tree, &biomes, tree.depth());
    assert_eq!(deepest.lines().count(), 64, "the deepest overview has a row per tile row");
}

#[test]
fn the_camera_pans_and_replays_bit_identically() {
    let (_m, tree, biomes) = world(0xC117, 96, 64);
    let cam = Camera::new(Coord3::ground(48, 32), 4);
    let frame = cam.frame(&tree, &biomes, 24, 12);
    assert_eq!(frame.lines().count(), 12);
    assert!(frame.lines().all(|l| l.chars().count() == 24));
    assert_eq!(frame, cam.frame(&tree, &biomes, 24, 12), "a view is a pure read");
}
