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
//! its primitive, and the conservation reservoirs its mass touches). This module holds the substrate: the
//! CONSERVATION LEDGER the whole budget closes against ([`SurfaceMassBudget`]), the driver-row CONTRACT
//! ([`DriverRow`], [`DriverRegistry`], [`TransportKernelId`]) that makes the driver membership data over a fixed
//! kernel vocabulary, and the SNAPSHOT-APPLY reconciliation ([`apportion`], [`reconcile_column`]) that lets many
//! writers change a column within one tick deterministically and conservatively.
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

use std::collections::BTreeMap;

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

/// The fixed TRANSPORT-KERNEL vocabulary the data-defined driver rows compose over. This enum is the honest
/// EXTENSIBILITY BOUNDARY of the substrate, the point the design smoke test warned a data-defined registry can
/// smuggle a closed set one level down: a driver reading a new PROPERTY key or tuning a new PARAMETER is a data
/// row (the membership grows with the world), but a driver needing a kernel not here is a FLOOR EXTENSION (a new
/// arm, a deliberate Rust change), named plainly, not unbounded data. The build-now four kernels close the
/// continuous mass budget; the deferred kernels (a non-local ballistic redistribution for impact and granular
/// mass flows, a phase-change transport for volatiles) are added as arms when their primitive lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportKernelId {
    /// Gravity-driven downslope diffusion (hillslope creep and threshold failure), relaxed by `fixed_cap_solve`.
    /// It sets the slope the fluid-shear and solid-solvent kernels read.
    HillslopeDiffusion,
    /// Fluid-shear entrainment and transport capacity in the exact-root form (`E = K * sqrt(A) * S`), the flow
    /// routed by `priority_flood`. Keyed on the fluid property key-set, so a liquid solvent and a gas are the
    /// same kernel with different property data.
    FluidShear,
    /// Thermal-chemical alteration: dissolution moving mass into the dissolved-load reservoir, and thermal or
    /// frost fracturing producing the mobile grains the transport kernels move.
    ThermalChemicalAlteration,
    /// Deposition, the settling of transported load where transport capacity drops, the conservation sink that
    /// closes the column-to-column half of the budget.
    Deposition,
}

/// A DRIVER ROW: the data record binding one [`TransportKernelId`] from the fixed vocabulary to its reserved
/// parameters (keyed by name, surfaced-with-basis, never fabricated), its OPEN forcing PROPERTY key-set (the
/// named world-property keys the kernel reads, extensible so an alien driver keyed on a triboelectric charge or
/// a Bingham yield stress is a data row, not a rewrite), and the [`MassReservoir`] fates its mass touches. The
/// driver MEMBERSHIP is data (a [`DriverRegistry`] of these rows grows with the world); the kernel vocabulary
/// and the four reservoirs are the fixed floor. This mirrors the decompose-driver pattern (a data binding of a
/// fixed kernel id to reserved params plus a world-declared axis set), the sibling data-defined registry.
#[derive(Clone, Debug, PartialEq)]
pub struct DriverRow {
    /// The driver's name, its key in the registry.
    pub name: String,
    /// The fixed-vocabulary kernel this row invokes.
    pub kernel: TransportKernelId,
    /// The OPEN forcing property key-set: the named world-property keys the kernel reads (density, viscosity,
    /// surface tension, latent heat, boiling point, a saturation curve, a chemical aggressiveness, or an
    /// off-list key like a triboelectric charge). Extensible by naming a new key, the data half of the
    /// extensibility line. It is the ARMING-step data contract: the byte-neutral driver kernels
    /// ([`crate::surface_drivers`]) currently take their forcing as flat typed arguments (off the run path), and
    /// when a genesis pass arms a driver the arming step reads these keys from the world's property data and
    /// passes the resolved values as those arguments. So the open key-set records WHICH world properties a driver
    /// depends on (the alien-generality contract) ahead of the kernel wiring that consumes them; until a driver is
    /// armed it is the declared dependency, not yet a live read.
    property_keys: Vec<String>,
    /// The kernel's reserved parameters, keyed by name. An absent parameter reads zero (the substrate absence
    /// convention, matching the decompose driver). On the run path each is loaded fail-loud from the calibration
    /// manifest, surfaced-with-basis, never fabricated.
    params: BTreeMap<String, Fixed>,
    /// The [`MassReservoir`] fates this driver's mass touches, so the conservation ledger knows its reservoir
    /// footprint. A transport kernel that redistributes within the solid column names only `ColumnSolid`; a
    /// dissolution kernel names `ColumnSolid` and `DissolvedLoad`.
    reservoirs: Vec<MassReservoir>,
}

