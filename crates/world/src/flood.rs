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

//! Priority-flood depression filling and sill routing, a Layer-0 determinism primitive for the
//! genesis-forward geology solvers (hydrology, drainage networks, and lake basins over a terrain grid).
//!
//! Priority-flood (Barnes, Lehman, and Mulla 2014) raises every cell to the lowest elevation at which it
//! drains to the grid boundary, so no interior sink remains, and records each cell's downstream receiver
//! (the neighbour it drains toward). It processes cells in increasing elevation order from the boundary
//! inward, so an interior basin is filled exactly to its sill (the lowest pass out of it) and no higher.
//!
//! The determinism is exact (Principle 3). Elevations are integers, and the processing order is a total
//! order on `(elevation, cell index)`: every cell enters the frontier once with a UNIQUE key, so there are
//! no ties to resolve non-deterministically, and the filled grid and the receiver map are pure functions
//! of the input elevation grid, independent of how the frontier is stored or how the work is split. A
//! parallel priority-flood and this serial one produce bit-identical results. This is what lets a drainage
//! network and a lake basin carry a state-hashable identity without a canonical-walk hazard.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// The result of a priority-flood: the depression-filled elevations and the downstream receiver of each
/// cell (the neighbour it drains toward). A boundary cell drains off-grid, so it receives itself; every
/// interior cell receives the neighbour that first reached it from the boundary side, so following the
/// receivers from any cell reaches the boundary in finitely many steps (there is no interior sink and no
/// cycle).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrainedGrid {
    /// The depression-filled elevation of each cell (at least its original elevation).
    pub filled: Vec<i64>,
    /// The downstream neighbour of each cell (itself for a boundary outlet).
    pub receiver: Vec<usize>,
}

/// The 4-connected orthogonal neighbours of `(x, y)` on a `width` by `height` grid, as flat indices.
fn neighbors4(x: usize, y: usize, width: usize, height: usize) -> impl Iterator<Item = usize> {
    let mut out = Vec::with_capacity(4);
    if x > 0 {
        out.push(y * width + (x - 1));
    }
    if x + 1 < width {
        out.push(y * width + (x + 1));
    }
    if y > 0 {
        out.push((y - 1) * width + x);
    }
    if y + 1 < height {
        out.push((y + 1) * width + x);
    }
    out.into_iter()
}

