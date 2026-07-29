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

//! The camera and the multi-zoom glyph view (design Part 14, Part 11, Part 54).
//!
//! The camera reads a level of the canonical [`QuadTree`] and draws it. Two reads are
//! offered: [`whole_map_frame`] renders the whole world at a chosen zoom (one glyph at zoom
//! 0, the per-tile grid at the deepest zoom), the overview ladder; [`Camera::frame`] renders
//! a fixed viewport centred on a world coordinate at the camera's zoom, the pan-and-zoom
//! read. Both are pure functions of the tree and the biome glyphs, so the view is a read of
//! canon and never writes it (Principle 10, observer independence, design Part 54): two
//! cameras at different zooms over the same world draw consistent pictures and neither
//! perturbs the simulation. Each read comes in three forms: a plain glyph frame, a
//! truecolor glyph frame for a terminal that supports 24-bit ANSI ([`whole_map_frame_color`],
//! [`Camera::frame_color`]), and an RGB pixel buffer for a window ([`Camera::paint`]). The
//! colour is presentation data on the biome (it never enters canonical state), so painting
//! the world in colour leaves determinism untouched. The GPU multi-scale path (Part 14) is a
//! later swap of these same reads.

use crate::lod::QuadTree;
use crate::terrain::{BiomeSet, Rgb};
use crate::topology::Coord3;

/// The glyph for a quadtree node: the dominant biome's glyph, or a space for a node off the
/// world.
#[inline]
fn node_glyph(tree: &QuadTree, biomes: &BiomeSet, level: u32, nx: i32, ny: i32) -> char {
    match tree.node(level, nx, ny) {
        Some(s) => biomes.glyph(s.dominant),
        None => ' ',
    }
}

/// The colour of a quadtree node: the dominant biome's colour, or `None` for a node off the
/// world.
#[inline]
fn node_color(tree: &QuadTree, biomes: &BiomeSet, level: u32, nx: i32, ny: i32) -> Option<Rgb> {
    tree.node(level, nx, ny).map(|s| biomes.color(s.dominant))
}

/// Append one truecolor cell to `s`: the biome colour as the background with a
/// luminance-chosen foreground glyph, then a reset. An off-world cell (`None`) is a plain
/// space, so panning past the edge stays blank.
fn push_ansi_cell(s: &mut String, color: Option<Rgb>, glyph: char) {
    use std::fmt::Write as _;
    match color {
        Some(c) => {
            let fg = if c.luminance() > 140 {
                (0, 0, 0)
            } else {
                (235, 235, 235)
            };
            // \x1b[48;2;r;g;bm sets the background, \x1b[38;2;r;g;bm the foreground.
            let _ = write!(
                s,
                "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{}\x1b[0m",
                c.r, c.g, c.b, fg.0, fg.1, fg.2, glyph
            );
        }
        None => s.push(' '),
    }
}

/// Render the whole world at a zoom level: the in-world node grid at that level, one node
/// glyph per cell, one row per line. Zoom 0 is the single root node (the whole-world
/// dominant biome); the deepest zoom (`tree.depth()`) is the per-tile map. A zoom past the
/// deepest level is clamped, so the ladder saturates at per-tile rather than erroring.
pub fn whole_map_frame(tree: &QuadTree, biomes: &BiomeSet, zoom: u32) -> String {
    let level = zoom.min(tree.depth());
    let side = tree.node_side(level);
    // The in-world node grid: as many nodes as it takes to cover the map at this level.
    let cols = div_ceil(tree.width(), side);
    let rows = div_ceil(tree.height(), side);
    let mut s = String::with_capacity(((cols + 1) * rows).max(0) as usize);
    for ny in 0..rows {
        for nx in 0..cols {
            s.push(node_glyph(tree, biomes, level, nx, ny));
        }
        s.push('\n');
    }
    s
}

/// The truecolor twin of [`whole_map_frame`]: the same overview at a zoom level, each cell
/// painted with its biome colour (background) and glyph (foreground) using 24-bit ANSI
/// escapes, one row per line. Renders in colour in a terminal that supports truecolor
/// (Windows Terminal, most modern emulators).
pub fn whole_map_frame_color(tree: &QuadTree, biomes: &BiomeSet, zoom: u32) -> String {
    let level = zoom.min(tree.depth());
    let side = tree.node_side(level);
    let cols = div_ceil(tree.width(), side);
    let rows = div_ceil(tree.height(), side);
    let mut s = String::with_capacity(((cols * 24 + 1) * rows).max(0) as usize);
    for ny in 0..rows {
        for nx in 0..cols {
            push_ansi_cell(
                &mut s,
                node_color(tree, biomes, level, nx, ny),
                node_glyph(tree, biomes, level, nx, ny),
            );
        }
        s.push('\n');
    }
    s
}