impl DriverRow {
    /// Build a driver row from its kernel, its property key-set, its reserved parameters, and the reservoir
    /// fates it touches. The parameter and property membership is data.
    pub fn new(
        name: impl Into<String>,
        kernel: TransportKernelId,
        property_keys: Vec<String>,
        params: BTreeMap<String, Fixed>,
        reservoirs: Vec<MassReservoir>,
    ) -> DriverRow {
        DriverRow {
            name: name.into(),
            kernel,
            property_keys,
            params,
            reservoirs,
        }
    }

    /// A reserved parameter by name; an absent one reads zero (the substrate absence convention).
    pub fn param(&self, name: &str) -> Fixed {
        self.params.get(name).copied().unwrap_or(Fixed::ZERO)
    }

    /// Whether the kernel reads a named world-property key.
    pub fn reads_property(&self, key: &str) -> bool {
        self.property_keys.iter().any(|k| k == key)
    }

    /// The forcing property key-set the kernel reads.
    pub fn property_keys(&self) -> &[String] {
        &self.property_keys
    }

    /// Whether this driver's mass touches a reservoir fate.
    pub fn touches(&self, reservoir: MassReservoir) -> bool {
        self.reservoirs.contains(&reservoir)
    }

    /// The reservoir fates this driver's mass touches.
    pub fn reservoirs(&self) -> &[MassReservoir] {
        &self.reservoirs
    }
}

/// The DRIVER REGISTRY: the data-defined, extensible membership of the surface-mass-transport substrate. It
/// holds the [`DriverRow`]s a world declares in registration order, the ONE canonical walk (a name lookup is a
/// convenience, never the walk), so a folded read over the drivers is reproducible and thread-invariant when a
/// genesis pass arms it (Principle 3). Empty by default and off the run path, so declaring it is byte-neutral;
/// the transport-and-deposition solve over the rows is the fixed Rust, the membership is data.
#[derive(Clone, Debug, Default)]
pub struct DriverRegistry {
    rows: Vec<DriverRow>,
}

impl DriverRegistry {
    /// An empty registry: no driver declared, the opt-out state a scenario that arms no transport stays in.
    pub fn new() -> DriverRegistry {
        DriverRegistry::default()
    }

    /// Register a driver row, appended in registration order (the canonical walk order).
    pub fn register(&mut self, row: DriverRow) {
        self.rows.push(row);
    }

    /// Whether no driver is registered (the byte-neutral opt-out state).
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The number of registered drivers.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Walk the registered drivers in registration order, the ONE canonical walk.
    pub fn iter(&self) -> impl Iterator<Item = &DriverRow> {
        self.rows.iter()
    }

    /// A driver by name (a convenience lookup, never the canonical walk); the first match, or none.
    pub fn get(&self, name: &str) -> Option<&DriverRow> {
        self.rows.iter().find(|r| r.name == name)
    }
}