/// Priority-flood a `width` by `height` integer elevation grid: fill every interior depression to its sill
/// and record the downstream receiver of each cell. Deterministic and worker-invariant (the total order on
/// `(elevation, index)` has no ties). Panics if `elevation.len()` is not `width * height`; a zero-area grid
/// returns an empty result.
pub fn priority_flood(width: usize, height: usize, elevation: &[i64]) -> DrainedGrid {
    let n = width * height;
    assert_eq!(
        elevation.len(),
        n,
        "elevation grid length {} must equal width*height {}",
        elevation.len(),
        n
    );
    let mut filled = elevation.to_vec();
    let mut visited = vec![false; n];
    let mut receiver = vec![0usize; n];
    // A min-heap by (elevation, index): Reverse turns the max-heap into a min-heap, and the index in the
    // key makes every entry unique, so the pop order is a total order with no non-deterministic ties.
    let mut frontier: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();

    // Seed the frontier with the boundary cells: each drains off-grid, so it receives itself.
    for y in 0..height {
        for x in 0..width {
            if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                let i = y * width + x;
                visited[i] = true;
                receiver[i] = i;
                frontier.push(Reverse((filled[i], i)));
            }
        }
    }

    while let Some(Reverse((e, c))) = frontier.pop() {
        let (cx, cy) = (c % width, c / width);
        for nb in neighbors4(cx, cy, width, height) {
            if !visited[nb] {
                visited[nb] = true;
                // Raise the neighbour to at least the sill level e: a cell in a basin is lifted to the
                // lowest pass out of it, a cell already above the sill keeps its own elevation.
                let new_e = filled[nb].max(e);
                filled[nb] = new_e;
                receiver[nb] = c;
                frontier.push(Reverse((new_e, nb)));
            }
        }
    }

    DrainedGrid { filled, receiver }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Follow the receivers from `start` and return whether the walk reaches a boundary outlet (a cell
    /// that receives itself) without cycling, the drainage-integrity property.
    fn drains_to_boundary(d: &DrainedGrid, start: usize, n: usize) -> bool {
        let mut c = start;
        for _ in 0..=n {
            if d.receiver[c] == c {
                return true; // a boundary outlet
            }
            c = d.receiver[c];
        }
        false // walked more than n steps: a cycle
    }

    #[test]
    fn a_single_pit_fills_to_its_sill() {
        // A 3x3 grid, flat rim at 5, a pit of 0 in the centre. The centre fills to the rim (5), the lowest
        // level at which it drains out; the rim is unchanged.
        let elev = vec![5, 5, 5, 5, 0, 5, 5, 5, 5];
        let d = priority_flood(3, 3, &elev);
        assert_eq!(d.filled, vec![5, 5, 5, 5, 5, 5, 5, 5, 5]);
        // The centre drains to a rim cell, which drains off-grid.
        assert!(drains_to_boundary(&d, 4, 9));
    }

    #[test]
    fn a_basin_fills_only_to_the_lowest_pass() {
        // A 4x4 basin: a high rim of 9 with ONE low pass of 3 on the top edge, and a deep interior of 0.
        // The interior fills to 3 (the sill, the lowest pass out), never to 9.
        //   9 3 9 9
        //   9 0 0 9
        //   9 0 0 9
        //   9 9 9 9
        let elev = vec![9, 3, 9, 9, 9, 0, 0, 9, 9, 0, 0, 9, 9, 9, 9, 9];
        let d = priority_flood(4, 4, &elev);
        // The four interior cells (indices 5,6,9,10) fill to the sill 3.
        for i in [5usize, 6, 9, 10] {
            assert_eq!(d.filled[i], 3, "interior cell {i} fills to the sill 3");
        }
        // The pass and the rim are unchanged.
        assert_eq!(d.filled[1], 3);
        assert_eq!(d.filled[0], 9);
        // Every interior cell drains to the boundary through the pass.
        for i in [5usize, 6, 9, 10] {
            assert!(drains_to_boundary(&d, i, 16));
        }
    }

    #[test]
    fn a_monotone_slope_is_unchanged_and_fully_drained() {
        // A grid that already drains everywhere (no depression) is returned with its elevations intact,
        // and every cell reaches the boundary.
        let (w, h) = (5, 4);
        let elev: Vec<i64> = (0..(w * h)).map(|i| (i % w) as i64).collect(); // rises west-to-east
        let d = priority_flood(w, h, &elev);
        assert_eq!(d.filled, elev, "a fully-drained grid needs no filling");
        for i in 0..(w * h) {
            assert!(drains_to_boundary(&d, i, w * h));
        }
    }

    #[test]
    fn the_result_is_a_pure_function_worker_invariant() {
        // The filled grid and receivers are a pure function of the input: recomputing gives a bit-identical
        // result (the total order on (elevation, index) leaves no non-determinism for a worker split to
        // perturb). A representative rugged grid.
        let (w, h) = (6, 5);
        let elev: Vec<i64> = (0..(w * h)).map(|i| ((i * 37 + 11) % 13) as i64).collect();
        let a = priority_flood(w, h, &elev);
        let b = priority_flood(w, h, &elev);
        assert_eq!(a, b);
        // Every cell drains to the boundary and no interior sink survives.
        for (i, &e0) in elev.iter().enumerate() {
            assert!(drains_to_boundary(&a, i, w * h));
            assert!(a.filled[i] >= e0, "filling never lowers a cell");
        }
    }

    #[test]
    #[should_panic(expected = "must equal width*height")]
    fn a_mismatched_grid_length_panics() {
        priority_flood(3, 3, &[0, 1, 2]);
    }
}
