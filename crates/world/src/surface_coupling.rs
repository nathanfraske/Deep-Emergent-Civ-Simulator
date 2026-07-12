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

//! The redistribution-to-ledger coupling: it makes a non-local surface move (a runout or a fan) close
//! exactly against a SNAPSHOT-APPLY mass ledger, so what leaves the source cell equals what arrives at the
//! rest cells even when the source removal is CLAMPED by contention.
//!
//! The seam this closes (gate-ruled, PR #174). The redistribution operator ([`crate::redistribute`])
//! apportions a GIVEN mass across its destinations. A snapshot-apply ledger (the surface-transport
//! substrate's `reconcile_column`) does not always honor the full demand: when several writers contest one
//! column, or the column holds less than the demand, the reconciliation apportions the available mass and
//! returns an HONORED removal `H <= M`, the demand, so the column never goes negative. Credit the
//! destinations `M` while only `H` left the source and `M - H` of mass is fabricated, matter from nothing.
//! So the coupling keys the destination split off the HONORED removal `H`, never the demand `M`: the credits
//! sum to exactly what left.
//!
//! Two phases, no circular dependency. The destination WEIGHTS are the fan and runout physics and are fixed
//! before `H` is known; only the MASS scales with `H`. So the driver computes the weights (physics), the
//! ledger reconciles the source removal to `H` (contention), and this coupling splits `H` across the fixed
//! weights. The honored amount never feeds back into WHICH cells receive, only into HOW MUCH.
//!
//! Built against the CONTRACT SHAPE, raw `i64` mass units and the honored-removal value the reconciliation
//! returns, so it depends on no ledger type and is proven in isolation beside the operator. A non-local move
//! is a spatial relocation WITHIN one mass reservoir (the solid column), not a change of fate, so it needs
//! no reservoir transfer: the whole delta field sums to zero and the reservoir's global total is preserved.
//!
//! The honest limit (gate note, PR #174): this single clamp rests on the ledger applying ADDITIONS IN FULL
//! and apportioning only REMOVALS, which is how the contract reads today, so the destination side needs no
//! clamp. If a merged contract ever caps additions (a per-column maximum), a destination could reject
//! arriving mass and the coupling would gain a second seam, a second-order rebalance of the un-credited
//! remainder. That does not exist in the contract as written; it is flagged for the wire-up step to confirm
//! against the merged shape.

use crate::redistribute::{redistribute, RedistributeError, Redistribution, Weighted};

/// A non-local surface move as the DRIVER produces it, before the ledger reconciles the source: the source
/// cell, the DEMANDED solid mass the event sheds (the driver's own physics, from the runout energy and the
/// source's erodible solid), and the destination weights the runout or fan placed (fixed, independent of how
/// much the ledger will honor). Raw `i64` mass units, the contract shape, so this carries no ledger type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonLocalMove {
    /// The source cell shedding the mass, a flat grid index.
    pub source: usize,
    /// The demanded solid mass the event sheds (the driver's physics), before the ledger's clamp.
    pub demanded: i64,
    /// The destination weights the runout or fan placed, fixed before the honored amount is known.
    pub dests: Vec<Weighted>,
}

impl NonLocalMove {
    /// The signed demand the snapshot-apply ledger reconciles for the SOURCE column: a removal, so negative.
    /// The ledger clamps this against the source's tick snapshot and the other contesting writers and returns
    /// the honored removal magnitude, which phase two ([`NonLocalMove::apply_honored`]) then splits.
    pub fn source_demand(&self) -> i64 {
        self.demanded.saturating_neg()
    }

