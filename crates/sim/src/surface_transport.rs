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

//! The surface-mass-transport substrate (genesis-forward Stage 3, the surface lane). Design in
//! `docs/working/GENESIS_STAGE3_SURFACE_TRANSPORT_SUBSTRATE.md`. Surface mass transport is a DATA-DEFINED,
//! EXTENSIBLE driver substrate: the transport-and-deposition solve over the driver kernels is the fixed Rust,
//! the driver MEMBERSHIP is data (a driver is a data row carrying its transport-law form, its property key-set,
//! its primitive, and the conservation reservoirs its mass touches). This module holds the substrate; this first
//! slice is the CONSERVATION LEDGER the whole budget closes against.
//!
//! [`SurfaceMassBudget`] is the FOUR-RESERVOIR conservation ledger. A pure-erosion budget with only
//! column-to-column deposition cannot close its mass budget for a dissolving, a volatile, or a low-gravity
//! world, because mass also leaves the solid column into other fates. The four reservoirs are the complete set
//! of MASS FATES on a surface, so they are the conservation FLOOR (physics, authorable), not world content: the
//! solid COLUMN (the elevation ledger), the DISSOLVED load (mass a chemical driver carries in solution before it
//! precipitates), the atmospheric VAPOR (mass a phase-change driver carries in transit before it redeposits),
//! and permanent LOSS to space (a sublimated volatile or ejecta above escape velocity on a low-gravity world).
//! The DRIVERS (data rows) move mass between these fixed reservoirs; the total across all four is invariant
//! under those moves, so the budget closes exactly under fixed-point arithmetic (Principle 3), and it declares
//! that total as its conserved projection to the Part-58 [`crate::conservation::ConservationRegistry`] when a
//! genesis pass arms it. Off the run path until then, a pure addition.

use civsim_core::Fixed;

/// The four MASS-FATE reservoirs of surface mass transport, the fixed conservation-floor accounts every driver's
/// mass moves between. Not world content and not a data-driven set: mass on a surface is in exactly one of these
/// four fates, so the set is closed by the physics of mass conservation (unlike the DRIVER membership, which is
/// data). The distinction the second design smoke test forced: a single column-to-column sink cannot close the
/// budget for a dissolving or a volatile or a low-gravity world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MassReservoir {
    /// The solid column, the mass resident in the elevation ledger (the [`crate::material::EarthworkField`]
    /// geological delta over the worldgen base). Erosion and deposition move mass column to column WITHIN this
    /// reservoir, so they leave its total unchanged; only a change of fate (dissolution, vaporization, escape)
    /// moves mass out of it.
    ColumnSolid,
    /// The dissolved load, mass a chemical-alteration driver removes into solution, carried until it
    /// precipitates back to the solid column as chemical sediment.
    DissolvedLoad,
    /// The atmospheric vapor, mass a phase-change driver sublimates and carries as vapor along the saturation
    /// gradient until it redeposits to the solid column.
    AtmosphericVapor,
    /// Permanent loss to space, mass sublimated or ejected above escape velocity on a low-gravity world. A
    /// terminal fate: mass here never returns, so it is the budget's only true boundary sink.
    LostToSpace,
}

/// The four-reservoir SURFACE-MASS BUDGET, the conservation ledger the surface-transport substrate closes
/// against. It holds the global mass in each of the four fixed [`MassReservoir`] fates and moves mass between
/// them through the one conserving [`Self::transfer`] primitive, so the TOTAL across all four is invariant by
/// construction (a move subtracts from one account exactly what it adds to another). The total is the conserved
/// projection the subsystem declares to the Part-58 registry; [`Self::is_conserved`] against the opening total
/// is the local check. Deterministic fixed-point arithmetic (Principle 3). Empty and off the run path until a
/// genesis pass arms it (all reservoirs default to zero), so declaring it is byte-neutral.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceMassBudget {
    column_solid: Fixed,
    dissolved_load: Fixed,
    atmospheric_vapor: Fixed,
    lost_to_space: Fixed,
}

/// A refused move on the [`SurfaceMassBudget`]: the requested transfer exceeded the source reservoir's balance,
/// which would drive an account negative and silently break conservation. Refused fail-loud rather than clamped,
/// so a driver that tries to move more mass than a fate holds is a caught defect, never a laundered leak.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsufficientMass;

