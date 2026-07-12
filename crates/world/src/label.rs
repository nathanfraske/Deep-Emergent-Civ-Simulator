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

//! Canonical connected-components labelling, a Layer-0 determinism primitive for the genesis-forward
//! geology solvers (plate identity, watershed identity, and any grid partition into connected regions).
//!
//! The determinism property is the whole point (Principle 3, and R-CANON-WALK's discipline applied to a
//! solver rather than a container): each cell's component LABEL is the lowest cell index in its connected
//! component, so the labelling is a pure function of the component's content, never of the order edges are
//! visited or of how the work is split across threads. A parallel labeller and a serial one produce
//! bit-identical labels, because the label is defined by the region, not discovered by a walk. This is
//! what lets a plate or a watershed carry a stable identity a state hash can fold without a canonical-walk
//! hazard.
//!
//! The mechanism is a union-find whose union always keeps the SMALLER index as the representative, so a
//! component's representative is its global minimum index independent of union order; path compression
//! only shortens chains and never moves a representative. The caller supplies the same-component adjacency
//! (the edges that join two cells into one region), so the primitive is grid-shape-agnostic: a geology
//! solver builds the edges from its own grid and its own same-plate or flows-to-the-same-outlet predicate.

/// A union-find (disjoint-set) whose representative of a set is always its minimum member index, so the
/// labelling it produces is order-independent and worker-invariant.
struct MinRootUnionFind {
    parent: Vec<usize>,
}

impl MinRootUnionFind {
    fn new(n: usize) -> MinRootUnionFind {
        MinRootUnionFind {
            parent: (0..n).collect(),
        }
    }

    /// The representative (minimum index) of `x`'s set, with path compression. Iterative, so a long chain
    /// never overflows the call stack.
    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // Compress: point every node on the path directly at the root.
        let mut node = x;
        while self.parent[node] != node {
            let next = self.parent[node];
            self.parent[node] = root;
            node = next;
        }
        root
    }

    /// Join the sets of `a` and `b`, keeping the smaller index as the representative so the union is
    /// order-independent (the merged set's representative is the minimum of the two representatives).
    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[hi] = lo;
        }
    }
}

/// Label the connected components of `n` cells joined by `edges` (the same-component adjacency), returning
/// each cell's label: the lowest cell index in its connected component. Deterministic and worker-invariant:
/// the result is independent of the order the edges are supplied, so a parallel build that produces the
/// edges in any order and a serial one label identically. A cell in no edge is its own singleton component
/// (its label is itself). An edge referencing an index at or beyond `n` is ignored rather than panicking,
/// so a caller's stray edge cannot crash the solver.
pub fn label_components(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut uf = MinRootUnionFind::new(n);
    for &(a, b) in edges {
        if a < n && b < n {
            uf.union(a, b);
        }
    }
    (0..n).map(|i| uf.find(i)).collect()
}

/// Label the connected components of a `width` by `height` grid under 4-connectivity (each cell joined to
/// its orthogonal neighbour when `connected(a, b)` holds for the two flat cell indices `a = y*width + x`).
/// A convenience over [`label_components`] for the grid case the geology solvers use; the labels are the
/// same lowest-index canonical identities, worker-invariant.
pub fn label_grid_4(
    width: usize,
    height: usize,
    connected: impl Fn(usize, usize) -> bool,
) -> Vec<usize> {
    let n = width * height;
    let mut edges = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            // Only the east and south neighbours, so each undirected edge is built once.
            if x + 1 < width {
                let e = i + 1;
                if connected(i, e) {
                    edges.push((i, e));
                }
            }
            if y + 1 < height {
                let s = i + width;
                if connected(i, s) {
                    edges.push((i, s));
                }
            }
        }
    }
    label_components(n, &edges)
}

/// The number of distinct connected components in a labelling (the count of cells that are their own
/// label, the component representatives).
pub fn component_count(labels: &[usize]) -> usize {
    labels.iter().enumerate().filter(|(i, &l)| *i == l).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_component_is_labelled_by_its_lowest_index() {
        // Two components: {0,1,2} and {4,5}; cell 3 is a singleton. Labels are the component minima.
        let edges = [(1, 2), (0, 1), (5, 4)];
        let labels = label_components(6, &edges);
        assert_eq!(labels, vec![0, 0, 0, 3, 4, 4]);
        assert_eq!(component_count(&labels), 3);
    }

    #[test]
    fn the_labelling_is_order_invariant_worker_invariant() {
        // The same edges in any order must give bit-identical labels: that is the worker-invariance a
        // parallel solver relies on. Build one labelling, then permute the edges every which way.
        let base = [(0, 1), (1, 2), (2, 3), (5, 6), (8, 4), (4, 0)];
        let reference = label_components(9, &base);
        // Reversed order.
        let mut rev = base.to_vec();
        rev.reverse();
        assert_eq!(label_components(9, &rev), reference);
        // A rotated order.
        let mut rot = base.to_vec();
        rot.rotate_left(3);
        assert_eq!(label_components(9, &rot), reference);
        // Each cell's label is the minimum index reachable from it, independent of how it was discovered.
        // Component {0,1,2,3,4,8} has minimum 0; {5,6} has minimum 5; {7} is a singleton.
        assert_eq!(reference, vec![0, 0, 0, 0, 0, 5, 5, 7, 0]);
    }

    #[test]
    fn a_grid_partitions_into_lowest_index_regions() {
        // A 3x2 grid split into a left 2x2-ish region and a right column by a predicate that severs the
        // seam between column 1 and column 2. Cells: indices 0..6, width 3.
        //   0 1 | 2
        //   3 4 | 5
        let labels = label_grid_4(3, 2, |a, b| {
            // Connected unless the edge crosses the column-1-to-column-2 seam.
            let (ax, bx) = (a % 3, b % 3);
            !((ax == 1 && bx == 2) || (ax == 2 && bx == 1))
        });
        // Left region {0,1,3,4} -> label 0; right column {2,5} -> label 2.
        assert_eq!(labels, vec![0, 0, 2, 0, 0, 2]);
        assert_eq!(component_count(&labels), 2);
    }

    #[test]
    fn singletons_and_empty_and_stray_edges_are_safe() {
        // No edges: every cell is its own component.
        assert_eq!(label_components(3, &[]), vec![0, 1, 2]);
        // Zero cells: an empty labelling, no panic.
        assert_eq!(label_components(0, &[]), Vec::<usize>::new());
        // A stray edge referencing an out-of-range index is ignored, not a panic.
        assert_eq!(label_components(2, &[(0, 9), (0, 1)]), vec![0, 0]);
    }
}