/// A view onto the world: where it is centred and how far it is zoomed in. The zoom is a
/// quadtree level (0 the whole-world root, higher is finer, clamped to the tree depth).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Camera {
    /// The world coordinate the viewport is centred on.
    pub center: Coord3,
    /// The quadtree level to read (0 coarsest; clamped to the tree depth).
    pub zoom: u32,
}

impl Camera {
    /// A camera centred on a coordinate at a zoom level.
    #[inline]
    pub fn new(center: Coord3, zoom: u32) -> Camera {
        Camera { center, zoom }
    }

    /// The quadtree level this camera reads, clamped to the tree's depth.
    #[inline]
    pub fn level(&self, tree: &QuadTree) -> u32 {
        self.zoom.min(tree.depth())
    }

    /// Render a `cols` by `rows` viewport centred on the camera, reading the camera's zoom
    /// level of the tree. Nodes outside the world draw as spaces, so panning past the edge
    /// shows the world ending rather than wrapping or erroring. The frame is `rows` lines of
    /// `cols` glyphs each.
    pub fn frame(&self, tree: &QuadTree, biomes: &BiomeSet, cols: i32, rows: i32) -> String {
        let cols = cols.max(0);
        let rows = rows.max(0);
        let level = self.level(tree);
        let side = tree.node_side(level);
        // The node the camera centre falls in (floor division, so a centre at a negative or
        // off-world coordinate still maps to a well-defined node index).
        let cnx = self.center.x.div_euclid(side);
        let cny = self.center.y.div_euclid(side);
        let ox = cnx - cols / 2;
        let oy = cny - rows / 2;
        let mut s = String::with_capacity(((cols + 1) * rows) as usize);
        for r in 0..rows {
            for c in 0..cols {
                s.push(node_glyph(tree, biomes, level, ox + c, oy + r));
            }
            s.push('\n');
        }
        s
    }

    /// The truecolor twin of [`Camera::frame`]: a `cols` by `rows` viewport centred on the
    /// camera with each cell painted in its biome colour by 24-bit ANSI escapes.
    pub fn frame_color(&self, tree: &QuadTree, biomes: &BiomeSet, cols: i32, rows: i32) -> String {
        let cols = cols.max(0);
        let rows = rows.max(0);
        let level = self.level(tree);
        let side = tree.node_side(level);
        let cnx = self.center.x.div_euclid(side);
        let cny = self.center.y.div_euclid(side);
        let ox = cnx - cols / 2;
        let oy = cny - rows / 2;
        let mut s = String::with_capacity(((cols * 24 + 1) * rows) as usize);
        for r in 0..rows {
            for c in 0..cols {
                push_ansi_cell(
                    &mut s,
                    node_color(tree, biomes, level, ox + c, oy + r),
                    node_glyph(tree, biomes, level, ox + c, oy + r),
                );
            }
            s.push('\n');
        }
        s
    }

    /// Paint the camera's view into a `px_w` by `px_h` RGB framebuffer (row-major
    /// `0x00RRGGBB` words, the layout a window blits directly), each quadtree node drawn as a
    /// `cell`-pixel square in its biome colour and any off-world pixel set to `bg`. The
    /// camera centre sits at the middle of the buffer. This is a pure read of the tree, so
    /// the window it feeds shows the world without ever writing canon (Principle 10).
    pub fn paint(
        &self,
        tree: &QuadTree,
        biomes: &BiomeSet,
        px_w: usize,
        px_h: usize,
        cell: usize,
        bg: Rgb,
    ) -> Vec<u32> {
        let cell = cell.max(1);
        let level = self.level(tree);
        let side = tree.node_side(level);
        // The viewport in nodes, and the top-left node so the centre lands mid-buffer.
        let cols = (px_w / cell).max(1) as i32;
        let rows = (px_h / cell).max(1) as i32;
        let cnx = self.center.x.div_euclid(side);
        let cny = self.center.y.div_euclid(side);
        let ox = cnx - cols / 2;
        let oy = cny - rows / 2;
        let bg = bg.pack();
        let mut buf = vec![bg; px_w * px_h];
        for py in 0..px_h {
            let ny = oy + (py / cell) as i32;
            let row = py * px_w;
            for px in 0..px_w {
                let nx = ox + (px / cell) as i32;
                buf[row + px] = node_color(tree, biomes, level, nx, ny)
                    .map(Rgb::pack)
                    .unwrap_or(bg);
            }
        }
        buf
    }
}

