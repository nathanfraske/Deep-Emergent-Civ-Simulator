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

//! Non-local conservative redistribution, a Layer-0 determinism primitive for the genesis-forward
//! geology surface drivers (the ballistic and gravity-driven mass transports that neither local
//! drainage `priority_flood` nor local relaxation `fixed_cap_solve` expresses).
//!
//! The transport already on the grid is LOCAL: a cell's mass advects to its single downhill neighbour
//! through an order-independent double-buffered stencil. A surface driver like an impact ejecta blanket or
//! a gravity-driven runout is NON-local: one source cell sheds its mass to many cells at once, some far
//! away. This primitive is that operator, and only that operator: it takes each source cell's mass and an
//! already-computed distribution over destination cells and produces the conservative signed delta field
//! the mass move implies. It does NOT decide WHERE the mass goes; the distribution is the caller's, so the
//! physical kernels that place the destinations (ballistic, granular) stay a separate, gated design.
//!
//! Two determinism hazards a non-local scatter has, and how this primitive removes both (Principle 3,
//! Principle 10). First, an integer split of a mass across weighted destinations that did not close would
//! fabricate or lose mass; the apportionment here is exact (the destination integers sum to the source
//! mass exactly, by the largest-remainder method with a lowest-index tie-break, so the split is both fair
//! and canonical). Second, when several sources credit one destination, a floating or saturating
//! accumulation would let the order leak into the result; the accumulation here is exact-integer addition
//! into a fresh delta buffer (associative and commutative, so order-independent and worker-invariant), read
//! from the caller's source list rather than a field mutated mid-pass (the double-buffer), and it fails
//! loud on overflow rather than saturating. So the delta field is a pure function of the inputs: a parallel
//! and a serial application are bit-identical, and the field sums to exactly zero (mass is moved, never
//! created or destroyed).

/// A weighted destination of a redistribution: the destination cell (a flat grid index) and its
/// non-negative weight. The weights set the PROPORTIONS the source mass splits in; their absolute scale
/// does not matter (only the ratios), so a kernel may return raw physical weights without normalising.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weighted {
    /// The destination cell, a flat index into the field.
    pub dest: usize,
    /// The destination's non-negative weight (its share of the source mass is `weight / sum-of-weights`).
    pub weight: u64,
}

/// One source cell's redistribution: the source cell, the non-negative integer mass leaving it, and the
/// weighted destinations it splits into. The mass is in the field's own integer units (for a fixed-point
/// field, its raw bits), so the split is exact in those units. A destination may equal the source (a
/// fraction of the mass stays), and the destinations need not be adjacent (the non-local reach).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redistribution {
    /// The source cell shedding the mass, a flat index into the field.
    pub source: usize,
    /// The non-negative integer mass leaving the source (in the field's own units).
    pub mass: i64,
    /// The weighted destinations the mass splits across.
    pub dests: Vec<Weighted>,
}

/// Why a redistribution could not be applied. Each is a fail-loud refusal, never a silent drop of mass,
/// so a caller cannot lose or fabricate mass without an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedistributeError {
    /// A cell index (source or destination) is at or beyond the field size.
    IndexOutOfRange {
        /// The offending flat index.
        index: usize,
        /// The field size it must be below.
        n_cells: usize,
    },
    /// The mass leaving a source is negative; a redistribution moves a non-negative amount out of a cell.
    NegativeMass {
        /// The source cell.
        source: usize,
        /// The offending mass.
        mass: i64,
    },
    /// A source has positive mass but nowhere to send it (no destinations, or every weight zero), so the
    /// mass could not be placed without vanishing; refused rather than dropped.
    NoDestination {
        /// The source cell whose mass could not be placed.
        source: usize,
    },
    /// Accumulating a destination's credited mass overflowed the integer field range; refused rather than
    /// saturated (a saturate would be order-dependent and would break conservation).
    Overflow {
        /// The cell whose accumulation overflowed.
        cell: usize,
    },
}