/// Apportion an available integer amount among a set of non-negative demands by the exact-integer
/// largest-remainder method (Hamilton's method), the deterministic core of the snapshot-apply reconciliation.
/// When several writers contest a column whose available mass is less than their total demand, the available is
/// split so each writer's claim is exact and order-independent. If the demands sum to at most `available`, each
/// is met in full. Otherwise each demander gets `floor(available * demand_i / total_demand)`, and the leftover
/// (`available` minus the sum of those floors) is handed out one unit at a time to the demanders with the
/// largest fractional remainders (`available * demand_i mod total_demand`), TIES BROKEN BY THE LOWER INDEX, so
/// the result is a pure function of the inputs (Principle 3, Principle 10). The allocations sum to
/// `min(total_demand, available)` exactly (conservative: no unit is created or lost) and each allocation is at
/// most its demand. It works in the raw integer mass domain (a `Fixed` mass is passed as its bits) so the split
/// is exact; `i128` intermediates hold `available * demand` without overflow. A negative `available` or demand
/// reads as zero.
pub fn apportion(available: i64, demands: &[i64]) -> Vec<i64> {
    let avail = i128::from(available.max(0));
    let clamped: Vec<i128> = demands.iter().map(|&d| i128::from(d.max(0))).collect();
    let total: i128 = clamped.iter().sum();
    if total <= avail {
        // Every demand met in full (the clamped non-negative demand).
        return clamped.iter().map(|&d| d as i64).collect();
    }
    if avail == 0 {
        return vec![0; demands.len()];
    }
    // total > avail > 0: the largest-remainder split.
    let mut alloc: Vec<i64> = Vec::with_capacity(clamped.len());
    let mut remainders: Vec<(i128, usize)> = Vec::with_capacity(clamped.len());
    let mut floor_sum: i128 = 0;
    for (i, &d) in clamped.iter().enumerate() {
        let num = avail * d;
        alloc.push((num / total) as i64);
        floor_sum += num / total;
        remainders.push((num % total, i));
    }
    // The leftover lies in `[0, clamped.len())`: each floor is within one unit of the exact share.
    let mut leftover = avail - floor_sum;
    // Largest remainder first, ties by the lower index: a total order, so no non-deterministic tie.
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, i) in &remainders {
        if leftover <= 0 {
            break;
        }
        alloc[i] += 1;
        leftover -= 1;
    }
    alloc
}

/// Reconcile one column's writer demands under the SNAPSHOT-APPLY discipline (raw integer mass units; a `Fixed`
/// mass is passed as its bits, the `Fixed` and [`crate::material::EarthworkField`] wiring is the arming step).
/// Each writer's `signed_demand` is computed against the tick SNAPSHOT (`available`, the column's mass at tick
/// start), never against a value another writer already moved this tick, so the apply is order-independent.
/// Additions (a positive demand, deposition or uplift) apply in full. Removals (a negative demand, erosion or
/// subsidence) are limited by the snapshot available mass: if the total removal exceeds it, the available is
/// APPORTIONED among the removing writers by [`apportion`], so no writer's honored removal exceeds its fair
/// share and the column never drops below zero. Returns each writer's APPLIED signed delta (in the input order,
/// because each writer routes its honored mass to its own reservoir fate) and the column's NET change. The
/// honored removals sum to `min(total_removal, available)` exactly, so the reconciliation is conservative and
/// deterministic. The net is accumulated in `i128` and saturated into `i64` so an extreme demand set cannot
/// overflow.
pub fn reconcile_column(available: i64, signed_demands: &[i64]) -> (Vec<i64>, i64) {
    // The removal magnitude each writer asks for (zero for an adding writer), apportioned against the snapshot.
    let removals: Vec<i64> = signed_demands
        .iter()
        .map(|&d| if d < 0 { d.saturating_neg() } else { 0 })
        .collect();
    let honored = apportion(available, &removals);
    let mut applied = Vec::with_capacity(signed_demands.len());
    let mut net: i128 = 0;
    for (i, &d) in signed_demands.iter().enumerate() {
        let a = if d >= 0 { d } else { -honored[i] };
        applied.push(a);
        net += i128::from(a);
    }
    let net = net.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    (applied, net)
}