impl SurfaceMassBudget {
    /// An empty budget, all four reservoirs at zero. The opt-out state a scenario that arms no transport stays
    /// in.
    pub fn new() -> SurfaceMassBudget {
        SurfaceMassBudget::default()
    }

    /// A budget seeded with `column_solid` mass in the solid column and the other three fates empty, the genesis
    /// opening state (all surface mass starts in the solid crust). The opening [`Self::total`] is what
    /// [`Self::is_conserved`] later checks against.
    pub fn seeded(column_solid: Fixed) -> SurfaceMassBudget {
        SurfaceMassBudget {
            column_solid,
            ..SurfaceMassBudget::default()
        }
    }

    /// The mass held in a reservoir.
    pub fn balance(&self, reservoir: MassReservoir) -> Fixed {
        match reservoir {
            MassReservoir::ColumnSolid => self.column_solid,
            MassReservoir::DissolvedLoad => self.dissolved_load,
            MassReservoir::AtmosphericVapor => self.atmospheric_vapor,
            MassReservoir::LostToSpace => self.lost_to_space,
        }
    }

    fn balance_mut(&mut self, reservoir: MassReservoir) -> &mut Fixed {
        match reservoir {
            MassReservoir::ColumnSolid => &mut self.column_solid,
            MassReservoir::DissolvedLoad => &mut self.dissolved_load,
            MassReservoir::AtmosphericVapor => &mut self.atmospheric_vapor,
            MassReservoir::LostToSpace => &mut self.lost_to_space,
        }
    }

    /// Move `amount` of mass from one reservoir to another, the ONE conserving primitive every driver's
    /// fate-change goes through: it subtracts from `from` exactly what it adds to `to`, so the total across the
    /// four reservoirs is unchanged (conservation by construction). A negative or zero `amount` is a no-op (a
    /// driver reports a non-negative fate change). A transfer larger than the `from` balance is refused
    /// fail-loud ([`InsufficientMass`]) rather than clamped, so no account is driven negative and no mass is
    /// silently created. A same-reservoir transfer is a no-op.
    pub fn transfer(
        &mut self,
        from: MassReservoir,
        to: MassReservoir,
        amount: Fixed,
    ) -> Result<(), InsufficientMass> {
        if amount <= Fixed::ZERO || from == to {
            return Ok(());
        }
        if self.balance(from) < amount {
            return Err(InsufficientMass);
        }
        *self.balance_mut(from) = self.balance(from) - amount;
        *self.balance_mut(to) = self.balance(to) + amount;
        Ok(())
    }

    /// The total mass across all four reservoirs, the CONSERVED quantity the substrate declares as its projection
    /// (design Part 58). Invariant under [`Self::transfer`]; a change in it is a boundary flow (the interior lane
    /// delivering new crust, or a seed), never a transport-driver artifact.
    pub fn total(&self) -> Fixed {
        self.column_solid + self.dissolved_load + self.atmospheric_vapor + self.lost_to_space
    }