    /// Phase two: split the HONORED removal (the ledger's reconciliation output, never the demand) across the
    /// FIXED destination weights, producing the signed delta field of the whole non-local move. The source is
    /// debited `honored_removal` and the destinations credited additions that sum to EXACTLY `honored_removal`
    /// (the operator's exact-integer split), so the field sums to zero and the reservoir's global total holds.
    /// Keyed off `honored_removal`, so a clamped honored amount credits only what left the source: fabrication
    /// is impossible. A destination that equals the source (a parcel that rested in place) nets correctly, the
    /// source both debited the whole honored amount and credited its own share. Fails loud on the operator's
    /// error paths (an out-of-range index, a negative honored amount, no valid destination for a positive
    /// honored amount, an accumulation overflow), never dropping or fabricating mass.
    pub fn apply_honored(
        &self,
        honored_removal: i64,
        n_cells: usize,
    ) -> Result<Vec<i64>, RedistributeError> {
        redistribute(
            n_cells,
            &[Redistribution {
                source: self.source,
                mass: honored_removal,
                dests: self.dests.clone(),
            }],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(dest: usize, weight: u64) -> Weighted {
        Weighted { dest, weight }
    }

    fn mv(source: usize, demanded: i64, dests: Vec<Weighted>) -> NonLocalMove {
        NonLocalMove {
            source,
            demanded,
            dests,
        }
    }

    #[test]
    fn the_source_demand_is_the_negative_of_the_demanded_mass() {
        let m = mv(3, 250, vec![w(5, 1)]);
        assert_eq!(m.source_demand(), -250, "the ledger reconciles a removal");
    }

    #[test]
    fn a_fully_honored_move_credits_the_whole_demand() {
        // The uncontested case: the ledger honors the full demand, so the destinations receive it all and the
        // field closes to zero (this is the operator's plain conservative move).
        let m = mv(0, 120, vec![w(4, 2), w(6, 1)]);
        let delta = m.apply_honored(120, 9).unwrap();
        assert_eq!(delta.iter().sum::<i64>(), 0, "the move conserves mass");
        assert_eq!(delta[0], -120, "the source shed the full honored amount");
        assert_eq!(delta[4] + delta[6], 120, "the destinations received it all");
    }

    #[test]
    fn the_credits_sum_to_the_honored_removal_never_the_demand() {
        // The load-bearing case: the ledger clamped the demand of 100 to an honored 60 (the column was
        // contested or held less). The destinations must receive exactly 60, not 100, so the 40 the ledger
        // did not honor is never fabricated.
        let m = mv(0, 100, vec![w(3, 1), w(5, 1), w(7, 2)]);
        let delta = m.apply_honored(60, 9).unwrap();
        assert_eq!(
            delta[0], -60,
            "the source is debited only what the ledger honored"
        );
        assert_eq!(
            delta[3] + delta[5] + delta[7],
            60,
            "the destinations receive exactly the honored amount, never the demand"
        );
        assert_eq!(
            delta.iter().sum::<i64>(),
            0,
            "no mass is fabricated at the boundary"
        );
    }

    #[test]
    fn a_self_rest_parcel_nets_correctly_no_double_count() {
        // Gate note 2: a fan parcel that rested at the source makes that cell both the removal and a
        // destination in the same tick. Keyed off the honored amount, the source is debited the whole honored
        // removal and credited its own share, so it nets -honored + share and the field still closes to zero,
        // with no double-count.
        let m = mv(2, 80, vec![w(2, 1), w(3, 1)]); // one destination IS the source (cell 2)
        let delta = m.apply_honored(80, 9).unwrap();
        assert_eq!(
            delta.iter().sum::<i64>(),
            0,
            "the self-rest move conserves mass"
        );
        // 80 splits 40/40: the source keeps 40 (net -80 + 40 = -40), the neighbour gains 40.
        assert_eq!(
            delta[2], -40,
            "the source nets its debit minus its own share"
        );
        assert_eq!(delta[3], 40, "the true downstream cell gains its share");
    }

    #[test]
    fn the_destination_cells_are_fixed_independent_of_the_honored_amount() {
        // Gate note 1: the weights (which cells receive) are the physics, fixed before the honored amount is
        // known; only the magnitude scales with it. So the set of credited cells is the same at two different
        // honored amounts, and each destination's share scales with the honored total.
        let m = mv(0, 200, vec![w(2, 3), w(5, 1)]);
        let small = m.apply_honored(40, 9).unwrap();
        let large = m.apply_honored(160, 9).unwrap();
        let credited = |d: &[i64]| -> Vec<usize> { (0..d.len()).filter(|&i| d[i] > 0).collect() };
        assert_eq!(
            credited(&small),
            credited(&large),
            "the credited cells do not depend on the honored amount, only the physics"
        );
        // The heavier-weighted cell 2 gets three quarters at both amounts (30 of 40, 120 of 160).
        assert_eq!(small[2], 30);
        assert_eq!(large[2], 120);
    }

    #[test]
    fn the_honored_move_is_deterministic() {
        let m = mv(4, 91, vec![w(0, 3), w(5, 5), w(8, 2)]);
        let a = m.apply_honored(50, 9).unwrap();
        let b = m.apply_honored(50, 9).unwrap();
        assert_eq!(
            a, b,
            "the same honored move reproduces the same delta field"
        );
    }

    #[test]
    fn a_negative_or_unplaceable_honored_amount_fails_loud() {
        // A negative honored amount (the ledger never returns one, but the boundary refuses it) and a
        // positive honored amount with no weight to place it on both fail loud rather than fabricate or drop.
        let m = mv(0, 50, vec![w(3, 1)]);
        assert!(matches!(
            m.apply_honored(-1, 9),
            Err(RedistributeError::NegativeMass { .. })
        ));
        let no_dest = mv(0, 50, vec![w(3, 0)]);
        assert!(matches!(
            no_dest.apply_honored(50, 9),
            Err(RedistributeError::NoDestination { .. })
        ));
        let out_of_range = mv(0, 50, vec![w(99, 1)]);
        assert!(matches!(
            out_of_range.apply_honored(50, 9),
            Err(RedistributeError::IndexOutOfRange { .. })
        ));
    }

    #[test]
    fn a_zero_honored_move_is_a_no_op() {
        // The ledger honored nothing (a fully contested or empty source): the move places nothing and the
        // field is untouched, so a source that lost its whole snapshot to a prior writer shifts no mass here.
        let m = mv(0, 100, vec![w(3, 1), w(5, 1)]);
        let delta = m.apply_honored(0, 9).unwrap();
        assert_eq!(delta, vec![0i64; 9], "a zero-honored move moves nothing");
    }
}
