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

//! Wiring the non-local redistribution primitive to the surface-mass ledger's snapshot-apply reconciliation
//! (the coupling slice 2, PR #174). The primitive and its honored-removal closure live in the world crate
//! ([`civsim_world::surface_coupling::NonLocalMove`]); the four-reservoir ledger and its per-column
//! reconciliation live in this crate ([`crate::surface_transport`]). The world crate is upstream of this one,
//! so the join lives HERE, and it is a thin binding: the closure was proven in isolation, this makes it live
//! against the real ledger.
//!
//! The two-phase move, now against the real reconciliation. A mass-flow or ejecta driver produces, at a source
//! cell, a [`NonLocalMove`] (the demanded solid mass and the runout or fan's destination weights). The
//! source column's removal is one writer's negative demand; the ledger's [`reconcile_column`] apportions it
//! against the source snapshot and the other writers contesting the column and returns the HONORED signed
//! removal (the applied entry for this writer), which is at most the demand and never drives the column
//! negative. That honored magnitude, not the demand, is fed to the closure's destination split, so the rest
//! cells receive exactly what left the source: the seam the slice-1 proof closed, now bound to the real
//! reconciliation output.
//!
//! The single clamp is correct against the merged contract (confirmed at source, PR #174): `reconcile_column`
//! applies ADDITIONS in full and apportions only REMOVALS, so the destination side needs no clamp and the
//! second seam (a destination rejecting arriving mass under an addition cap) does not exist here. A non-local
//! move relocates mass WITHIN the [`crate::surface_transport::MassReservoir::ColumnSolid`] fate (a spatial move, not a change of fate),
//! so it needs no [`crate::surface_transport::SurfaceMassBudget`] transfer: the per-cell delta sums to zero and the fate's global total
//! is preserved, which is what keeps the budget conserved across the move.
//!
//! Byte-neutral: this binding is built and tested but armed by no scenario (no run-path caller), so the
//! canonical pins hold, the same discipline slice 1 kept. The full multi-driver tick (all writers' source
//! removals reconciled against the tick snapshot, then the honored masses applied as the destination
//! additions) is the arming step, sequenced when a scenario runs the surface-transport drivers.

use civsim_world::redistribute::RedistributeError;
use civsim_world::surface_coupling::NonLocalMove;

use crate::surface_transport::reconcile_column;