/// Apportion `mass` (non-negative) across `dests` by weight into exact integers that sum to `mass`, by the
/// largest-remainder method: each destination gets `floor(mass * weight / total_weight)`, and the leftover
/// units (at most one per destination) go to the destinations with the largest fractional remainders, ties
/// broken by the lowest destination position. Deterministic (the tie-break is a total order over positions)
/// and exact (the parts sum to `mass`). Returns the per-destination integer parts aligned with `dests`, or
/// `None` if the mass is positive and the total weight is zero (nothing to apportion onto). A zero mass
/// yields all-zero parts even when the total weight is zero.
fn apportion(mass: i64, dests: &[Weighted]) -> Option<Vec<i64>> {
    debug_assert!(
        mass >= 0,
        "apportion is called only with a non-negative mass"
    );
    if mass == 0 {
        return Some(vec![0; dests.len()]);
    }
    let total: u128 = dests.iter().map(|d| d.weight as u128).sum();
    if total == 0 {
        return None;
    }
    let m = mass as u128;
    // The exact floor share and the fractional remainder (over the common denominator `total`) per cell.
    let mut parts: Vec<i64> = Vec::with_capacity(dests.len());
    let mut remainders: Vec<u128> = Vec::with_capacity(dests.len());
    let mut assigned: u128 = 0;
    for d in dests {
        let numer = m * d.weight as u128;
        let floor = numer / total;
        parts.push(floor as i64);
        remainders.push(numer % total);
        assigned += floor;
    }
    // The leftover units the floors did not place (always fewer than the destination count). Give one each
    // to the largest remainders, lowest position first on a tie, so the split closes to `mass` canonically.
    let leftover = (m - assigned) as usize;
    if leftover > 0 {
        let mut order: Vec<usize> = (0..dests.len()).collect();
        order.sort_by(|&a, &b| remainders[b].cmp(&remainders[a]).then(a.cmp(&b)));
        for &i in order.iter().take(leftover) {
            parts[i] += 1;
        }
    }
    Some(parts)
}