/// The reconcile-honored COMPOSITION CONTRACT, the one form both the surface arming step and the redistribution
/// lane honor: reconcile a contested column's removal demands against the tick snapshot FIRST, then key every
/// downstream sink write off the HONORED removal, never the raw demand. The seam this closes (surfaced by the
/// section-9 panel on the surface lane and independently by the redistribution lane) is that a source driver bounds
/// its removal by a local rule (the slope drop) and would route that FULL demanded mass to its sink, while
/// [`reconcile_column`] clamps a contested column's honored removal to the snapshot mass it holds, so the sink
/// would gain more than the column lost and the ledger would fabricate the difference.
///
/// `honored_removals` returns, for one column, each source writer's HONORED removal: the mass it both removes from
/// the column AND routes to its sink. It reconciles the removal demands against the `available` snapshot (through
/// [`reconcile_column`], so the split is the exact-integer largest-remainder apportionment, order-independent) and
/// returns the honored magnitude per writer. The load-bearing invariant: the honored removal a writer routes to
/// its sink equals the honored removal it takes from the column, so under contention the source lowering and the
/// sink gain are the same mass and no mass is fabricated at the reconcile seam. The honored removals sum to
/// `min(total_demand, available)` exactly.
///
/// The surface source writers are single-sink (fluid-shear entrains to deposition, dissolution moves to the
/// dissolved-load reservoir), so a writer routes its whole honored removal to its one sink. A writer that FANS its
/// honored removal across several sinks or destinations (the redistribution lane) splits the SAME honored removal
/// with [`apportion`], so the destinations sum to exactly it; the single-sink case is that split with one weight.
/// A fixed-point fraction-multiply is never used to rescale a raw sink by an honored FRACTION: it would round and
/// the sinks would not sum to the honored removal, reopening the leak at the rounding scale.
pub fn honored_removals(available: i64, removal_demands: &[i64]) -> Vec<i64> {
    let signed: Vec<i64> = removal_demands.iter().map(|&d| -d.max(0)).collect();
    let (applied, _net) = reconcile_column(available, &signed);
    // Each removing writer's applied delta is negative; the honored removal magnitude is its negation.
    applied.iter().map(|&a| a.saturating_neg()).collect()
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

    fn params(pairs: &[(&str, i32)]) -> BTreeMap<String, Fixed> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Fixed::from_int(*v)))
            .collect()
    }

    #[test]
    fn a_driver_row_binds_a_kernel_to_named_params_and_an_open_property_key_set() {
        // A driver is a data row: a fixed-vocabulary kernel plus reserved params by name plus the open forcing
        // property keys it reads. An absent param reads zero (the substrate absence convention).
        let row = DriverRow::new(
            "fluvial-water",
            TransportKernelId::FluidShear,
            vec!["density".into(), "viscosity".into()],
            params(&[("erodibility", 3), ("capacity_coefficient", 7)]),
            vec![MassReservoir::ColumnSolid],
        );
        assert_eq!(row.kernel, TransportKernelId::FluidShear);
        assert_eq!(row.param("erodibility"), Fixed::from_int(3));
        assert_eq!(row.param("capacity_coefficient"), Fixed::from_int(7));
        assert_eq!(
            row.param("absent"),
            Fixed::ZERO,
            "an absent param reads zero"
        );
        assert!(row.reads_property("density") && row.reads_property("viscosity"));
        assert!(row.touches(MassReservoir::ColumnSolid));
        assert!(!row.touches(MassReservoir::DissolvedLoad));
    }

    #[test]
    fn an_alien_driver_reads_an_off_list_property_key_as_a_data_row() {
        // The open property key-set admits the alien: a driver keyed on a triboelectric charge (electrostatic
        // dust transport) or a Bingham yield stress (a mud or lava mass flow) is a data row, not a rewrite. The
        // key is any name; the kernel vocabulary is the only fixed set.
        let dust = DriverRow::new(
            "electrostatic-dust",
            TransportKernelId::FluidShear,
            vec!["triboelectric_charge".into(), "grain_size".into()],
            params(&[("mobility", 1)]),
            vec![MassReservoir::ColumnSolid],
        );
        assert!(
            dust.reads_property("triboelectric_charge"),
            "an off-list property key is a data row"
        );
        assert_eq!(dust.property_keys().len(), 2);
    }

    #[test]
    fn a_dissolution_driver_declares_the_dissolved_load_reservoir_footprint() {
        // A thermal-chemical dissolution driver moves mass from the solid column into the dissolved-load
        // reservoir, so it declares both fates; the conservation ledger reads this footprint.
        let row = DriverRow::new(
            "carbonate-dissolution",
            TransportKernelId::ThermalChemicalAlteration,
            vec!["chemical_aggressiveness".into()],
            params(&[("dissolution_rate", 2)]),
            vec![MassReservoir::ColumnSolid, MassReservoir::DissolvedLoad],
        );
        assert!(row.touches(MassReservoir::ColumnSolid));
        assert!(row.touches(MassReservoir::DissolvedLoad));
        assert!(!row.touches(MassReservoir::LostToSpace));
    }

    #[test]
    fn the_registry_walks_the_drivers_in_registration_order_and_is_empty_by_default() {
        let mut reg = DriverRegistry::new();
        assert!(reg.is_empty(), "the byte-neutral opt-out default");
        reg.register(DriverRow::new(
            "hillslope",
            TransportKernelId::HillslopeDiffusion,
            vec!["slope".into()],
            params(&[("diffusivity", 1)]),
            vec![MassReservoir::ColumnSolid],
        ));
        reg.register(DriverRow::new(
            "deposition",
            TransportKernelId::Deposition,
            vec!["grain_size".into()],
            BTreeMap::new(),
            vec![MassReservoir::ColumnSolid],
        ));
        assert_eq!(reg.len(), 2);
        let names: Vec<&str> = reg.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["hillslope", "deposition"],
            "the canonical walk is registration order"
        );
        assert_eq!(
            reg.get("deposition").map(|r| r.kernel),
            Some(TransportKernelId::Deposition),
            "a name lookup is a convenience over the walk"
        );
        assert!(reg.get("absent").is_none());
    }

    #[test]
    fn an_uncontested_apportionment_meets_every_demand_in_full() {
        // Total demand at or below the available: each demander gets exactly what it asked.
        assert_eq!(apportion(10, &[3, 3, 3]), vec![3, 3, 3]);
        assert_eq!(
            apportion(9, &[3, 3, 3]),
            vec![3, 3, 3],
            "sum equal to available"
        );
        assert_eq!(apportion(100, &[1, 0, 5]), vec![1, 0, 5]);
    }

    #[test]
    fn a_contested_apportionment_splits_by_largest_remainder_conservatively() {
        // total 9 > available 7: floor(7*3/9) = 2 each (sum 6), leftover 1 to the largest remainder; the three
        // remainders are equal (7*3 mod 9 = 3 each), so the tie breaks to the lowest index.
        let a = apportion(7, &[3, 3, 3]);
        assert_eq!(a, vec![3, 2, 2], "leftover unit to index 0 on the tie");
        assert_eq!(
            a.iter().sum::<i64>(),
            7,
            "the split sums to the available exactly"
        );
        // No allocation exceeds its demand.
        for (alloc, demand) in a.iter().zip([3, 3, 3]) {
            assert!(*alloc <= demand);
        }
    }

    #[test]
    fn the_apportionment_is_conservative_and_bounded_on_an_uneven_contest() {
        // A larger demander takes a larger share, the split still sums to the available, none over-allocated.
        let demands = [10, 3, 1];
        let a = apportion(7, &demands);
        assert_eq!(a.iter().sum::<i64>(), 7, "sums to the available");
        for (alloc, demand) in a.iter().zip(demands) {
            assert!(*alloc <= demand, "no writer over its demand");
            assert!(*alloc >= 0);
        }
        assert!(
            a[0] >= a[1] && a[1] >= a[2],
            "a larger demand takes no smaller a share"
        );
    }

    #[test]
    fn the_apportionment_is_a_pure_function_of_its_inputs() {
        // Deterministic and order-independent: the same inputs give a bit-identical split (Principle 3, 10).
        let demands = [5, 2, 8, 1, 4];
        assert_eq!(apportion(11, &demands), apportion(11, &demands));
        // Zero available yields all zero; negative inputs read as zero.
        assert_eq!(apportion(0, &[3, 4]), vec![0, 0]);
        assert_eq!(apportion(-5, &[3, 4]), vec![0, 0]);
        assert_eq!(
            apportion(10, &[-3, 4]),
            vec![0, 4],
            "a negative demand reads as zero"
        );
    }

    #[test]
    fn reconcile_applies_additions_in_full_and_removals_within_the_snapshot() {
        // A column with 100 available; two writers add, one removes within the available: all honored in full.
        let (applied, net) = reconcile_column(100, &[20, -30, 10]);
        assert_eq!(applied, vec![20, -30, 10]);
        assert_eq!(net, 0, "20 + 10 added, 30 removed");
    }

    #[test]
    fn reconcile_apportions_removals_that_exceed_the_snapshot_so_the_column_never_goes_negative() {
        // Two erosion writers demand 60 and 60 (total 120) from a column with only 80 available: the 80 is
        // apportioned (40 each here, equal demands), so the honored removal is exactly the available, and a
        // simultaneous addition of 10 applies in full.
        let (applied, net) = reconcile_column(80, &[-60, -60, 10]);
        assert_eq!(
            applied,
            vec![-40, -40, 10],
            "the 80 available split evenly between the removers"
        );
        assert_eq!(net, -70, "10 added, 80 removed");
        // The removals never exceed the snapshot available (the column does not go below zero).
        let removed: i64 = applied.iter().filter(|&&a| a < 0).map(|&a| -a).sum();
        assert_eq!(
            removed, 80,
            "honored removal equals the available, not the 120 demanded"
        );
    }

    #[test]
    fn reconcile_is_a_no_op_on_no_writers_or_zero_demands() {
        assert_eq!(reconcile_column(50, &[]), (vec![], 0));
        assert_eq!(reconcile_column(50, &[0, 0]), (vec![0, 0], 0));
    }

    #[test]
    fn honored_removals_sum_to_the_column_removal_so_the_composition_conserves() {
        // The reconcile-honored contract: two erosion writers demand 60 and 60 from a column holding 80. The
        // honored removals are apportioned (40 each), and their SUM is exactly the mass the column loses, so the
        // sinks that receive the honored removals gain exactly what the column lost. Routing the raw demand (120)
        // to the sinks instead would deposit 120 while the column lost only 80, fabricating 40 at the seam.
        let demands = [60, 60];
        let honored = honored_removals(80, &demands);
        assert_eq!(
            honored,
            vec![40, 40],
            "the honored removals are apportioned"
        );
        let sink_total: i64 = honored.iter().sum();
        // The column removal is what reconcile_column applies (the negated applied deltas).
        let (applied, _net) = reconcile_column(80, &[-60, -60]);
        let column_removal: i64 = applied.iter().map(|&a| -a).sum();
        assert_eq!(
            sink_total, column_removal,
            "the sinks gain exactly what the column loses"
        );
        assert_eq!(
            sink_total, 80,
            "the honored total is the snapshot available, not the 120 demanded"
        );
        assert!(
            demands.iter().sum::<i64>() > sink_total,
            "routing the raw demand would fabricate the difference (the seam the contract closes)"
        );
    }

    #[test]
    fn an_uncontested_column_honors_every_removal_in_full() {
        // When the demands fit the snapshot, each writer's honored removal equals its demand (no apportionment).
        assert_eq!(honored_removals(100, &[20, 30]), vec![20, 30]);
        assert_eq!(honored_removals(50, &[50, 0]), vec![50, 0]);
    }

    #[test]
    fn honored_removals_agree_with_the_reconcile_applied_deltas() {
        // The honored removal magnitude is exactly the negation of reconcile_column's applied delta for each
        // writer, so the composition helper cannot drift from the reconciliation it is built on.
        let demands = [10, 3, 1];
        let honored = honored_removals(7, &demands);
        let signed: Vec<i64> = demands.iter().map(|&d| -d).collect();
        let (applied, _net) = reconcile_column(7, &signed);
        let from_applied: Vec<i64> = applied.iter().map(|&a| -a).collect();
        assert_eq!(honored, from_applied);
        assert_eq!(
            honored.iter().sum::<i64>(),
            7,
            "conservative against the snapshot"
        );
    }

    #[test]
    fn the_honored_removal_source_conserves_through_deposition() {
        // The capstone the suite lacked: the honored removal fed as the deposition SOURCE conserves end to end.
        // The two writers' honored removals (40 each on a contested column of 80) route to two cells that drain to
        // an outlet; deposition settles the whole honored load, so total deposited equals the honored removal
        // total equals the column removal, with no fabricated mass at the composition seam.
        let honored = honored_removals(80, &[60, 60]); // [40, 40]
        let entrained: Vec<Fixed> = honored.iter().map(|&h| Fixed::from_int(h as i32)).collect();
        let entrained = [entrained[0], entrained[1], Fixed::ZERO]; // two sources draining to cell 2 (outlet)
        let receiver = [2usize, 2, 2];
        let capacity = [Fixed::from_int(1000); 3];
        let pass =
            crate::surface_drivers::deposit(&entrained, &receiver, &capacity).expect("valid");
        let deposited: Fixed = pass.deposited.iter().fold(Fixed::ZERO, |a, &v| a + v);
        let honored_total = Fixed::from_int(honored.iter().sum::<i64>() as i32);
        assert_eq!(
            deposited, honored_total,
            "deposited equals the honored removal, conserved"
        );
        assert_eq!(
            honored_total,
            Fixed::from_int(80),
            "the column removal, not the 120 demanded"
        );
    }

    #[test]
    fn honored_removals_is_a_pure_function() {
        assert_eq!(
            honored_removals(11, &[5, 2, 8, 1]),
            honored_removals(11, &[5, 2, 8, 1])
        );
        assert_eq!(honored_removals(0, &[3, 4]), vec![0, 0]);
    }
}