/// Ceiling division for non-negative `n` by positive `d`.
#[inline]
fn div_ceil(n: i32, d: i32) -> i32 {
    if n <= 0 {
        0
    } else {
        (n + d - 1) / d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lod::QuadTree;
    use crate::terrain::BiomeSet;
    use crate::topology::FlatBounded;
    use crate::worldgen::{TileMap, WorldgenParams};

    fn tree(seed: u64, w: i32, h: i32) -> (QuadTree, TileMap, BiomeSet) {
        let biomes = BiomeSet::dev_default();
        let map = TileMap::generate(
            seed,
            FlatBounded::new(w, h, 1),
            &biomes,
            &WorldgenParams::dev_default(),
        );
        let t = QuadTree::build(&map);
        (t, map, biomes)
    }

    #[test]
    fn zoom_zero_is_a_single_glyph() {
        let (t, _m, b) = tree(0xEA27, 48, 24);
        let frame = whole_map_frame(&t, &b, 0);
        assert_eq!(frame, format!("{}\n", node_glyph(&t, &b, 0, 0, 0)));
        assert_eq!(frame.lines().count(), 1, "one row");
        assert_eq!(frame.trim_end().chars().count(), 1, "one glyph");
    }

    #[test]
    fn the_overview_grows_as_it_zooms_in() {
        let (t, _m, b) = tree(0xEA27, 48, 24);
        let mut prev = 0usize;
        for z in 0..=t.depth() {
            let lines = whole_map_frame(&t, &b, z).lines().count();
            assert!(
                lines >= prev,
                "the overview rows do not shrink as zoom rises"
            );
            prev = lines;
        }
    }

    #[test]
    fn the_deepest_zoom_equals_the_per_tile_render() {
        let (t, m, b) = tree(0xEA27, 48, 24);
        // At the deepest level a node is one tile, so the overview is the per-tile map; the
        // map is 48 wide and the root 64, so the deepest overview is 64 wide (the off-world
        // columns draw as spaces), while render_glyphs is exactly the world. Compare the
        // world sub-rectangle.
        let frame = whole_map_frame(&t, &b, t.depth());
        let render = m.render_glyphs(&b);
        let frame_rows: Vec<&str> = frame.lines().collect();
        let render_rows: Vec<&str> = render.lines().collect();
        assert_eq!(
            frame_rows.len(),
            24,
            "the deepest overview has a row per tile row"
        );
        for (fr, rr) in frame_rows.iter().zip(render_rows.iter()) {
            let f: String = fr.chars().take(48).collect();
            assert_eq!(f, *rr, "the deepest overview matches the per-tile render");
        }
    }

    #[test]
    fn a_frame_has_the_requested_size() {
        let (t, _m, b) = tree(0xEA27, 48, 24);
        let cam = Camera::new(Coord3::ground(24, 12), 3);
        let frame = cam.frame(&t, &b, 20, 10);
        let rows: Vec<&str> = frame.lines().collect();
        assert_eq!(rows.len(), 10, "row count is the requested height");
        assert!(
            rows.iter().all(|r| r.chars().count() == 20),
            "each row is the requested width"
        );
    }

    #[test]
    fn the_view_is_a_pure_read_and_replays_identically() {
        let (t, _m, b) = tree(0xEA27, 48, 24);
        let cam = Camera::new(Coord3::ground(24, 12), 4);
        assert_eq!(cam.frame(&t, &b, 16, 8), cam.frame(&t, &b, 16, 8));
        for z in 0..=t.depth() {
            assert_eq!(whole_map_frame(&t, &b, z), whole_map_frame(&t, &b, z));
        }
    }

    #[test]
    fn panning_by_a_node_shifts_the_window_by_one() {
        let (t, _m, b) = tree(0xEA27, 64, 64);
        let level = 4u32;
        let side = t.node_side(level);
        let a = Camera::new(Coord3::ground(32, 32), level);
        // Move the centre right by exactly one node at this level.
        let bcam = Camera::new(Coord3::ground(32 + side, 32), level);
        let frame_a = a.frame(&t, &b, 9, 5);
        let frame_b = bcam.frame(&t, &b, 9, 5);
        // Column c+1 of frame A equals column c of frame B (the window slid one node right).
        let ra: Vec<Vec<char>> = frame_a.lines().map(|l| l.chars().collect()).collect();
        let rb: Vec<Vec<char>> = frame_b.lines().map(|l| l.chars().collect()).collect();
        for r in 0..ra.len() {
            for c in 0..8 {
                assert_eq!(ra[r][c + 1], rb[r][c], "the window slid exactly one node");
            }
        }
    }

    #[test]
    fn panning_off_the_edge_draws_space_not_a_panic() {
        let (t, _m, b) = tree(0xEA27, 48, 24);
        // Centre far off the world; the viewport should be all spaces, no panic.
        let cam = Camera::new(Coord3::ground(-1000, -1000), 5);
        let frame = cam.frame(&t, &b, 8, 4);
        assert!(
            frame.chars().all(|c| c == ' ' || c == '\n'),
            "off-world draws as space"
        );
    }

    #[test]
    fn the_colour_overview_carries_ansi_and_replays() {
        let (t, _m, b) = tree(0xEA27, 48, 24);
        let frame = whole_map_frame_color(&t, &b, 4);
        assert!(
            frame.contains("\x1b[48;2;"),
            "cells set a truecolor background"
        );
        assert!(frame.contains("\x1b[0m"), "cells reset");
        assert_eq!(
            frame,
            whole_map_frame_color(&t, &b, 4),
            "a colour view is a pure read"
        );
        // The same number of rows as the plain overview.
        assert_eq!(
            frame.lines().count(),
            whole_map_frame(&t, &b, 4).lines().count()
        );
    }

    #[test]
    fn the_pixel_buffer_has_the_right_size_and_replays() {
        let (t, _m, b) = tree(0xEA27, 96, 64);
        let cam = Camera::new(Coord3::ground(48, 32), 5);
        let bg = crate::terrain::Rgb::new(8, 8, 12);
        let buf = cam.paint(&t, &b, 320, 200, 4, bg);
        assert_eq!(buf.len(), 320 * 200, "one word per pixel");
        assert_eq!(
            buf,
            cam.paint(&t, &b, 320, 200, 4, bg),
            "painting is a pure read"
        );
    }

    #[test]
    fn off_world_pixels_take_the_background() {
        let (t, _m, b) = tree(0xEA27, 48, 24);
        let bg = crate::terrain::Rgb::new(8, 8, 12);
        // Centre far off the world: every pixel is the background colour.
        let cam = Camera::new(Coord3::ground(-100000, -100000), 5);
        let buf = cam.paint(&t, &b, 64, 64, 4, bg);
        assert!(
            buf.iter().all(|&w| w == bg.pack()),
            "off-world is all background"
        );
    }

    #[test]
    fn a_painted_cell_matches_its_node_colour() {
        let (t, _m, b) = tree(0xEA27, 96, 64);
        let cam = Camera::new(Coord3::ground(48, 32), 5);
        let bg = crate::terrain::Rgb::new(8, 8, 12);
        let (w, h, cell) = (320usize, 200usize, 4usize);
        let buf = cam.paint(&t, &b, w, h, cell, bg);
        // The centre pixel's colour must be the colour of the node under the camera centre.
        let level = cam.level(&t);
        let side = t.node_side(level);
        let want = node_color(
            &t,
            &b,
            level,
            cam.center.x.div_euclid(side),
            cam.center.y.div_euclid(side),
        )
        .map(crate::terrain::Rgb::pack)
        .unwrap_or(bg.pack());
        // The centre node sits at viewport centre (cols/2, rows/2), i.e. mid-buffer.
        let cx = (w / cell / 2) * cell;
        let cy = (h / cell / 2) * cell;
        assert_eq!(
            buf[cy * w + cx],
            want,
            "the centre pixel is the centre node colour"
        );
    }
}