/// Apply a set of non-local redistributions to an `n_cells` field as a conservative signed delta field:
/// each source is debited its mass and each destination credited its apportioned share, so the returned
/// deltas sum to exactly zero (mass is moved, never created or destroyed). Deterministic and
/// worker-invariant: the per-destination split is the canonical largest-remainder apportionment, the
/// accumulation is exact-integer addition (associative, so order-independent), and the fail-loud overflow
/// names the LOWEST cell index whose net does not fit the field range, not a mid-accumulation partial the
/// move order could pick, so both the delta field AND the refusal are pure functions of `moves` regardless
/// of order or work split. Fails loud on an out-of-range index, a negative mass, a source with positive
/// mass but no valid destination, or a net that overflows the field range, never dropping or saturating.
pub fn redistribute(
    n_cells: usize,
    moves: &[Redistribution],
) -> Result<Vec<i64>, RedistributeError> {
    // Accumulate the net per cell in a wide integer so no running partial overflows within the pass; the
    // associativity of exact-integer addition then makes the accumulation independent of the move order, and
    // the single narrowing pass below turns any out-of-range net into a canonical (lowest-index) refusal.
    let mut acc = vec![0i128; n_cells];
    for mv in moves {
        if mv.source >= n_cells {
            return Err(RedistributeError::IndexOutOfRange {
                index: mv.source,
                n_cells,
            });
        }
        if mv.mass < 0 {
            return Err(RedistributeError::NegativeMass {
                source: mv.source,
                mass: mv.mass,
            });
        }
        for d in &mv.dests {
            if d.dest >= n_cells {
                return Err(RedistributeError::IndexOutOfRange {
                    index: d.dest,
                    n_cells,
                });
            }
        }
        if mv.mass == 0 {
            continue;
        }
        let parts = apportion(mv.mass, &mv.dests)
            .ok_or(RedistributeError::NoDestination { source: mv.source })?;
        // Debit the source, then credit each destination its exact part. The i128 net cannot overflow for
        // any grid that fits in memory (each move adds at most an i64 magnitude), so the wide add is safe.
        acc[mv.source] -= mv.mass as i128;
        for (d, &part) in mv.dests.iter().zip(&parts) {
            acc[d.dest] += part as i128;
        }
    }
    // Narrow each net back to the field's i64 range in cell order, so an overflow names the lowest cell that
    // does not fit (canonical and order-independent), never a mid-accumulation partial.
    let mut delta = vec![0i64; n_cells];
    for (cell, &net) in acc.iter().enumerate() {
        delta[cell] = i64::try_from(net).map_err(|_| RedistributeError::Overflow { cell })?;
    }
    Ok(delta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(dest: usize, weight: u64) -> Weighted {
        Weighted { dest, weight }
    }

    #[test]
    fn an_even_split_places_every_unit_and_closes() {
        // 100 units over four equal destinations: 25 each, sum 100.
        let parts = apportion(100, &[w(1, 1), w(2, 1), w(3, 1), w(4, 1)]).unwrap();
        assert_eq!(parts, vec![25, 25, 25, 25]);
        assert_eq!(parts.iter().sum::<i64>(), 100);
    }

    #[test]
    fn an_uneven_split_gives_the_leftover_to_the_largest_remainder_lowest_index_first() {
        // 10 units over three equal weights: floor 3 each (9), one leftover. Equal remainders, so the
        // lowest position takes it: [4, 3, 3], sum 10.
        let parts = apportion(10, &[w(0, 1), w(1, 1), w(2, 1)]).unwrap();
        assert_eq!(parts, vec![4, 3, 3]);
        assert_eq!(parts.iter().sum::<i64>(), 10);
    }

    #[test]
    fn a_weighted_split_follows_the_weights_and_still_closes() {
        // 10 units split 3:1: floors 7 and 2 (remainders 2/4 and 2/4, equal), one leftover to the lowest
        // position, so [8, 2], sum 10.
        let parts = apportion(10, &[w(0, 3), w(1, 1)]).unwrap();
        assert_eq!(parts.iter().sum::<i64>(), 10);
        assert_eq!(parts, vec![8, 2]);
    }

    #[test]
    fn the_apportionment_is_exact_for_a_range_of_masses_and_weights() {
        // Over many masses and a fixed uneven weight set, the parts always sum to the mass exactly (the
        // conservation core), and no part is negative.
        let dests = [w(0, 5), w(1, 3), w(2, 2), w(3, 7), w(4, 1)];
        for mass in 0..500i64 {
            let parts = apportion(mass, &dests).unwrap();
            assert_eq!(parts.iter().sum::<i64>(), mass, "closes for mass {mass}");
            assert!(parts.iter().all(|&p| p >= 0));
        }
    }

    #[test]
    fn zero_mass_places_nothing_even_with_zero_total_weight() {
        assert_eq!(apportion(0, &[w(0, 0), w(1, 0)]).unwrap(), vec![0, 0]);
        assert_eq!(apportion(0, &[]).unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn positive_mass_with_no_weight_cannot_be_placed() {
        assert!(apportion(5, &[w(0, 0), w(1, 0)]).is_none());
        assert!(apportion(5, &[]).is_none());
    }

    #[test]
    fn a_redistribution_conserves_mass_the_delta_field_sums_to_zero() {
        // One source sheds 100 to three distant cells; the source is debited 100 and the destinations
        // credited 100 in total, so the whole delta field sums to zero.
        let moves = [Redistribution {
            source: 0,
            mass: 100,
            dests: vec![w(5, 2), w(7, 1), w(9, 1)],
        }];
        let delta = redistribute(16, &moves).unwrap();
        assert_eq!(delta.iter().sum::<i64>(), 0, "mass is moved, never created");
        assert_eq!(delta[0], -100, "the source is debited its whole mass");
        assert_eq!(
            delta[5] + delta[7] + delta[9],
            100,
            "the destinations receive it all"
        );
        assert!(
            delta[5] >= delta[7] && delta[5] >= delta[9],
            "the heaviest weight gets the most"
        );
    }

    #[test]
    fn a_source_that_is_its_own_destination_nets_correctly() {
        // A fraction of the mass stays: the source is both debited its mass and credited its own share, so
        // its net delta is the mass it sheds after keeping its own share. Conservation still holds.
        let moves = [Redistribution {
            source: 3,
            mass: 10,
            dests: vec![w(3, 1), w(4, 1)],
        }];
        let delta = redistribute(9, &moves).unwrap();
        assert_eq!(delta.iter().sum::<i64>(), 0);
        // 10 splits 5/5: source keeps 5 (net -10 + 5 = -5), the neighbour gains 5.
        assert_eq!(delta[3], -5);
        assert_eq!(delta[4], 5);
    }

    #[test]
    fn overlapping_destinations_from_many_sources_accumulate_order_independently() {
        // Two sources both credit the same destination. Exact-integer accumulation is commutative, so the
        // result is identical whichever order the sources are applied: the worker-invariance property.
        let a = Redistribution {
            source: 0,
            mass: 8,
            dests: vec![w(4, 1)],
        };
        let b = Redistribution {
            source: 1,
            mass: 6,
            dests: vec![w(4, 1)],
        };
        let forward = redistribute(9, &[a.clone(), b.clone()]).unwrap();
        let reversed = redistribute(9, &[b, a]).unwrap();
        assert_eq!(
            forward, reversed,
            "the delta field does not depend on source order"
        );
        assert_eq!(
            forward[4], 14,
            "the shared destination gathers both sources"
        );
        assert_eq!(forward.iter().sum::<i64>(), 0);
    }

    #[test]
    fn the_redistribution_is_deterministic() {
        let moves = [
            Redistribution {
                source: 2,
                mass: 37,
                dests: vec![w(0, 3), w(5, 5), w(8, 2)],
            },
            Redistribution {
                source: 8,
                mass: 91,
                dests: vec![w(1, 1), w(2, 1), w(3, 1), w(4, 1)],
            },
        ];
        let x = redistribute(9, &moves).unwrap();
        let y = redistribute(9, &moves).unwrap();
        assert_eq!(x, y, "the same inputs reproduce the same delta field");
        assert_eq!(x.iter().sum::<i64>(), 0);
    }

    #[test]
    fn a_zero_mass_move_is_a_no_op() {
        let moves = [Redistribution {
            source: 0,
            mass: 0,
            dests: vec![w(1, 1)],
        }];
        let delta = redistribute(4, &moves).unwrap();
        assert_eq!(delta, vec![0, 0, 0, 0]);
    }

    #[test]
    fn an_out_of_range_destination_fails_loud() {
        let moves = [Redistribution {
            source: 0,
            mass: 5,
            dests: vec![w(9, 1)],
        }];
        assert_eq!(
            redistribute(4, &moves),
            Err(RedistributeError::IndexOutOfRange {
                index: 9,
                n_cells: 4
            })
        );
    }

    #[test]
    fn an_out_of_range_source_fails_loud() {
        let moves = [Redistribution {
            source: 9,
            mass: 5,
            dests: vec![w(0, 1)],
        }];
        assert_eq!(
            redistribute(4, &moves),
            Err(RedistributeError::IndexOutOfRange {
                index: 9,
                n_cells: 4
            })
        );
    }

    #[test]
    fn a_negative_mass_fails_loud() {
        let moves = [Redistribution {
            source: 0,
            mass: -1,
            dests: vec![w(1, 1)],
        }];
        assert_eq!(
            redistribute(4, &moves),
            Err(RedistributeError::NegativeMass {
                source: 0,
                mass: -1
            })
        );
    }

    #[test]
    fn positive_mass_with_no_valid_destination_fails_loud() {
        // No destinations at all, and all-zero weights: both refuse rather than drop the mass.
        let none = [Redistribution {
            source: 0,
            mass: 5,
            dests: vec![],
        }];
        assert_eq!(
            redistribute(4, &none),
            Err(RedistributeError::NoDestination { source: 0 })
        );
        let zero_weight = [Redistribution {
            source: 0,
            mass: 5,
            dests: vec![w(1, 0), w(2, 0)],
        }];
        assert_eq!(
            redistribute(4, &zero_weight),
            Err(RedistributeError::NoDestination { source: 0 })
        );
    }

    #[test]
    fn an_accumulation_overflow_fails_loud_rather_than_saturating() {
        // A destination already near the integer ceiling that would overflow on the credit is refused, not
        // saturated (a saturate would break conservation and be order-dependent).
        let moves = [
            Redistribution {
                source: 0,
                mass: i64::MAX,
                dests: vec![w(2, 1)],
            },
            Redistribution {
                source: 1,
                mass: i64::MAX,
                dests: vec![w(2, 1)],
            },
        ];
        assert_eq!(
            redistribute(4, &moves),
            Err(RedistributeError::Overflow { cell: 2 })
        );
    }
}