/// Reconcile a non-local move's SOURCE removal through the ledger's snapshot-apply, then split the HONORED
/// removal across the move's fixed destination weights, returning the signed per-cell delta field that closes
/// to zero. `source_available` is the source column's snapshot solid mass (raw `i64` bits), and
/// `other_source_demands` are the signed demands of the OTHER writers contesting the source column this tick
/// (empty when the move is the column's only writer). The move's own writer is placed first, so the honored
/// removal is the magnitude of the first applied entry. The destinations receive exactly the honored amount,
/// never the demand, so a contested or insufficient source cannot fabricate mass. Fails loud on the closure's
/// error paths (an out-of-range index, no valid destination for a positive honored amount, an overflow).
pub fn reconcile_and_apply(
    mv: &NonLocalMove,
    source_available: i64,
    other_source_demands: &[i64],
    n_cells: usize,
) -> Result<Vec<i64>, RedistributeError> {
    let mut demands = Vec::with_capacity(1 + other_source_demands.len());
    demands.push(mv.source_demand());
    demands.extend_from_slice(other_source_demands);
    let (applied, _net) = reconcile_column(source_available, &demands);
    // The move's writer is index 0; its applied entry is the honored signed removal (at most zero). The
    // magnitude is exactly what the ledger let leave the source, and that is what the destinations share.
    let honored = applied[0].saturating_neg();
    mv.apply_honored(honored, n_cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_transport::{MassReservoir, SurfaceMassBudget};
    use civsim_core::Fixed;
    use civsim_world::redistribute::Weighted;

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
    fn an_uncontested_move_closes_fully_against_the_ledger() {
        // The source holds more than the demand and no other writer contests it, so the ledger honors the
        // full demand and the destinations receive it all: the plain conservative move, now through the real
        // reconcile_column.
        let m = mv(0, 100, vec![w(3, 2), w(6, 1)]);
        let delta = reconcile_and_apply(&m, 1000, &[], 9).unwrap();
        assert_eq!(delta.iter().sum::<i64>(), 0, "the move conserves mass");
        assert_eq!(delta[0], -100, "the source shed the full demand");
        assert_eq!(delta[3] + delta[6], 100, "the destinations received it all");
    }

    #[test]
    fn a_contested_source_honors_only_the_share_and_never_fabricates() {
        // The source holds 90 but two writers demand 100 (mine) and 50 (another), 150 total. The ledger
        // apportions the 90 available: mine is honored 90 * 100/150 = 60. The destinations receive exactly
        // that 60, never the demanded 100, so the 40 the ledger did not honor is never fabricated.
        let m = mv(0, 100, vec![w(3, 1), w(5, 1), w(7, 2)]);
        let delta = reconcile_and_apply(&m, 90, &[-50], 9).unwrap();
        assert_eq!(
            delta[0], -60,
            "the source is debited only its honored share"
        );
        assert_eq!(
            delta[3] + delta[5] + delta[7],
            60,
            "the destinations receive exactly what the ledger honored"
        );
        assert_eq!(
            delta.iter().sum::<i64>(),
            0,
            "no mass is fabricated at the boundary"
        );
    }

    #[test]
    fn an_insufficient_source_honors_only_what_it_holds() {
        // The source holds only 30 but the move demands 80 and nothing else contests it: the removal is
        // limited to the snapshot, so the honored amount is 30 and the destinations receive 30.
        let m = mv(1, 80, vec![w(4, 1), w(5, 1)]);
        let delta = reconcile_and_apply(&m, 30, &[], 9).unwrap();
        assert_eq!(delta[1], -30, "the source cannot shed more than it holds");
        assert_eq!(
            delta[4] + delta[5],
            30,
            "the destinations receive the honored 30"
        );
        assert_eq!(delta.iter().sum::<i64>(), 0);
    }

    #[test]
    fn the_surface_mass_budget_stays_conserved_across_the_move() {
        // A non-local move relocates mass WITHIN the solid column, so the global ColumnSolid fate total is
        // unchanged and the four-reservoir budget stays conserved. The per-cell field's bit-sum is the global
        // solid mass; the move preserves it (the delta sums to zero), so the budget the ledger tracks holds.
        let field: Vec<i64> = vec![0, 0, 0, 500, 0, 0, 0, 0, 0]; // all the solid at the source cell 3
        let sum: i64 = field.iter().sum();
        let budget = SurfaceMassBudget::seeded(Fixed::from_bits(sum));
        let opening = budget.total();
        let m = mv(3, 500, vec![w(0, 1), w(8, 1)]);
        let delta = reconcile_and_apply(&m, field[3], &[], 9).unwrap();
        let moved: Vec<i64> = field.iter().zip(&delta).map(|(&f, &d)| f + d).collect();
        assert_eq!(
            moved.iter().sum::<i64>(),
            sum,
            "the relocation preserves the global solid mass"
        );
        assert_eq!(
            Fixed::from_bits(moved.iter().sum()),
            budget.balance(MassReservoir::ColumnSolid),
            "the moved field's total matches the ledger's ColumnSolid balance"
        );
        assert!(
            budget.is_conserved(opening),
            "a within-fate spatial move leaves the four-reservoir budget conserved"
        );
    }

    #[test]
    fn the_wire_up_is_deterministic() {
        let m = mv(4, 91, vec![w(0, 3), w(5, 5), w(8, 2)]);
        let a = reconcile_and_apply(&m, 70, &[-20], 9).unwrap();
        let b = reconcile_and_apply(&m, 70, &[-20], 9).unwrap();
        assert_eq!(
            a, b,
            "the same reconciled move reproduces the same delta field"
        );
    }
}