    /// Whether the budget still holds its `opening_total`, the local conservation check: every transfer conserves
    /// the total, so a mismatch is a real leak (a driver that created or destroyed mass off the ledger), never a
    /// rounding artifact, because `Fixed` addition is exact.
    pub fn is_conserved(&self, opening_total: Fixed) -> bool {
        self.total() == opening_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transfer_conserves_the_total_across_the_four_reservoirs() {
        // The load-bearing invariant: moving mass between fates leaves the total unchanged, so the budget closes.
        let mut b = SurfaceMassBudget::seeded(Fixed::from_int(1000));
        let opening = b.total();
        // Dissolution: solid -> dissolved. Vaporization: solid -> vapor. Escape: vapor -> lost.
        b.transfer(
            MassReservoir::ColumnSolid,
            MassReservoir::DissolvedLoad,
            Fixed::from_int(120),
        )
        .expect("solid has the mass");
        b.transfer(
            MassReservoir::ColumnSolid,
            MassReservoir::AtmosphericVapor,
            Fixed::from_int(80),
        )
        .expect("solid has the mass");
        b.transfer(
            MassReservoir::AtmosphericVapor,
            MassReservoir::LostToSpace,
            Fixed::from_int(30),
        )
        .expect("vapor has the mass");
        assert!(
            b.is_conserved(opening),
            "every transfer conserves the total"
        );
        assert_eq!(b.total(), Fixed::from_int(1000));
        // The mass landed in the right fates.
        assert_eq!(b.balance(MassReservoir::ColumnSolid), Fixed::from_int(800));
        assert_eq!(
            b.balance(MassReservoir::DissolvedLoad),
            Fixed::from_int(120)
        );
        assert_eq!(
            b.balance(MassReservoir::AtmosphericVapor),
            Fixed::from_int(50)
        );
        assert_eq!(b.balance(MassReservoir::LostToSpace), Fixed::from_int(30));
    }

    #[test]
    fn precipitation_and_redeposition_return_mass_to_the_solid_column() {
        // The reverse moves close the loop: dissolved load precipitates back, vapor redeposits back.
        let mut b = SurfaceMassBudget::seeded(Fixed::from_int(500));
        let opening = b.total();
        b.transfer(
            MassReservoir::ColumnSolid,
            MassReservoir::DissolvedLoad,
            Fixed::from_int(200),
        )
        .expect("solid mass");
        b.transfer(
            MassReservoir::DissolvedLoad,
            MassReservoir::ColumnSolid,
            Fixed::from_int(150),
        )
        .expect("dissolved mass precipitates");
        assert_eq!(b.balance(MassReservoir::ColumnSolid), Fixed::from_int(450));
        assert_eq!(b.balance(MassReservoir::DissolvedLoad), Fixed::from_int(50));
        assert!(b.is_conserved(opening));
    }

    #[test]
    fn a_transfer_larger_than_the_source_is_refused_fail_loud() {
        // Refused rather than clamped: an account is never driven negative and mass is never silently created.
        let mut b = SurfaceMassBudget::seeded(Fixed::from_int(100));
        let opening = b.total();
        assert_eq!(
            b.transfer(
                MassReservoir::ColumnSolid,
                MassReservoir::DissolvedLoad,
                Fixed::from_int(150),
            ),
            Err(InsufficientMass),
            "cannot dissolve more solid than the column holds"
        );
        // The refused move left the budget untouched and conserved.
        assert_eq!(b.balance(MassReservoir::ColumnSolid), Fixed::from_int(100));
        assert!(b.is_conserved(opening));
    }

    #[test]
    fn loss_to_space_is_terminal_so_the_world_loses_that_mass_but_the_budget_still_closes() {
        // Loss to space is the only true boundary sink: the mass is gone from the world, yet the four-account
        // budget still totals its opening (the lost mass is accounted in the LostToSpace reservoir, not leaked).
        let mut b = SurfaceMassBudget::seeded(Fixed::from_int(300));
        let opening = b.total();
        b.transfer(
            MassReservoir::ColumnSolid,
            MassReservoir::AtmosphericVapor,
            Fixed::from_int(90),
        )
        .expect("solid mass");
        b.transfer(
            MassReservoir::AtmosphericVapor,
            MassReservoir::LostToSpace,
            Fixed::from_int(90),
        )
        .expect("vapor escapes");
        assert_eq!(b.balance(MassReservoir::LostToSpace), Fixed::from_int(90));
        assert!(
            b.is_conserved(opening),
            "escaped mass is accounted, not leaked"
        );
    }

    #[test]
    fn a_zero_or_same_reservoir_transfer_is_a_no_op() {
        let mut b = SurfaceMassBudget::seeded(Fixed::from_int(10));
        let opening = b.total();
        b.transfer(
            MassReservoir::ColumnSolid,
            MassReservoir::DissolvedLoad,
            Fixed::ZERO,
        )
        .expect("zero is a no-op");
        b.transfer(
            MassReservoir::ColumnSolid,
            MassReservoir::ColumnSolid,
            Fixed::from_int(5),
        )
        .expect("same reservoir is a no-op");
        assert_eq!(b.balance(MassReservoir::ColumnSolid), Fixed::from_int(10));
        assert!(b.is_conserved(opening));
    }

    #[test]
    fn an_empty_budget_is_the_byte_neutral_default() {
        let b = SurfaceMassBudget::new();
        assert_eq!(b.total(), Fixed::ZERO);
        assert!(b.is_conserved(Fixed::ZERO));
    }
}
