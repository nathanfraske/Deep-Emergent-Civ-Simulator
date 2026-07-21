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

//! THE ABIOTIC DEEP-TIME RUN (seam 4, the wiring): the derived planet EVOLVES over geological time. The ruling was that
//! the physics is already built and dormant (the interior convection, the melt rung, the impact chain, the star
//! aging); seam 4 steps it on a clock so the surface CHANGES and the provinces write themselves onto that clock,
//! never an authored map. This module is the run state and the tick; the viewer's time control drives it and the
//! surface re-derives and redraws each step.
//!
//! Slice 1 (this): the INTERIOR THERMAL EVOLUTION. A lateral field of mantle columns steps its convection
//! ([`convection_step_si`]) each tick, so the interior cools and convects over deep time. Lateral variation EMERGES
//! rather than being painted: a column with more radiogenic heat or a hotter start relaxes to a different thermal
//! state than its neighbour, so the provinces are the written record of each column's own history. Nothing is
//! authored: the columns start uniform (a fresh planet has no thermal history yet) and diverge only by their own
//! parameters and the convection law.
//!
//! The later slices wire onto this same clock: the melt-driven VOLCANISM (the interior temperature crossing the
//! seam-6 solidus extracts melt that emplaces crust, thickening it where the mantle is hot, which retires the
//! authored crustal-thickness fixture as the thickness becomes the accumulated melt), the IMPACT CHAIN (the
//! accretion-tail flux draws impacts that crater and blanket the surface), and the STAR AGING (the main-sequence
//! luminosity climbing over the star's life warms the surface). Each is a step term added here; slice 1 is the
//! interior spine they hang on.
//!
//! The STAR-AGING slice (this): the star ages on the SAME deep-time clock and BRIGHTENS. A main-sequence star's
//! core hydrogen fuses to helium, the core's mean molecular weight rises, so the core contracts and heats and the
//! luminosity CLIMBS over the star's life. The brightening is DERIVED, never an authored curve: the star's
//! main-sequence lifetime `t_ms` is the nuclear timescale (fuel over burn rate, `t_sun * (M/M_sun) / (L_zams/L_sun)`)
//! from the star's mass and its zero-age luminosity, which the stellar front-end ([`crate::stellar`]) already
//! gives, and the current luminosity is the ZAMS anchor climbed by the Gough factor `1 / (1 - c * t/t_ms)`. The
//! surface warmth reads [`DeepTimeState::stellar_luminosity_ratio`] each step and warms as it climbs. Past `t_ms`
//! the star LEAVES the main sequence (the post-main-sequence giant branch is a separate stellar-evolution arc), so
//! the brightening caps there rather than extrapolating a regime this law does not describe.
//!
//! THE BOMBARDMENT slice (this): the impact chain wires onto the same clock. Each tick, after the interior
//! steps, [`bombard_tick`] draws this tick's impacts from the accretion-tail flux ([`civsim_world::impact_flux`])
//! and, for each, derives the crater the crater-scaling law opens. Discrete craters are RECORDED AS ROWS, never
//! rasters: each drawn impact emits a [`CraterRow`] (its position, its true derived rim diameter and bowl depth,
//! its age) into [`DeepTimeState::craters`], the individual the renderer stamps analytically at the viewport's
//! own resolution (so a sub-cell crater is a discrete object at its real size, not smeared across a whole coarse
//! convective cell). The CROSS-SCALE WRITE RULE keeps the large-basin feedback: a crater whose rim diameter is at
//! or above the convective cell size (the province grid spacing) ALSO rasterizes into the province relief field
//! ([`DeepTimeState::impact_relief_m`]) through the already-built conservative [`apply_impact`] (the basin that
//! resurfaces a province, its thermal/province feedback), while a sub-cell crater writes a ROW only. A process
//! writes into a field only at or above that field's derived scale. The flux DECLINES with epoch, so early epochs
//! are heavily cratered and late ones quiescent (the honest late-heavy-bombardment history, never an authored
//! curve); the impact sizes are DRAWN from the collisional-cascade size-frequency distribution (large basins
//! rare, small craters common), never a fixed size; the large-basin raster conserves mass exactly (the excavated
//! bowl equals the deposited blanket). The draw is a deterministic seeded draw keyed on the world identity and the
//! tick (the splitmix64 counter stream, never floating randomness), bounded by a per-tick cap. It is a term
//! SEPARATE from [`step_deep_time`] so a run that does not bombard replays bit-for-bit.
//!
//! THE SUPPORT-BOUND COLLAPSE slice (this): the mechanical-strength BOUND on topography. The volcanic crust and
//! the impact record compose into a surface relief the Airy isostasy floats ([`airy_isostatic_elevation`]), and a
//! crustal column can hold only so much topography above the surrounding datum before its own weight overcomes
//! its deviatoric strength and it FLOWS (lower-crustal flow / gravitational collapse). That ceiling is the
//! support bound `sigma_y / (rho * g)`, the yield strength over the buoyant gravitational load, and relief taller
//! than it is unphysical. [`relax_to_support_bound`] models the collapse: each tick, after the interior steps, it
//! projects each column's isostatic relief, caps it at the derived support bound, and FLOWS the excess crust to
//! the columns below the cap (the accommodation space) through the built conservative redistribution
//! ([`civsim_world::redistribute`]), so total crust mass is conserved to the bit and the relief relaxes to the
//! bound. The yield strength is DERIVED, never the reserved 1e8 Pa literal: it reads the crust's OWN operative
//! shear strength from the mechanical floor (the Frenkel ideal scaled by the per-class knockdown,
//! [`civsim_materials::properties::operative_shear_strength_gpa`]), the derive-down that retires the authored
//! bound on this path. WHAT IS DERIVED HERE AND WHAT IS NOT: the CAP is derived, and the COLLAPSE LAW is an
//! authored limiting idealization standing in for transport physics this module does not carry. The whole
//! over-bound excess moves within one tick, so there is no flow RATE; the excess reaches every under-cap column
//! in the field whatever its distance, so there is no TRANSPORT PATH; and nothing is dissipated and no damage
//! state accumulates, so a column that has collapsed is indistinguishable from one that never did. The absence
//! of a rate is a GAP rather than a floor that needs none: under the Residual Law's fluctuation-dissipation
//! clause, a relaxation that dissipates with no fluctuation partner is declared INCOMPLETE. It is a term
//! SEPARATE from [`step_deep_time`], applied by the caller after it, so a run that does not collapse replays
//! bit-for-bit.
//!
//! THE MASS LEDGER slice (this): the crust is a STOCK moved by a TRANSACTION, never a thickness that appears.
//! Before this the crust only grew, toward the equilibrium its melt column supports, and that growth was an
//! equilibrium CLAMP rather than a transfer: it subtracted nothing from anything, so the "finite fusible source"
//! the saturation was named for was a description with no reservoir behind it, and crust DESTRUCTION was not
//! merely unimplemented but unrepresentable, because there were no stocks to move mass between. The state now
//! carries two AREAL-MASS stocks per lateral cell ([`DeepTimeState::crust_areal_mass`] and
//! [`DeepTimeState::source_areal_mass`]), and melt extraction DEBITS the source and CREDITS the crust by one and
//! the same fixed-point value, so their sum is invariant to the bit. The thickness the isostasy floats is the
//! crust stock's own geometric READOUT (`mass / density`), recomputed wherever the stock moves rather than
//! accumulated beside it, so the geometry cannot drift from the ledger and no second copy of the crust exists.
//! What this slice deliberately does NOT add is a foundering or recycling RATE: the isostatic sign establishes
//! that a dense column is unstable, never how much of it founders per tick, and a rate written without a source
//! would be fabricated. The ledger is the state change that lets a sourced rate land later without redesigning
//! the state again.
//!
//! Determinism (Principle 3, Principle 10): [`convection_step_si`] is a pure function and the columns are walked in
//! index order, so the tick is a pure function of the state and the parameters, worker-invariant. Dormant: no
//! scenario or viewer drives it yet (the time control is the next slice), so it is byte-neutral over the run
//! path.

use crate::geodynamics::{convection_step_si, ColumnState, SiColumnParams};
use civsim_core::{Fixed, Rng, StateHasher};
use civsim_materials::properties::operative_shear_strength_gpa;
use civsim_physics::geodynamics::airy_isostatic_elevation;
use civsim_physics::melting::adiabatic_melt_column;
use civsim_world::ballistic::{BallisticForces, EjectaFan};
use civsim_world::crater::{crater, CraterCoupling, Impactor, Target};
use civsim_world::impact_event::apply_impact;
use civsim_world::impact_flux::{size_at_number_fraction, tail_rate_fraction};
use civsim_world::redistribute::{redistribute, Redistribution, Weighted};
use civsim_world::terrain::relief_datum;

/// One DISCRETE crater the bombardment drew, recorded as a ROW (never a raster): its position on the surface, the
/// crater law's own derived rim diameter and transient bowl depth, and its age. The renderer stamps it
/// analytically at the viewport's resolution (the viewer's crater stamp), so the crater is a discrete object at
/// its true derived size down to the finest scale any zoom resolves, rather than a sub-cell feature smeared
/// across a whole coarse convective cell. The position is the drawn cell's centre as a normalized surface
/// coordinate `(u, v)` (longitude fraction in `[0, 1)`, latitude fraction in `[0, 1]`), resolution-independent so
/// a finer display grid resolves it. The diameter and depth are the crater-scaling law's outputs
/// ([`civsim_world::crater::crater`]), so the morphology conditions on the world (a low-gravity or weak-target
/// world craters differently). The age is the geological time (megayears) at the strike, carried for a later
/// age-dependent degradation (a fresh crater is sharp, an old one relaxed); this slice stamps every row fresh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraterRow {
    /// The crater centre's longitude fraction in `[0, 1)` (the drawn cell's centre).
    pub u: Fixed,
    /// The crater centre's latitude fraction in `[0, 1]` (the drawn cell's centre).
    pub v: Fixed,
    /// The transient rim diameter (metres), the crater-scaling law's own output.
    pub diameter_m: Fixed,
    /// The transient bowl depth (metres), the crater-scaling law's own output (`bowl_aspect * diameter`).
    pub depth_m: Fixed,
    /// The geological time at the strike (megayears), carried for a later age-dependent crater degradation.
    pub age_myr: Fixed,
}

/// The deep-time state of a derived planet: a lateral field of interior mantle columns (one per surface cell or
/// province), the crust each column has BUILT so far, the geological time elapsed, and the discrete crater ROWS
/// the bombardment drew. The field is the spatial extent the lateral variation lives in: each column carries its
/// own thermal state and its own accumulated crust, so provinces emerge from columns that evolve differently,
/// never an authored arrangement. The star's age rides this same clock (`star_age_start_myr + elapsed_myr`).
#[derive(Clone, Debug, PartialEq)]
pub struct DeepTimeState {
    /// The interior mantle columns, one per lateral cell, each evolving by its own convection over deep time.
    pub columns: Vec<ColumnState>,
    /// The crust each column has BUILT (kilometres), one per lateral cell, the DERIVED crustal thickness that
    /// retires the authored 30 km crustal-thickness fixture: a hot, long-lived column builds thick crust, a cold
    /// one thin, the provinces.
    ///
    /// It is the crust STOCK's geometric READOUT, never a second accumulator: it is
    /// [`DeepTimeState::crust_areal_mass`] divided by the emplaced-crust density
    /// ([`thickness_km_from_areal_mass`]), recomputed wherever the stock moves. The mass is what the ledger
    /// conserves and the thickness is what the isostasy floats, and keeping the thickness a function of the mass
    /// is what stops the two representations of one crust from drifting apart tick by tick.
    pub crust_thickness_km: Vec<Fixed>,
    /// THE CRUST STOCK: the areal mass of crust each column has emplaced, one per lateral cell, in MEGAGRAMS PER
    /// SQUARE METRE ([`areal_mass_mg_per_m2`]). A fresh planet carries none; every megagram here was DEBITED from
    /// [`DeepTimeState::source_areal_mass`] by the melt transaction, so the crust is mass that moved rather than
    /// mass that appeared.
    pub crust_areal_mass: Vec<Fixed>,
    /// THE SOURCE STOCK: the areal mass of unmelted mantle source remaining under each column, one per lateral
    /// cell, in the same megagrams per square metre. Melt extraction debits it and credits the crust by the same
    /// value, so `crust_areal_mass[i] + source_areal_mass[i]` is invariant to the bit under the transaction
    /// ([`step_deep_time`]). The initial stock is the caller's DERIVED convecting-mantle areal mass (the
    /// convecting depth times the mantle density), never an authored reservoir.
    ///
    /// The honest limit: the source is treated as uniformly fertile, with no fertile-versus-depleted split and no
    /// composition carried, so what a column has already yielded changes the stock's SIZE and not its
    /// meltability. A compositional source (the residue that melting leaves behind, feeding back into solidus and
    /// density) is the named follow-on the ledger makes reachable.
    pub source_areal_mass: Vec<Fixed>,
    /// The geological time elapsed (megayears), the clock the provinces are written against.
    pub elapsed_myr: Fixed,
    /// The accumulated LARGE-BASIN bombardment relief (metres), one per lateral cell, the province-scale
    /// topography the CROSS-SCALE WRITE RULE deposits: only a crater at or above the convective cell size (a
    /// basin that resurfaces a province) rasterizes here, through the conservative [`apply_impact`], for its
    /// thermal/province feedback. A sub-cell crater writes a [`CraterRow`] only and never touches this field, so
    /// the coarse province field is no longer smeared with sub-cell craters (rows not rasters). It is in METRES
    /// (the crater law's own length units), whereas the volcanic crust is in kilometres, so the two carry their
    /// own units. A fresh planet has none; each large-basin impact adds a conservative delta (the excavated bowl
    /// equals the deposited blanket, so an impact never changes this field's sum). By default (sub-cell craters at
    /// the ~thousand-km province scale) this field stays zero and the whole crater record lives in `craters`; a
    /// basin-scale impactor restores its own province feedback here.
    pub impact_relief_m: Vec<Fixed>,
    /// The discrete crater ROWS the bombardment drew over the run so far ([`CraterRow`]), the individual-crater
    /// record the renderer stamps analytically at the viewport's resolution (rows not rasters, down to the finest
    /// scale any zoom resolves). Each drawn impact whose crater the law resolves appends one row here, at its true
    /// derived size and position. Heavily struck early and quiescent late (the flux declines with epoch), the
    /// derived record, not an authored map.
    pub craters: Vec<CraterRow>,
    /// The number of discrete craters the bombardment has drawn over the run so far (`craters.len()`, an impact
    /// whose crater the law resolved), the bombardment-history count. Heavily struck early (the accretion tail is
    /// intense) and quiescent late (the reservoir is swept up), the honest decline the flux model drives; the
    /// derived record, not an authored intensity.
    pub impact_count: u64,
    /// THE DEFERRED-IMPACT QUEUE: whole impact events the accretion tail DELIVERED but the per-tick cap
    /// ([`ImpactFluxParams::per_tick_impact_cap`]) could not apply, WITHHELD and re-offered on the next tick.
    /// The cap is a compute-and-determinism bound rather than a physical one, so truncating the draw at it used
    /// to delete real bodies from the reservoir's budget with no ledger: a tick offering a thousand strikes
    /// against a cap of five discarded the other nine hundred and ninety-five, while the flux's analytic decline
    /// ([`tail_rate_fraction`]) went on asserting all thousand had been swept. The queue is the WITHHOLDING
    /// principle (an un-drawn event stays conserved and is offered again) rather than the largest-remainder
    /// apportionment the redistribution uses: cap truncation is a carry with one queue and one next tick, never
    /// `m` claimants contending over an indivisible quantum.
    ///
    /// The honest limit: withholding conserves the COUNT and does not preserve the arrival TIME. A body the cap
    /// defers strikes later than the flux delivered it, so a heavily-capped early bombardment is smeared into the
    /// epochs after it. The queue makes that visible and bounded where the truncation made it invisible; only a
    /// cap high enough not to bind removes it.
    pub deferred_impacts: u64,
    /// The impact events this run drew and could NOT realize: a size the reservoir's size-frequency distribution
    /// could not draw, a crater the scaling law could not solve, or a basin raster whose application would have
    /// left the representable window or failed its own zero-sum check. Each of these used to `continue`
    /// silently, so a deleted event left no trace at all and [`DeepTimeState::impact_count`] (which advances only
    /// after a strike fully lands) could not tell a quiet epoch from a broken one.
    ///
    /// These are NOT deferred, and that is why they are counted apart: a deferred event failed for CAPACITY and
    /// will succeed later, while these failed against the world's own parameters and would fail identically on
    /// every retry, so re-offering them would spin forever. They are the honest LOSS from the reservoir's budget,
    /// recorded rather than hidden, and a run that accumulates them is reporting that its impactor population and
    /// its crater law do not meet.
    pub unrealized_impacts: u64,
    /// The star's main-sequence AGE (megayears) when this run began: the star and the planet share one clock, so
    /// the star's current age is this start age plus `elapsed_myr` ([`DeepTimeState::star_age_myr`]). A fresh
    /// planet co-forming with a zero-age star begins at zero; a run beginning around an already-aged star carries
    /// the star's prior age here. This is the per-run start age the brightening is measured against, not an
    /// authored map.
    pub star_age_start_myr: Fixed,
}

impl DeepTimeState {
    /// A RECEIPT over the realized physics state, for the physics determinism baseline. Deliberately NOT
    /// called a state hash, and deliberately not reusable as one: under the Chaos Protocol (the R-ASSEMBLY
    /// ruling, `docs/working/R_ASSEMBLY_RESEARCH_QUESTION.md`) a content hash is a CONSTITUTIVE input, gate
    /// 5's seed discipline content-hashing world identity plus the embryo field to decide WHICH world is
    /// drawn. This digest is the opposite direction of causation: it reads a state that has already been
    /// realized and folds nothing back into the world. Keeping the two named apart keeps a reader from
    /// feeding a receipt into a seed slot, which would make the drawn world depend on its own outcome.
    ///
    /// What a pin on this digest MEANS, stated so it is not over-read: it pins the REALIZATION, a given seed
    /// through a given measure, never a physical trajectory. The Chaos Protocol forbids integrating the
    /// Lyapunov-sensitive stages as fixed point (a byte-neutrality landmine, and no derivation anyway, since
    /// below the Lyapunov horizon a trajectory is a hash of sub-band digits the seed stream already carries).
    /// The per-column deep-time thermal evolution folded here is the dissipative regime rather than that one:
    /// it CONVERGES, the measured state going byte-identical from tick 11 and drifting only sub-quantum
    /// after, which is the signature of relaxation toward equilibrium rather than of divergence. So it is
    /// legitimately integrated and legitimately pinned. The assembly stage is the chaotic one, and its
    /// determinism lives in the seeded draw, never here.
    ///
    /// So a moved digest carries a readable meaning, which the biology-harness pins it replaces could not
    /// give: either a derivation changed (review it), or the seed derivation changed, so a different world
    /// was drawn (a different question entirely).
    ///
    /// Every field is folded, each collection length-prefixed. The vectors are POSITIONAL, one entry per
    /// lateral cell, so they fold in index order and are never sorted: their order is the geometry, and
    /// sorting would hash a different world that happens to hold the same values.
    pub fn realization_digest(&self) -> u128 {
        let mut h = StateHasher::new();
        // The clocks the provinces are written against.
        h.write_fixed(self.elapsed_myr);
        h.write_fixed(self.star_age_start_myr);
        h.write_u64(self.impact_count);
        // The bombardment's two RESIDUAL counts. A world holding a deferred queue is mid-bombardment and owes
        // strikes; a world carrying unrealized events has a reservoir and a crater law that do not meet. Neither
        // is the same world as one whose bombardment closed cleanly at the same crater count.
        h.write_u64(self.deferred_impacts);
        h.write_u64(self.unrealized_impacts);
        // The interior, one column per lateral cell, in cell order.
        h.write_u64(self.columns.len() as u64);
        for c in &self.columns {
            h.write_fixed(c.temperature);
            // The onset latch is state: a column that has begun convecting is a different world from one
            // sitting at the same temperature that has not.
            h.write_u32(u32::from(c.convecting));
        }
        // The derived crust, the field that retires the authored thickness fixture.
        h.write_u64(self.crust_thickness_km.len() as u64);
        for t in &self.crust_thickness_km {
            h.write_fixed(*t);
        }
        // THE MASS LEDGER, both stocks, in cell order. The thickness above is the crust stock's readout, so on
        // the melt path these move together; they are folded separately anyway, because a state whose stocks and
        // thickness disagree is a different (and broken) world from one where they agree, and a receipt that
        // could not tell them apart would hide exactly that.
        h.write_u64(self.crust_areal_mass.len() as u64);
        for m in &self.crust_areal_mass {
            h.write_fixed(*m);
        }
        h.write_u64(self.source_areal_mass.len() as u64);
        for m in &self.source_areal_mass {
            h.write_fixed(*m);
        }
        // The province-scale bombardment relief.
        h.write_u64(self.impact_relief_m.len() as u64);
        for r in &self.impact_relief_m {
            h.write_fixed(*r);
        }
        // The crater rows, in formation order, which is append-only and so already deterministic.
        h.write_u64(self.craters.len() as u64);
        for c in &self.craters {
            h.write_fixed(c.u);
            h.write_fixed(c.v);
            h.write_fixed(c.diameter_m);
            h.write_fixed(c.depth_m);
            h.write_fixed(c.age_myr);
        }
        h.finish()
    }

    /// The initial state of a fresh planet: `n_cells` mantle columns all at the same starting temperature, none
    /// convecting yet (the Rayleigh-onset latch fires per column as each steepens), and no crust built yet (a
    /// fresh planet has no accumulated crust, it is what the melt history produces). A young planet has NO lateral
    /// thermal history, so the field starts uniform; the variation is what the deep-time run produces, not an
    /// authored initial map.
    ///
    /// `initial_temperature` is the YOUNG POTENTIAL TEMPERATURE the R-YOUNG-TEMPERATURE verdict pins
    /// ([`civsim_physics::young_thermal::young_thermal_verdict`]): for a MELTED world the magma-ocean lock-up
    /// handoff (the world's own derived solidus plus the phi_c superheat, so the columns start super-solidus and
    /// the melt engages), and for a NEVER-MELTED or MARGINAL world the sub-solidus cold peak. This retires the
    /// fixed 1588 K Earth-MORB anchor as the melted-world initial condition; the caller supplies the derived value.
    ///
    /// `source_areal_mass` is the melt SOURCE stock every column begins with (megagrams per square metre,
    /// [`areal_mass_mg_per_m2`]), the caller's DERIVED convecting-mantle areal mass: the convecting-mantle depth
    /// times the mantle density, both of which the caller already derives from the planet's interior structure.
    /// It is uniform for the same reason the columns are: a fresh planet has no lateral history to have spent one
    /// column's source further than another's.
    pub fn young(n_cells: usize, initial_temperature: Fixed, source_areal_mass: Fixed) -> Self {
        DeepTimeState {
            columns: vec![
                ColumnState {
                    temperature: initial_temperature,
                    convecting: false,
                };
                n_cells
            ],
            crust_thickness_km: vec![Fixed::ZERO; n_cells],
            // A fresh planet has emplaced no crust, so the crust stock is empty and the whole ledger sits in the
            // source. Every megagram the crust later holds is one this stock lost.
            crust_areal_mass: vec![Fixed::ZERO; n_cells],
            source_areal_mass: vec![source_areal_mass; n_cells],
            elapsed_myr: Fixed::ZERO,
            impact_relief_m: vec![Fixed::ZERO; n_cells],
            craters: Vec::new(),
            impact_count: 0,
            // A fresh planet owes no withheld strikes and has failed to realize none.
            deferred_impacts: 0,
            unrealized_impacts: 0,
            star_age_start_myr: Fixed::ZERO,
        }
    }

    /// The same young planet, but around a star that already carries `age_myr` of main-sequence life at the run's
    /// start (a run beginning mid-life rather than at the star's zero-age main sequence). The planet's own history
    /// still begins fresh (uniform columns, no crust); only the star's clock is offset. Returns the state so it
    /// chains off [`DeepTimeState::young`].
    pub fn with_star_start_age(mut self, age_myr: Fixed) -> Self {
        self.star_age_start_myr = age_myr;
        self
    }

    /// The star's CURRENT main-sequence age (megayears): its age when the run began plus the geological time
    /// elapsed since. The star and the planet share one deep-time clock, so as the run steps the star ages with it.
    pub fn star_age_myr(&self) -> Fixed {
        self.star_age_start_myr.saturating_add(self.elapsed_myr)
    }

    /// The CURRENT stellar luminosity ratio `L(t)/L_sun` the deep-time surface reads at this run's present age.
    /// This is the thread from the star's aging to the planet's warmth: the surface reads this each step and warms
    /// as it climbs ([`current_luminosity_ratio`]). `None` once the star has left the main sequence at `t_ms`, the
    /// post-main-sequence regime being a separate stellar-evolution arc.
    pub fn stellar_luminosity_ratio(&self, aging: &StarAgingParams) -> Option<Fixed> {
        current_luminosity_ratio(aging, self.star_age_myr())
    }
}

/// THE LEDGER'S UNIT BRIDGE: the areal mass (megagrams per square metre) a layer of `thickness_km` at
/// `density_kg_per_m3` carries. The conversion is EXACT and factor-free, which is why the unit was chosen: a
/// thickness in kilometres times a density in kilograms per cubic metre IS an areal mass in megagrams per square
/// metre, because the `1000 m/km` that scales the thickness up and the `1000 kg/Mg` that scales the mass down
/// cancel. A unit conversion, never a world-content value (Principle 11).
///
/// WHY NOT KILOGRAMS PER SQUARE METRE, the obvious SI choice: a planetary mantle column does not fit. A
/// Mars-class convecting mantle (about 1500 km at 3300 kg per cubic metre) is `5.0e9 kg/m^2` and an Earth-class
/// one is past `1e10`, both above the `2.147e9` ceiling of Q32.32, so the SOURCE STOCK could not be held at all.
/// The same column in megagrams per square metre is `5.0e6`, and a 30 km crust is `8.7e4`, both far inside the
/// window with the fixed point's sub-nanogram resolution still under them. `None` on a NEGATIVE thickness, a
/// non-positive density, or an overflow, so a degenerate layer refuses rather than returning a fabricated mass.
/// A ZERO thickness is not degenerate and returns `Some(0)`: a layer of no thickness has no mass, which is the
/// right answer rather than a refusal, and the melt transaction reads it as one of its two physical zeros
/// ([`step_deep_time`]). The guard says "non-positive" for the density alone, because a material of no density
/// is a material that does not exist.
pub fn areal_mass_mg_per_m2(thickness_km: Fixed, density_kg_per_m3: Fixed) -> Option<Fixed> {
    if thickness_km < Fixed::ZERO || density_kg_per_m3 <= Fixed::ZERO {
        return None;
    }
    thickness_km.checked_mul(density_kg_per_m3)
}

/// The inverse of [`areal_mass_mg_per_m2`]: the THICKNESS (kilometres) an areal mass of `areal_mass_mg_per_m2`
/// megagrams per square metre stands as at `density_kg_per_m3`. This is the crust stock's geometric readout, the
/// one place [`DeepTimeState::crust_thickness_km`] comes from, so the isostasy floats a thickness that is a
/// function of the ledger rather than a parallel accumulation of it. `None` on a non-positive density or an
/// overflow.
pub fn thickness_km_from_areal_mass(
    areal_mass_mg_per_m2: Fixed,
    density_kg_per_m3: Fixed,
) -> Option<Fixed> {
    if density_kg_per_m3 <= Fixed::ZERO {
        return None;
    }
    areal_mass_mg_per_m2.checked_div(density_kg_per_m3)
}

/// The kilometre-to-metre unit bridge (`1 km = 1000 m`), the exact factor the crust thickness (kilometres)
/// converts through to the metres the impact relief and the ballistic surface carry, and that the support bound
/// converts back through. A fundamental unit conversion (Principle 11), never an authored world-content value.
const M_PER_KM: Fixed = Fixed::from_int(1000);

/// The per-world melt-and-crust parameters the deep-time volcanism reads: the seam-6 adiabatic-melt-column
/// closure's inputs (the solidus surface value and slope, the adiabat slope, the melt productivity, the source
/// density, and gravity), plus the mantle PROCESSING TIME over which one melt column's worth of crust is
/// delivered to the surface. The solidus surface value and slope are DERIVED from the world's own endmember
/// signatures upstream ([`civsim_materials::surface_composition::SurfaceComposition::solidus_surface_k`]), never
/// an authored peridotite value; the adiabat, productivity, source density, and gravity are floor values or
/// per-world data. The processing time is a flagged DERIVE-DOWN, not an independent knob: it is the overturn /
/// melt-delivery timescale `convecting_depth / convective_velocity`, and BOTH ingredients already exist in the
/// convection kernel ([`crate::geodynamics::convecting_mantle_depth_m`] and the Stokes velocity
/// [`civsim_physics::laws::stokes_velocity`]), so it derives the moment those run at physical SI scale. It stays
/// reserved-with-basis only because the kernel currently runs on the representable-scaled interim operating point
/// (the Stokes velocity is physical only in SI), so the derive-down is coupled to the same SI / Tier-2 units
/// wiring that retires that operating point ([`province_column_params`]); ~100 Myr to 1 Gyr for a silicate
/// mantle is its surfaced-basis value, not a fabricated one.
#[derive(Clone, Copy, Debug)]
pub struct MeltParams {
    /// The solidus surface temperature (K), the seam-6 melt-column input. DERIVED upstream from the world's own
    /// endmember signatures, never an authored solidus.
    pub solidus_surface_k: Fixed,
    /// The solidus slope (K per GPa).
    pub solidus_slope_k_per_gpa: Fixed,
    /// The adiabat slope (K per GPa).
    pub adiabat_slope_k_per_gpa: Fixed,
    /// The melt productivity (per GPa).
    pub productivity_per_gpa: Fixed,
    /// The melt source density (kg per cubic metre).
    pub source_density_kg_per_m3: Fixed,
    /// The EMPLACED-CRUST density (kg per cubic metre), the density the extracted melt stands as crust at, and so
    /// the ledger's own conversion: the mass the transaction debits from the source is the mass the crust HAS
    /// (`thickness * this`), and the thickness the isostasy floats is the crust stock divided by it again
    /// ([`areal_mass_mg_per_m2`], [`thickness_km_from_areal_mass`]). It is the SAME derived crust density the
    /// Airy flotation reads ([`SupportBoundParams`] carries it in grams per cubic centimetre), and a caller feeds
    /// both from one derived home, as it already feeds one derived gravity into this struct, the crater target,
    /// and the ballistic forces.
    ///
    /// A NAMED INCONSISTENCY it inherits rather than creates, recorded because the ledger is the first thing that
    /// makes it visible: [`adiabatic_melt_column`] forms its column thickness with the SOURCE density in the
    /// denominator (`crust = (dF/dP) * P0^2 / (2 * rho_source * g)`, the pressure-to-depth conversion
    /// `dz = dP / (rho_source * g)`), and everything downstream then floats that same thickness at the CRUST
    /// density. So the mass this ledger moves is the mass the crust has and the mass the isostasy weighs, and it
    /// does not equal the melt mass the column's own integral implies.
    ///
    /// THE SIZE OF THAT RESIDUAL IS NOT DETERMINED, which is the sharper finding and the reason nothing is
    /// reconciled here. It depends on what basis the melt fraction `F` carries, and `civsim_physics::melting`
    /// does not declare one. If `F` is a MASS fraction the extracted melt is `t * rho_source`, the residual is
    /// `t * (rho_source - rho_crust)`, and the familiar density-ratio figure (about 14 percent of the crust mass
    /// for a mafic crust off a peridotite source) is the right one. If `F` is a VOLUME fraction the extracted
    /// melt is `t * rho_melt` at the LIQUID density, which is near the crust density rather than the source
    /// density, so the residual is small and is a melt-to-solid densification instead. And the one supplier the
    /// melt module itself names for the productivity term ([`civsim_physics::melting::batch_melt_fraction`], a
    /// lever rule over MOLE fractions) yields neither without endmember molar masses, which nothing on that path
    /// reads. Meanwhile the value callers feed is the McKenzie-Bickle literature productivity, a mass-basis
    /// figure. Two suppliers of one term on two bases is the actual defect.
    ///
    /// So this is FLAGGED and not settled, rather than settled by picking whichever density made the numbers
    /// agree. What the ledger needs from `civsim_physics::melting` before it can close the residual: the melt
    /// fraction's BASIS declared, and, on a mass basis, the extracted melt MASS per unit area returned beside the
    /// thickness (on a volume basis, a melt density taken as an input, since the liquid density is neither the
    /// source's nor the crust's). Only then does the destination-explicit form
    /// (`source debit = extracted melt = crust credit + retained melt + lost volatiles + returned residue`) have
    /// a determined total to partition, and the three destination stocks it needs can be sized rather than
    /// guessed. Adding those stocks now would encode an assumption about the basis, which is the fabrication the
    /// flag exists to prevent.
    pub crust_density_kg_per_m3: Fixed,
    /// Gravity (m per second squared).
    pub gravity_m_per_s2: Fixed,
    /// The mantle processing time (megayears), the overturn timescale one melt column's crust is delivered over.
    /// A flagged DERIVE-DOWN (`convecting_depth / convective_velocity`, both in the convection kernel), reserved
    /// only until the SI / Tier-2 units wiring lets the Stokes velocity run at physical scale; not an independent knob.
    pub processing_time_myr: Fixed,
}

/// THE VOLCANISM: the crust increment a column's interior melt delivers in one tick, RELAXING the crust toward
/// the equilibrium thickness the melt column supports rather than accumulating a flux without bound. The column's
/// current interior temperature is the mantle potential temperature the seam-6 [`adiabatic_melt_column`]
/// (McKenzie-Bickle) crosses against the solidus, and the crust that a full melt column makes is the EQUILIBRIUM
/// the column can support: the fusible source is finite, so as the crust approaches that equilibrium the source
/// depletes and production falls to zero. The tick closes the remaining deficit over the mantle processing time,
/// `(equilibrium - prev_crust_km) * dt_myr / processing_time`, CLAMPED non-negative: a crust already at or above
/// the column's equilibrium builds no more (the saturation, the physics of a finite source, not an authored cap),
/// and a made crust does not un-form when the mantle cools. A sub-solidus column (a mantle colder than its
/// surface solidus) makes no melt and no crust: zero, never negative, never a fabricated crust. This is
/// derive-first (the equilibrium is the melt the interior produces, keyed on the column's own temperature) and
/// the province-builder AND the province bound: a hotter, longer-lived column relaxes to a thicker equilibrium
/// than its neighbour, so the surface crust spread is the BOUNDED readout of the interior's melt history rather
/// than an unbounded integral. Returns the crust increment (kilometres), non-negative.
pub fn crust_growth(
    potential_temperature_k: Fixed,
    prev_crust_km: Fixed,
    melt: &MeltParams,
    dt_myr: Fixed,
) -> Fixed {
    if melt.processing_time_myr <= Fixed::ZERO || dt_myr <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    match adiabatic_melt_column(
        potential_temperature_k,
        melt.solidus_surface_k,
        melt.solidus_slope_k_per_gpa,
        melt.adiabat_slope_k_per_gpa,
        melt.productivity_per_gpa,
        melt.source_density_kg_per_m3,
        melt.gravity_m_per_s2,
    ) {
        // The deficit toward the equilibrium crust the column supports. Clamped non-negative: at or above the
        // equilibrium the finite source is spent (saturation) and the crust does not un-form, so no negative step.
        Some(column) => {
            let deficit = column
                .crust_thickness_km
                .checked_sub(prev_crust_km)
                .unwrap_or(Fixed::ZERO);
            if deficit <= Fixed::ZERO {
                return Fixed::ZERO;
            }
            deficit
                .checked_mul(dt_myr)
                .and_then(|c| c.checked_div(melt.processing_time_myr))
                .unwrap_or(Fixed::ZERO)
        }
        None => Fixed::ZERO,
    }
}

/// Advance the deep-time state by one tick: step every interior column's convection ([`convection_step_si`]), grow
/// each column's crust from the melt its interior now delivers ([`crust_growth`]), and accumulate the elapsed
/// geological time. `column_params` is either ONE entry broadcast to every column (a laterally uniform world) or
/// one per column (each cell's own composition and radiogenic budget, the source of lateral variation); any other
/// length is a caller mismatch and REFUSES the tick, where it used to broadcast the first entry across columns
/// the caller had asked to differ. `melt` is
/// the shared per-world volcanism parameters. `dt_myr` is the tick's geological duration. The crust GROWS TOWARD
/// EACH COLUMN'S EQUILIBRIUM and saturates there: a crust the interior once built stays, but the finite fusible
/// source cannot push it past the equilibrium the column supports ([`crust_growth`]), so the surface thickness is
/// the BOUNDED melt history, hot columns building thicker crust (the provinces, and the derived thickness that
/// retires the 30 km fixture) without an unbounded runaway. Returns the next state. Deterministic and
/// worker-invariant (a pure per-column map in index order). Every failure is a TYPED REFUSAL
/// ([`DeepTimeRefusal`]) rather than a silent no-op: no parameters at all, a parameter set matching neither one
/// column nor every column, a ledger whose stocks do not cover every column, a column whose convection step or
/// whose melt conversion or whose stock transfer left the representable window, a thickness readout the stock
/// cannot back, and a clock that cannot advance.
///
/// THE MELT TRANSACTION. The crust increment is no longer an addition to a thickness: it is a TRANSFER between
/// the two stocks the state now carries. The tick converts the increment [`crust_growth`] asks for into an areal
/// mass at the emplaced-crust density, CAPS it at what the column's source stock still holds (the finite source
/// made real, where the equilibrium saturation alone was a description with no reservoir behind it), then debits
/// the source and credits the crust by ONE AND THE SAME fixed-point value, so
/// `crust_areal_mass[i] + source_areal_mass[i]` is invariant to the bit. The thickness is then re-read off the
/// crust stock ([`thickness_km_from_areal_mass`]) rather than accumulated beside it, so the geometry the isostasy
/// floats stays a function of the ledger and the two cannot drift apart. A column with no source left, and a
/// sub-solidus column with no melt to move, transfer nothing and carry on: those are the finite source and the
/// cold interior, and zero is the right answer for both. A conversion or a transfer that cannot be REPRESENTED
/// refuses the tick instead, because moving zero mass on failed arithmetic is indistinguishable from those two
/// physical zeros and would report a quiescent column where the ledger had broken.
/// WHY A DEEP-TIME STEP REFUSED. Every arm is a stop, and none of them is a state a caller may confuse with
/// a world that simply did not change.
///
/// # WHY THIS REPLACED AN `Option` AND A HOLD
///
/// The per-column convection step used to read `convection_step_si(col, p).unwrap_or(*col)`, under a comment
/// arguing that the parameters were validated when the province field was built, so a refusal here was "an
/// arithmetic backstop and not a path the physics takes". VALIDATION AT CONSTRUCTION DOES NOT PROVE EVERY
/// LATER STATE REMAINS IN SCOPE: the column evolves, its temperature moves every tick, and a state reachable
/// after a thousand steps can leave the domain that the initial one sat inside. When that happened the fallback
/// asserted "this column did not evolve", which is a PHYSICAL claim the arithmetic had no basis for, and it
/// would have been invisible because a held column looks exactly like a cold one.
///
/// Measured before the change rather than assumed: over both pinned scenarios the refusal path is taken ZERO
/// times, so making it loud is byte-neutral today. That it does not fire on two worlds is what makes it
/// UNTESTED rather than what makes it safe.
///
/// The transaction is ATOMIC in the refusal: one column that cannot step refuses the whole tick, because a
/// state with some columns advanced and one held is a world that never existed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeepTimeRefusal {
    /// No column parameters were supplied, so there is nothing to step.
    NoColumnParams,
    /// The areal-mass ledger does not cover every column, so the transaction has no stock to balance over.
    LedgerDoesNotCoverEveryColumn,
    /// A column's convection step left the representable window or its own domain. Carries the column index so
    /// the failure is locatable rather than merely reported.
    ColumnStepRefused { index: usize },
    /// A column's crust-thickness readout refused after the melt transaction moved its mass.
    ThicknessReadoutRefused { index: usize },
    /// The parameter set covers neither ONE column (the laterally uniform broadcast) nor every column, so the
    /// caller and the field disagree about how many distinct interiors this world has. It used to BROADCAST
    /// entry zero over every column, so seven parameter rows against a hundred columns silently ran a hundred
    /// copies of the first row: a laterally uniform world, delivered to a caller who had asked for a varied one
    /// and had no way to tell. Carries both counts so the mismatch is locatable.
    ColumnParamsLengthMismatch {
        /// How many parameter rows the caller supplied.
        supplied: usize,
        /// How many columns the field holds.
        columns: usize,
    },
    /// A column's melt increment could not be CONVERTED to an areal mass (the product left the representable
    /// window, or the emplaced-crust density is non-positive). Distinct from the two physical zeros (a
    /// sub-solidus column with no melt, and a source already spent), which are real zeros and continue.
    MeltMassUnrepresentable { index: usize },
    /// A column's melt TRANSFER could not be applied: the credit to the crust stock or the debit from the source
    /// stock left the representable window. Holding both stocks would have preserved the PAIRING while asserting
    /// "no melt moved this tick", a physical claim the failed arithmetic has no basis for.
    TransferUnrepresentable { index: usize },
    /// The geological clock could not advance: `elapsed + dt` left the representable window. Magnitude-unreachable
    /// in any physical run (Q32.32 rails near `2.147e9` megayears, about 155,000 times the age of the universe),
    /// and an arm regardless, because "unreachable" is a claim about magnitudes rather than about the arithmetic,
    /// and a saturating clock reports a tick that advanced time when it did not.
    ClockOverflow,
}

pub fn step_deep_time(
    state: &DeepTimeState,
    column_params: &[SiColumnParams],
    melt: &MeltParams,
    dt_myr: Fixed,
) -> Result<DeepTimeState, DeepTimeRefusal> {
    if column_params.is_empty() {
        return Err(DeepTimeRefusal::NoColumnParams);
    }
    // The ledger must cover every column, else the transaction has no stock to balance over for some cell. A
    // caller mismatch is refused loud rather than partly-transacted.
    let n = state.columns.len();
    if state.crust_thickness_km.len() != n
        || state.crust_areal_mass.len() != n
        || state.source_areal_mass.len() != n
    {
        return Err(DeepTimeRefusal::LedgerDoesNotCoverEveryColumn);
    }
    // The parameters must arrive in one of the two DOCUMENTED shapes: one entry broadcast to every column, or one
    // entry per column. Any other length used to fall through to entry zero for every column, which is a silent
    // BROADCAST of one interior over a field the caller had asked to vary; see `DeepTimeRefusal`.
    if column_params.len() != 1 && column_params.len() != n {
        return Err(DeepTimeRefusal::ColumnParamsLengthMismatch {
            supplied: column_params.len(),
            columns: n,
        });
    }
    let per_column = column_params.len() == state.columns.len();
    let columns: Vec<ColumnState> = state
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let p = if per_column {
                &column_params[i]
            } else {
                &column_params[0]
            };
            // A refusal REFUSES THE TICK, carrying which column could not step. It used to hold the column
            // instead, which stated that the column did not evolve; see `DeepTimeRefusal`.
            convection_step_si(col, p).ok_or(DeepTimeRefusal::ColumnStepRefused { index: i })
        })
        .collect::<Result<Vec<_>, _>>()?;
    // THE VOLCANISM, AS A TRANSACTION. Each column's stepped interior temperature asks for a crust increment
    // (relaxing toward the equilibrium the column supports, so a made crust stays and does not un-form when the
    // mantle cools). That increment is then PAID FOR out of the column's own source stock: converted to an areal
    // mass, capped at what the source still holds, and moved. Mass is what conserves; the thickness follows.
    let mut crust_thickness_km = Vec::with_capacity(n);
    let mut crust_areal_mass = Vec::with_capacity(n);
    let mut source_areal_mass = Vec::with_capacity(n);
    for (i, col) in columns.iter().enumerate() {
        let prev_thickness = state.crust_thickness_km[i];
        let prev_crust = state.crust_areal_mass[i];
        let prev_source = state.source_areal_mass[i];
        // What the melt column asks for this tick, in thickness, then in the mass that thickness costs.
        let asked_km = crust_growth(col.temperature, prev_thickness, melt, dt_myr);
        let asked_mass = areal_mass_mg_per_m2(asked_km, melt.crust_density_kg_per_m3);
        // What the source can pay. Two PHYSICAL zeros continue here, and one FAILURE refuses, where a single
        // catch-all arm used to swallow all three together. The physical zeros are a sub-solidus column that
        // asked for no melt and a source already spent (the finite-source limit as a reservoir rather than as a
        // clamp); both move zero mass because zero is the right answer. A conversion that could not be FORMED
        // moves zero mass because the arithmetic failed, and pooling it with the other two made it unobservable
        // by construction: the state it produced was identical to a quiescent column's.
        let moved = match asked_mass {
            Some(m) if m > Fixed::ZERO && prev_source > Fixed::ZERO => m.min(prev_source),
            Some(_) => Fixed::ZERO,
            None => return Err(DeepTimeRefusal::MeltMassUnrepresentable { index: i }),
        };
        // THE TRANSFER, the same value out of one stock and into the other, so the pair's sum is invariant to the
        // bit. `checked_*` on both legs keeps the two sides from ever moving apart: a credit that overflowed
        // while its debit landed would be mass created out of a rounding boundary. Holding BOTH stocks preserved
        // that pairing, which is why it was written, and it also asserted "no melt moved this tick", which the
        // failed arithmetic gives no basis for. The pairing is preserved by refusing instead.
        let (next_crust, next_source) = match (
            prev_crust.checked_add(moved),
            prev_source.checked_sub(moved),
        ) {
            (Some(c), Some(s)) => (c, s),
            _ => return Err(DeepTimeRefusal::TransferUnrepresentable { index: i }),
        };
        // The thickness is the crust stock's READOUT, re-read each tick rather than accumulated beside it. A
        // refusal holds the previous thickness rather than reporting a crust the stock does not back.
        let next_thickness = thickness_km_from_areal_mass(next_crust, melt.crust_density_kg_per_m3)
            .ok_or(DeepTimeRefusal::ThicknessReadoutRefused { index: i })?;
        crust_thickness_km.push(next_thickness);
        crust_areal_mass.push(next_crust);
        source_areal_mass.push(next_source);
    }
    // THE CLOCK. A saturating add would have railed the clock at the fixed-point ceiling and returned a state
    // reporting that the tick advanced geological time when it did not, and every later tick would have railed
    // silently at the same instant. It is the same failure-to-no-change conversion as the three above, at a
    // magnitude no physical run reaches; see `DeepTimeRefusal::ClockOverflow`.
    let elapsed_myr = state
        .elapsed_myr
        .checked_add(dt_myr)
        .ok_or(DeepTimeRefusal::ClockOverflow)?;
    Ok(DeepTimeState {
        columns,
        crust_thickness_km,
        crust_areal_mass,
        source_areal_mass,
        elapsed_myr,
        // The bombardment is a SEPARATE step term ([`bombard_tick`]), applied by the caller after this one, so the
        // interior tick carries the impact record forward unchanged and stays byte-identical for a run that does
        // not bombard (the viewer's existing interior-and-volcanism step is untouched).
        impact_relief_m: state.impact_relief_m.clone(),
        craters: state.craters.clone(),
        impact_count: state.impact_count,
        deferred_impacts: state.deferred_impacts,
        unrealized_impacts: state.unrealized_impacts,
        // The star's start age is a per-run constant; its CURRENT age advances through `elapsed_myr` above, so the
        // star ages on the same clock as the interior without a separate step term.
        star_age_start_myr: state.star_age_start_myr,
    })
}

/// The per-world impact-flux configuration the deep-time bombardment reads, every field a datum (Principle 11,
/// admit-the-alien): the accretion-tail reservoir and the collisional-cascade size-frequency distribution the
/// flux model draws from, the per-event physical state the crater law and ejecta fan read, and one determinism
/// and cost bound. A different disk, a captured swarm, or an alien impactor population is a different set of
/// numbers through the SAME wiring, not a new code path. Nothing here is authored inline; the fields are the
/// world's own reserved-with-basis data.
#[derive(Clone, Copy, Debug)]
pub struct ImpactFluxParams {
    /// The reservoir's total impacting-body count over the whole accretion tail (the number of bodies above
    /// `min_impactor_radius_m` the leftover-planetesimal reservoir holds). The tail spreads this count over deep
    /// time as `exp(-t/tau)`, so the expected strikes in a tick are this count times the tail's decline across
    /// the tick. Reserved-with-basis: the leftover-planetesimal count the world's disk delivers (the residual
    /// reservoir mass over the mean body mass of the size-frequency distribution), derived-down when the disk
    /// model supplies the residual mass; a per-world datum, never fabricated.
    pub reservoir_body_count: Fixed,
    /// The accretion-tail sweep-up timescale `tau` (megayears), the flux model's own decay constant. Reserved-
    /// with-basis (the planet's gravitational-focusing sweep-up time over the reservoir's dynamical spreading,
    /// the flux model's per-world dynamical value, near tens of megayears for late accretion).
    pub sweep_timescale_myr: Fixed,
    /// The collisional-cascade differential size-frequency slope `p` (the Dohnanyi slope, near 3.5). Cited to the
    /// cascade literature (Dohnanyi 1969), the same slope the grain-opacity cascade reads.
    pub differential_slope: Fixed,
    /// The smallest impactor radius the reservoir delivers (metres), the size-frequency lower bound. A per-world
    /// reservoir datum.
    pub min_impactor_radius_m: Fixed,
    /// The largest impactor radius the reservoir delivers (metres), the size-frequency upper bound (the
    /// reservoir's biggest surviving body). A per-world reservoir datum.
    pub max_impactor_radius_m: Fixed,
    /// The impact closing speed `U` (metres per second) the crater law reads; the encounter geometry and the
    /// world's gravity deliver it. Reserved-with-basis (the reservoir's mean surface encounter speed, the escape
    /// speed folded with the approach velocity). A per-world datum.
    pub impact_velocity_m_s: Fixed,
    /// The impactor bulk density (kilograms per cubic metre), the reservoir's composition. A per-world datum.
    pub impactor_density: Fixed,
    /// The target's surface material and world state the crater law reads (gravity, effective strength, bulk
    /// density), each a floor axis or a per-world datum.
    pub target: Target,
    /// The target material's crater coupling row (the coupling exponents and fit coefficients), reserved-with-
    /// basis per material as the crater law documents.
    pub coupling: CraterCoupling,
    /// The ejecta ballistic launch (speed, elevation angle, azimuth count) the isotropic fan fires. Physical
    /// launch data, reserved-with-basis (the characteristic ejecta launch speed, a fraction of the impact speed
    /// near the 45-degree maximum-range angle; the azimuth count a resolution-and-determinism bound). One
    /// characteristic launch for every event this world, the honest first-slice limit: a per-event ejecta speed
    /// scaling with the crater's own excavation velocity is the named refinement.
    pub ejecta: EjectaFan,
    /// The world force parameters the ballistic arc reads: gravity (the floor value, in metres per second
    /// squared), the cell edge length (metres, the province grid's own spatial scale), and the march step cap.
    pub forces: BallisticForces,
    /// The maximum number of impacts applied in one tick, a determinism-and-cost bound (the per-tick work is
    /// bounded however intense the early bombardment is), never a physical limit. Reserved-with-basis: the
    /// per-tick impact budget, set so the earliest, most intense ticks stay inside the tick's compute envelope;
    /// the accumulated relief is still heavily cratered early.
    pub per_tick_impact_cap: u32,
}

/// THE BOMBARDMENT: draw this tick's impacts from the accretion-tail flux and record each as a discrete crater
/// ROW ([`DeepTimeState::craters`], the individual the renderer stamps analytically), applying the CROSS-SCALE
/// WRITE RULE so a crater at or above the convective cell size ALSO rasterizes into
/// [`DeepTimeState::impact_relief_m`] (the large-basin province feedback) while a sub-cell crater writes a ROW
/// only. Returns the next state with the crater rows appended and the [`DeepTimeState::impact_count`] advanced.
/// Call it AFTER [`step_deep_time`] each tick, so `state.elapsed_myr` is the tick's END: the flux interval is
/// `[elapsed_myr - dt_myr, elapsed_myr]`, and the fraction of the reservoir swept across it (the accretion
/// tail's own decline, `rate(t0) - rate(t1)`) is the expected strike count once scaled by the reservoir's body
/// count. Because that decline is steep early and flat late, an early tick draws many impacts and a late one
/// few (the honest late-heavy-bombardment history), never an authored bombardment curve. Each impact's size is
/// DRAWN from the collisional-cascade size-frequency distribution ([`size_at_number_fraction`], large basins
/// rare and small craters common), its location a uniform cell (the maximum-entropy prior, no authored spatial
/// pattern), and the crater-scaling law ([`crater`]) derives its rim diameter and bowl depth (the row's size).
/// A large basin's raster goes through the already-built [`apply_impact`] (the crater-scaling law, the ballistic
/// ejecta fan, the conservative redistribution); the returned delta sums to exactly zero, so the impact relief's
/// sum never changes (mass is moved, never created or lost).
///
/// Determinism and the bound (Principle 3, Principle 10): the strike count and every location and size draw come
/// from ONE seeded stream keyed on the world identity and the tick index ([`Rng::for_coords`], the observer-safe
/// coordinate path, the splitmix64 counter style), never floating randomness; the integer strike count is the
/// expected value's floor plus a seeded Bernoulli on its fractional remainder (deterministic stochastic
/// rounding, so the run delivers the expected total without authoring a per-tick count), capped at
/// `per_tick_impact_cap` so the per-tick work is bounded. THE CAP WITHHOLDS RATHER THAN TRUNCATES: what it
/// cannot admit this tick is queued on [`DeepTimeState::deferred_impacts`] and re-offered on the next one, so a
/// compute bound no longer deletes bodies from a reservoir whose analytic decline goes on counting them as
/// swept. An event the world's own parameters cannot realize (a size the distribution will not draw, a crater
/// the law will not solve, a raster failing its zero-sum or representability check) is counted on
/// [`DeepTimeState::unrealized_impacts`] instead, the honest loss, because a retry would refuse it identically
/// forever. A degenerate call (a zero-area grid, a grid that does not match the province field, a non-positive
/// tick duration), a crust thickness whose metre conversion is unrepresentable, or a tail rate outside its own
/// domain falls SOFT to the unchanged state, never a panic and never a fabricated impact.
pub fn bombard_tick(
    state: &DeepTimeState,
    width: usize,
    height: usize,
    flux: &ImpactFluxParams,
    world_seed: u64,
    tick_index: u64,
    dt_myr: Fixed,
) -> DeepTimeState {
    let mut next = state.clone();
    let n_cells = width.saturating_mul(height);
    // Dormant and soft on a degenerate call or a grid-versus-field mismatch: no change, never a panic.
    if width == 0
        || height == 0
        || n_cells != state.columns.len()
        || state.impact_relief_m.len() != n_cells
        // The crust thickness is INDEXED over the same grid when the running surface is built, so a short one
        // would have indexed out of bounds in a function documented never to panic.
        || state.crust_thickness_km.len() != n_cells
        || dt_myr <= Fixed::ZERO
    {
        return next;
    }

    // The flux DECLINES with epoch. The fraction of the reservoir swept across this tick's interval
    // [t0, t1] = [elapsed - dt, elapsed] is `rate(t0) - rate(t1)` (the accretion tail's own decline over the
    // interval, since the cumulative accreted fraction is `1 - rate`), so an early tick (steep decline) captures
    // many bodies and a late tick (flat, spent tail) few. Clamp the interval start to zero for the first tick.
    let t1 = state.elapsed_myr;
    let mut t0 = state.elapsed_myr.checked_sub(dt_myr).unwrap_or(Fixed::ZERO);
    if t0 < Fixed::ZERO {
        t0 = Fixed::ZERO;
    }
    let rate0 = match tail_rate_fraction(t0, flux.sweep_timescale_myr) {
        Some(r) => r,
        None => return next, // a non-positive sweep timescale: no decay defined, no draw
    };
    // A REFUSAL AT THE INTERVAL'S END IS NOT A ZERO RATE. This read `unwrap_or(Fixed::ZERO)`, which reports a
    // failed rate as a fully-swept tail and so makes `swept_fraction` MAXIMAL: alone among the fallbacks in this
    // function, it turned a failure into the LARGEST possible draw rather than the smallest. The tail rate
    // refuses only on a domain error (a negative clock) or an overflow, and neither is a statement about how
    // much reservoir remains.
    let rate1 = match tail_rate_fraction(t1, flux.sweep_timescale_myr) {
        Some(r) => r,
        None => return next,
    };

    // The stream is keyed on the world identity and the tick, the observer-safe coordinate path, never floating
    // randomness. Counter 0 is the strike-count Bernoulli and the per-strike draws start at 1, so a tick that
    // delivers no new bodies consumes no counter and the per-strike stream is unshifted.
    let rng = Rng::for_coords(world_seed, &[tick_index]);
    // THE NEW BODIES THIS TICK: the reservoir's body count times the fraction the tail swept across the interval,
    // rounded to a whole number of events by a SEEDED Bernoulli on the fractional remainder (deterministic
    // stochastic rounding, so the run delivers the expected total without authoring a per-tick count). A spent
    // tail (nothing swept) or an exhausted reservoir delivers no new bodies, which is a physical statement rather
    // than a failure, so the deferred queue below still drains against it.
    //
    // THE SHOT-NOISE GAP, measured rather than waved at. Floor-plus-Bernoulli has variance `r(1-r) <= 0.25` in
    // the count, whatever the expectation, where a Poisson arrival process has variance equal to the expectation
    // itself. At an expected 10.3 strikes that is 0.21 against 10.3, an under-dispersion of about fiftyfold, so
    // this bombardment carries essentially NO shot noise: it delivers the right mean with almost none of the
    // clustering a real accretion tail shows, and a run cannot produce an anomalously heavy or light epoch by
    // chance. Closing it needs a seeded Poisson sampler, which the workspace does not have
    // ([`civsim_core::Rng`] exposes `for_entity`, `for_coords`, `range_i32`, `range_u32` and `unit_fixed`, with
    // no arrival-process draw), so it is a named follow-on rather than a defect fixed here.
    let newly_offered = match rate0
        .checked_sub(rate1)
        .filter(|swept| *swept > Fixed::ZERO)
        .and_then(|swept| flux.reservoir_body_count.checked_mul(swept))
        .filter(|expected| *expected > Fixed::ZERO)
    {
        Some(expected) => {
            let floor_count = expected.to_int().max(0) as u64;
            let remainder = expected
                .checked_sub(Fixed::from_int(expected.to_int()))
                .unwrap_or(Fixed::ZERO);
            floor_count.saturating_add(u64::from(rng.unit_fixed(0) < remainder))
        }
        None => 0,
    };

    // THE OFFER, and the WITHHOLDING that makes the cap conservative. This tick's new bodies plus the queue the
    // cap withheld earlier are offered together; the cap admits what it can and the REST IS WITHHELD rather than
    // discarded. The cap is a compute bound, so truncating at it used to delete real bodies from a reservoir
    // whose analytic decline went on counting them as swept. Every body the tail delivers now either strikes, is
    // still queued, or is counted unrealizable ([`DeepTimeState::unrealized_impacts`]), so the budget closes.
    let offered = newly_offered.saturating_add(state.deferred_impacts);
    let count = offered.min(flux.per_tick_impact_cap as u64);
    next.deferred_impacts = offered - count;
    if count == 0 {
        return next;
    }

    // The running surface the ejecta arcs fly over: the volcanic crust (kilometres, converted to metres) plus
    // the bombardment relief built so far, so an impact clears the real relief (volcanic highlands AND earlier
    // craters), and a later strike this tick sees the craters the earlier ones dug.
    //
    // BOTH LEGS ARE CHECKED and the whole surface is built or none of it is. The kilometre-to-metre conversion
    // fell back to zero and the relief add saturated, so a column whose crust could not be expressed in metres
    // reached the ballistic arc as a datum-level surface: the ejecta flew over sea level where the world holds a
    // highland, and nothing recorded it. A surface this tick cannot form is a tick it cannot bombard, which is
    // the soft-fail the degenerate-call guard above already takes.
    let surface_built: Option<Vec<Fixed>> = (0..n_cells)
        .map(|i| {
            next.crust_thickness_km[i]
                .checked_mul(M_PER_KM)
                .and_then(|crust_m| crust_m.checked_add(next.impact_relief_m[i]))
        })
        .collect();
    let mut surface_m = match surface_built {
        Some(s) => s,
        None => return next,
    };

    for i in 0..count {
        // Distinct counters per strike keep the location and size draws independent within the tick stream.
        let source = rng.range_u32(2 * i + 1, n_cells as u32) as usize;
        let u = rng.unit_fixed(2 * i + 2);
        let radius = match size_at_number_fraction(
            u,
            flux.min_impactor_radius_m,
            flux.max_impactor_radius_m,
            flux.differential_slope,
        ) {
            Some(r) => r,
            // A non-drawable reservoir (a degenerate size range): never fabricate a size. COUNTED rather than
            // merely skipped, because the event came out of the reservoir's budget; and NOT deferred, because
            // the same range would refuse it identically on every later tick, so re-offering it would spin.
            None => {
                next.unrealized_impacts = next.unrealized_impacts.saturating_add(1);
                continue;
            }
        };
        let impactor = Impactor {
            radius,
            velocity: flux.impact_velocity_m_s,
            density: flux.impactor_density,
        };
        // The crater the scaling law derives for this strike: its rim diameter and transient bowl depth, the
        // row's true size and the cross-scale threshold's comparand. A non-physical or unbounded impact yields no
        // crater; skip it, never fabricate a size.
        let bowl = match crater(impactor, flux.target, flux.coupling) {
            Some(c) => c,
            // Counted for the same reason as the size refusal above: a crater the law cannot solve for this
            // world's target and coupling will not solve on a retry either.
            None => {
                next.unrealized_impacts = next.unrealized_impacts.saturating_add(1);
                continue;
            }
        };

        // THE BASIN RASTER IS PREPARED AND VALIDATED BEFORE ANYTHING IS COMMITTED, so a strike lands WHOLE or
        // not at all (the same atomicity `DeepTimeRefusal` argues for the interior tick). The row used to be
        // pushed and the count advanced before the raster was attempted, so a basin that could not be applied
        // left a crater row and a count with no relief behind them.
        let basin_raster = if bowl.diameter >= flux.forces.cell_size {
            match prepared_basin_raster(
                width,
                height,
                &surface_m,
                source,
                impactor,
                flux,
                &next.impact_relief_m,
            ) {
                Some(r) => Some(r),
                None => {
                    next.unrealized_impacts = next.unrealized_impacts.saturating_add(1);
                    continue;
                }
            }
        } else {
            None
        };

        // EMIT THE ROW: the discrete crater individual, recorded at its true derived size (rows not rasters). The
        // position is the drawn cell's CENTRE as a normalized surface coordinate `(u, v)`, resolution-independent
        // so a finer display grid resolves it; the diameter and depth are the crater law's own outputs, so the
        // morphology conditions on the world. The renderer stamps it analytically at the viewport's resolution.
        let col = (source % width) as i64;
        let row = (source / width) as i64;
        next.craters.push(CraterRow {
            u: Fixed::from_ratio(2 * col + 1, 2 * width as i64),
            v: Fixed::from_ratio(2 * row + 1, 2 * height as i64),
            diameter_m: bowl.diameter,
            depth_m: bowl.depth,
            age_myr: t1,
        });
        next.impact_count = next.impact_count.saturating_add(1);

        // THE CROSS-SCALE WRITE RULE: a process writes into a field only at or above that field's derived scale.
        // A crater whose rim diameter is at or above the convective cell size (`flux.forces.cell_size`, the
        // province grid spacing) STILL rasterizes into the province relief field, through the built conservative
        // [`apply_impact`] (the basin that resurfaces a province, its thermal/province feedback). A sub-cell
        // crater has written its ROW above and touches this coarse field no further (rows not rasters), so the
        // province field is no longer smeared with sub-cell craters.
        //
        // The raster was composed and CHECKED above, through the already-built chain (the crater law's
        // paraboloid bowl, the isotropic ballistic ejecta fan, the conservative redistribution), so committing
        // it here is an assignment that cannot fail. Both fields move by the identical delta, which preserves
        // each sum exactly and keeps a later basin this tick flying over the terrain the earlier ones dug.
        if let Some((relief, surface)) = basin_raster {
            next.impact_relief_m = relief;
            surface_m = surface;
        }

        // HEAT-LEDGER HOOK (a deep-time interior heat ledger does not exist yet, so this posts nowhere). The
        // impactor delivers kinetic energy `E = 1/2 * m * U^2`, with `m = (4/3) pi radius^3 * impactor_density`,
        // from THIS event's own data (`impactor` and `flux.impact_velocity_m_s`, both in scope). That energy is
        // the bombardment heating a young, heavily-struck surface carries, and would post to a deep-time heat
        // ledger here. It is deliberately NOT formed or accumulated as a fixed-point value: a real impactor's
        // kinetic energy overflows Q32.32 by many orders (the same reason the crater law returns the ejecta as a
        // mass RATIO and never forms the impactor mass), so the posting awaits the wide-magnitude energy substrate
        // (the log-space / Tier-2 units wiring), a sibling to the assembly binding-energy posting gap already
        // flagged. The hook is this site; the impactor state the energy needs is in scope.
        //
        // The open edge is carried in the TYPE and no longer in this comment alone: `impact_kinetic_energy_j` is
        // the call that ledger will make, and it REFUSES with `ImpactHeatRefusal::UnrepresentableInFixedPoint`,
        // so a consumer meets a stop it must handle rather than an omission it may not notice.
    }

    next
}

/// Form the basin RASTER for one large impact and VALIDATE it before any of it is committed: the conservative
/// [`apply_impact`] delta, checked for the zero sum it promises and for representability in both fields it lands
/// in, returned as the two MOVED fields (the province impact relief and the running ballistic surface), or `None`
/// if either check fails. Returning the moved fields rather than the delta is what makes the commit at the call
/// site an assignment that cannot fail, so the strike is atomic without a second arithmetic argument.
///
/// WHY THE ZERO SUM IS CHECKED HERE rather than trusted: [`apply_impact`] documents that its delta sums to
/// exactly zero (the excavated bowl equals the deposited blanket), and until now only a unit test asserted it, so
/// a kernel that stopped being conservative would have written a mass-creating basin into every later run with
/// nothing at the site to catch it. This is the Residual Law's conservation clause used as a DETECTOR where the
/// quantity moves.
///
/// WHY THE ADDS ARE CHECKED: both fields used `saturating_add`. The support-bound collapse can argue its own
/// saturating add cannot saturate, and the argument is the CAP rather than the magnitudes (every column there is
/// bounded by a representable mass cap). The accumulated impact relief carries no such cap, so no equivalent
/// argument was available here, and one saturated cell would have silently broken the zero sum that makes the
/// bombardment conservative.
fn prepared_basin_raster(
    width: usize,
    height: usize,
    surface_m: &[Fixed],
    source: usize,
    impactor: Impactor,
    flux: &ImpactFluxParams,
    impact_relief_m: &[Fixed],
) -> Option<(Vec<Fixed>, Vec<Fixed>)> {
    let delta = apply_impact(
        width,
        height,
        surface_m,
        source,
        impactor,
        flux.target,
        flux.coupling,
        flux.ejecta,
        flux.forces,
    );
    if delta.len() != impact_relief_m.len() || delta.len() != surface_m.len() {
        return None;
    }
    if delta.iter().map(|&d| d as i128).sum::<i128>() != 0 {
        return None;
    }
    let mut relief = Vec::with_capacity(delta.len());
    let mut surface = Vec::with_capacity(delta.len());
    for (cell, &d) in delta.iter().enumerate() {
        relief.push(Fixed::from_bits(
            impact_relief_m[cell].to_bits().checked_add(d)?,
        ));
        surface.push(Fixed::from_bits(surface_m[cell].to_bits().checked_add(d)?));
    }
    Some((relief, surface))
}

/// WHY AN IMPACT'S HEAT IS NOT POSTED, carried as a TYPE rather than as a comment alone. The bombardment
/// delivers kinetic energy to a young surface and there is no deep-time interior heat ledger for it to post
/// into, so the edge is open at both ends: no sink, and no representable quantity to send. Naming the second
/// half as a refusal keeps a later consumer from assuming the energy was simply not wired up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactHeatRefusal {
    /// The impactor's own data is degenerate (a non-positive radius, density, or closing speed), so no kinetic
    /// energy is defined for it.
    DegenerateImpactor,
    /// The kinetic energy leaves the fixed-point window. This is the ordinary case for a planetary impactor
    /// rather than an edge case, and it is the reason the posting waits: Q32.32 rails near `2.147e9`, while a
    /// kilometre-scale body at a few kilometres per second carries joules past `1e19`. The same magnitude wall
    /// is why the crater law returns its ejecta as a mass RATIO and never forms the impactor mass.
    UnrepresentableInFixedPoint,
}

/// The kinetic energy `E = 1/2 * m * U^2` one impactor delivers (joules), with `m = (4/3) * pi * r^3 * rho` from
/// THIS event's own data, so an alien impactor population is a data row rather than a code path. This is the
/// call a deep-time interior heat ledger will make; it exists now so the open edge is a typed REFUSAL a consumer
/// must handle rather than a comment a consumer may miss.
///
/// It is expected to refuse for any planetary impactor, and it EARNS that refusal from the arithmetic rather
/// than asserting it: the products are formed smallest-first and checked at every stage, so the caller learns
/// the energy is unrepresentable because the multiplication said so. It is not called from [`bombard_tick`],
/// because a value that cannot be formed cannot be accumulated; the posting lands when the wide-magnitude energy
/// substrate does (the log-space / Tier-2 units wiring), a sibling to the assembly binding-energy posting gap.
pub fn impact_kinetic_energy_j(impactor: Impactor) -> Result<Fixed, ImpactHeatRefusal> {
    if impactor.radius <= Fixed::ZERO
        || impactor.density <= Fixed::ZERO
        || impactor.velocity <= Fixed::ZERO
    {
        return Err(ImpactHeatRefusal::DegenerateImpactor);
    }
    let unrepresentable = || ImpactHeatRefusal::UnrepresentableInFixedPoint;
    // E = (2/3) * pi * r^3 * rho * U^2, formed in stages so the first overflow is the one reported.
    let r3 = impactor
        .radius
        .checked_mul(impactor.radius)
        .and_then(|r2| r2.checked_mul(impactor.radius))
        .ok_or_else(unrepresentable)?;
    let mass = r3
        .checked_mul(impactor.density)
        .and_then(|m| m.checked_mul(Fixed::PI))
        .and_then(|m| m.checked_mul(Fixed::from_ratio(4, 3)))
        .ok_or_else(unrepresentable)?;
    let u_squared = impactor
        .velocity
        .checked_mul(impactor.velocity)
        .ok_or_else(unrepresentable)?;
    mass.checked_mul(u_squared)
        .and_then(|e| e.checked_div(Fixed::from_int(2)))
        .ok_or_else(unrepresentable)
}

/// The GPa-to-Pa unit bridge (`1 GPa = 1e9 Pa`), the exact factor the derived operative shear strength (in GPa)
/// converts through to the pascals the support bound reads. A fundamental unit conversion (Principle 11), never
/// an authored world-content value.
const PA_PER_GPA: Fixed = Fixed::from_int(1_000_000_000);

/// The per-world crust MECHANICAL-STRENGTH parameters the support-bound relief collapse reads: the crust and
/// mantle densities the Airy isostasy floats the relief on, the crust's OWN derived shear modulus and the
/// reserved-with-basis per-class strength knockdown its operative yield strength comes through, and the derived
/// surface gravity. Every field is derived or read (Principle 11): a different crust chemistry, a different
/// world, is a different set of numbers through the SAME wiring, never a new code path (admit-the-alien).
#[derive(Clone, Copy, Debug)]
pub struct SupportBoundParams {
    /// The DERIVED crust density (KILOGRAMS PER CUBIC METRE): the load density `rho` in the support bound, the
    /// Airy flotation density the relief floats at, and the density this collapse converts its thickness cap into
    /// a MASS cap through, one value, never authored here.
    ///
    /// It was in grams per cubic centimetre until the ledger landed, and the unit moved for a reason worth
    /// recording: the melt transaction converts through the same physical density in kilograms per cubic metre
    /// ([`MeltParams::crust_density_kg_per_m3`]), and a caller feeding `2.9` here and `2900` there was feeding
    /// two numbers that are equal in physics and NOT equal in fixed point, because `from_ratio(29, 10)` times a
    /// thousand is not exactly `2900`. The crust's thickness would then have been its stock's readout at one
    /// density after growth and at a slightly different one after a collapse, which is the drift a ledger exists
    /// to make impossible. On one unit there is one number. The Airy law is unaffected either way: it reads the
    /// dimensionless ratio `(rho_m - rho_c) / rho_m`, so the unit cancels out of it.
    pub crust_density: Fixed,
    /// The DERIVED mantle density (kilograms per cubic metre) the crust floats on, the Airy reference, in the
    /// same unit as the crust density above and for the same reason.
    pub mantle_density: Fixed,
    /// The crust's OWN DERIVED shear modulus `G` (gigapascals), read from the crust's composition upstream (the
    /// ionic/metal bulk-modulus tier times the reserved-with-basis Pugh ratio). The operative yield strength
    /// derives from it through the Frenkel ideal and the knockdown below, so the bound reads the crust's own
    /// strength, NOT the reserved 1e8 Pa literal.
    pub crust_shear_modulus_gpa: Fixed,
    /// The reserved-with-basis per-class strength KNOCKDOWN (dimensionless, in `(0, 1]`): the ratio of the
    /// operative (measured yield/flow) shear strength to the ideal Frenkel strength `G / (2*pi)`, from mobile
    /// dislocations ([`civsim_materials::properties::operative_shear_strength_gpa`], `~1e-2` for a soft annealed
    /// metal up to `~0.7` for a covalent solid). Reserved-with-basis per bonding class, surfaced not authored.
    pub strength_knockdown: Fixed,
    /// The DERIVED surface gravity `g` (metres per second squared), `g = G M / R^2` from the planet mass and
    /// radius, the gravitational load the topography stands against.
    pub gravity_m_per_s2: Fixed,
}

/// The crust's DERIVED operative yield strength (pascals): the deviatoric strength a crustal column holds before
/// it flows, from the crust's OWN derived shear modulus through the Frenkel ideal shear strength `G / (2*pi)`
/// scaled by the reserved-with-basis per-class knockdown ([`operative_shear_strength_gpa`]), converted to pascals
/// by the [`PA_PER_GPA`] unit bridge. This is the DERIVE-DOWN that RETIRES the reserved 1e8 Pa crustal-yield
/// literal on the collapse path: `sigma_y` reads the crust's composition-grounded strength, never an authored
/// constant. `None` on a non-positive derivation (a shear modulus or knockdown the operative law rejects) or an
/// overflow (an unboundedly stiff crust, which routes to no collapse, the correct outcome, rather than a panic).
fn derived_crust_yield_pa(shear_modulus_gpa: Fixed, knockdown: Fixed) -> Option<Fixed> {
    let operative_gpa = operative_shear_strength_gpa(shear_modulus_gpa, knockdown);
    if operative_gpa <= Fixed::ZERO {
        return None;
    }
    operative_gpa.checked_mul(PA_PER_GPA)
}

/// THE SUPPORT-BOUND COLLAPSE: relax each column's isostatic relief to the mechanical support bound by flowing
/// the excess crust to the lows, mass conserved to the bit. For each column the isostatic relief is its Airy
/// elevation ([`airy_isostatic_elevation`]) above the field DATUM (the mean elevation, [`relief_datum`], the same
/// reference the viewer's relief amplitude reads), and the support bound is `sigma_y / (rho * g)` with `sigma_y`
/// the crust's DERIVED operative yield strength ([`derived_crust_yield_pa`], retiring the reserved 1e8 Pa
/// literal), `rho` the derived crust density, and `g` the derived surface gravity. Because the Airy elevation is
/// linear in the crust thickness (`elevation = k * thickness`, `k = (rho_m - rho_c) / rho_m` the buoyant
/// fraction), the bound maps to a single THICKNESS CAP `T_cap = (bound + datum) / k`: a column thicker than the
/// cap holds unsupportable topography. The excess of every over-cap column FLOWS to the columns below the cap
/// (the accommodation space), apportioned by each low's available room, through the built conservative
/// redistribution ([`redistribute`]), so the crust bit-sum is invariant (mass is moved, never created or lost),
/// the datum is unchanged (the mean is conserved), and every column ends at or below the cap, so all relief ends
/// at or below the bound.
///
/// IT MOVES THE LEDGER, not the thickness. The conserved quantity is [`DeepTimeState::crust_areal_mass`], so the
/// thickness cap is converted to a MASS cap at the emplaced-crust density and the redistribution runs on the
/// stock; each moved column's thickness is then re-read off its moved stock
/// ([`thickness_km_from_areal_mass`]). Running it on the thickness instead would leave a column holding less
/// topography and the same mass, which is the incoherence a ledger exists to make impossible. The source stock is
/// untouched: this is a LATERAL transfer of crust that already exists, never an extraction or a return, so the
/// ledger's crust-plus-source total is invariant across it as well.
///
/// THE CAP IS DERIVED; THE COLLAPSE LAW IS NOT. The ceiling `sigma_y / (rho * g)` reads the crust's own strength
/// and its own gravitational load, so the bound this relaxes TO is derived. HOW it relaxes is an authored
/// limiting idealization standing in for transport physics this module does not carry, and the three things it
/// lacks are stated rather than implied: the whole over-bound excess moves within one tick, so there is no flow
/// RATE; the excess is apportioned across every under-cap column in the field regardless of distance (the
/// destination scan below walks all `n` columns, with no neighbour or distance restriction), so there is no
/// TRANSPORT PATH; and no energy is dissipated and no damage state is accumulated, so a column that has
/// collapsed once is indistinguishable afterwards from one that never did.
///
/// The absence of a rate is a GAP, never a virtue. Under the Residual Law's fluctuation-dissipation clause a
/// relaxation that dissipates with no fluctuation partner is declared INCOMPLETE, and the instantaneous limit is
/// what a reserved-with-basis viscous rate would replace rather than a floor that needs none. A rate (the
/// crust's own effective viscosity against the driving stress), and a LOCAL grid-neighbour (downhill) flow in
/// place of the accommodation-space fill, are the named refinements, flagged not authored. A crust denser
/// than the mantle (`k <= 0`) FOUNDERS rather than standing as topography, a delamination regime this tall-relief
/// collapse does not cover, so it is left unchanged. Call it AFTER [`step_deep_time`] (and after [`bombard_tick`])
/// each tick, so the interior tick stays byte-identical and a run that never collapses replays bit-for-bit.
/// Fail-soft (the state returned unchanged) on a degenerate density, gravity, or yield, a relief already inside
/// the bound (nothing to relax), or a redistribution refusal; never a panic and never a fabricated relief.
/// Deterministic and worker-invariant (a pure function of the state and the parameters).
pub fn relax_to_support_bound(state: &DeepTimeState, params: &SupportBoundParams) -> DeepTimeState {
    let n = state.crust_thickness_km.len();
    // The collapse moves the LEDGER, so it needs a crust stock covering every column; a mismatch is left
    // unchanged rather than partly moved.
    if n == 0 || state.crust_areal_mass.len() != n {
        return state.clone();
    }
    // The Airy buoyant fraction k = (rho_m - rho_c) / rho_m, the elevation a unit of crust thickness stands at.
    // A non-positive mantle density, or a crust at or above the mantle density (k <= 0, a foundering column), has
    // no standing topography to collapse: the support bound is a TALL-relief mechanism, so leave it unchanged.
    if params.mantle_density <= Fixed::ZERO {
        return state.clone();
    }
    let contrast = match params.mantle_density.checked_sub(params.crust_density) {
        Some(c) if c > Fixed::ZERO => c,
        _ => return state.clone(),
    };
    let k = match contrast.checked_div(params.mantle_density) {
        Some(k) if k > Fixed::ZERO => k,
        _ => return state.clone(),
    };

    // The DERIVED yield strength (pascals), the reserved 1e8 literal retired on this path.
    let sigma_y_pa =
        match derived_crust_yield_pa(params.crust_shear_modulus_gpa, params.strength_knockdown) {
            Some(s) => s,
            None => return state.clone(),
        };
    // The support bound sigma_y / (rho * g): rho already in kg/m^3, g in m/s^2, the bound in metres, converted
    // to the kilometres the crust thickness and relief carry.
    if params.gravity_m_per_s2 <= Fixed::ZERO {
        return state.clone();
    }
    let bound_km = match params
        .crust_density
        .checked_mul(params.gravity_m_per_s2)
        .filter(|rho_g| *rho_g > Fixed::ZERO)
        .and_then(|rho_g| sigma_y_pa.checked_div(rho_g))
        .and_then(|bound_m| bound_m.checked_div(M_PER_KM))
    {
        Some(b) if b > Fixed::ZERO => b,
        _ => return state.clone(),
    };

    // The field DATUM (mean isostatic elevation), the reference the relief is measured against. elevation_i =
    // airy(rho_c, rho_m, thickness_i); a non-flotation input (a non-positive mantle density) is already guarded.
    let mut elevations = Vec::with_capacity(n);
    for t in &state.crust_thickness_km {
        match airy_isostatic_elevation(params.crust_density, params.mantle_density, *t) {
            Some(e) => elevations.push(e),
            None => return state.clone(),
        }
    }
    let datum = match relief_datum(&elevations) {
        Some(d) => d,
        None => return state.clone(),
    };
    // The support CAP on thickness: the thickness whose relief equals the bound. relief = k*T - datum = bound, so
    // T_cap = (bound + datum) / k. A column above the cap sheds its excess; a column below has room to receive.
    let t_cap = match bound_km
        .checked_add(datum)
        .and_then(|numer| numer.checked_div(k))
    {
        Some(t) => t,
        None => return state.clone(),
    };

    // THE CAP IN MASS. The conserved quantity is the crust stock, so the thickness cap is converted once to the
    // areal mass a column of that thickness carries, and the whole redistribution runs on the stock. The density
    // is read straight off the params, in the same kilograms per cubic metre the melt transaction emplaces at, so
    // the two conversions are the same number and not two roundings of one.
    let rho_kg = params.crust_density;
    let m_cap = match areal_mass_mg_per_m2(t_cap, rho_kg) {
        Some(m) => m,
        None => return state.clone(),
    };

    // The destinations: every under-cap column, weighted by its available room (the deficit to the cap) in raw
    // field bits, so the shed crust flows preferentially into the deepest lows and no receiver overshoots the cap
    // (each receiver's share of the excess is at most its room, since the total excess is strictly below the
    // total room for a mean-centred relief field). The weight ratios are what redistribute reads.
    let mut dests: Vec<Weighted> = Vec::new();
    for (i, m) in state.crust_areal_mass.iter().enumerate() {
        if let Some(deficit) = m_cap.checked_sub(*m) {
            let bits = deficit.to_bits();
            if deficit > Fixed::ZERO && bits > 0 {
                dests.push(Weighted {
                    dest: i,
                    weight: bits as u64,
                });
            }
        }
    }
    // The sources: every over-cap column sheds its excess crust MASS (in raw field bits) across the shared
    // accommodation-space destinations. INSTANTANEOUS collapse: the whole excess moves this tick.
    let mut moves: Vec<Redistribution> = Vec::new();
    for (i, m) in state.crust_areal_mass.iter().enumerate() {
        if let Some(excess) = m.checked_sub(m_cap) {
            let bits = excess.to_bits();
            if excess > Fixed::ZERO && bits > 0 {
                moves.push(Redistribution {
                    source: i,
                    mass: bits,
                    dests: dests.clone(),
                });
            }
        }
    }
    // Nothing over the bound: the relief is already supportable, so the term is a no-op (dormant), never a move.
    if moves.is_empty() {
        return state.clone();
    }

    // The conservative delta (raw field bits, summing to exactly zero). A refusal (no destination for a source, an
    // overflow) falls soft to the unchanged state rather than dropping or fabricating crust.
    let delta = match redistribute(n, &moves) {
        Ok(d) => d,
        Err(_) => return state.clone(),
    };
    let mut next = state.clone();
    for (i, d) in delta.iter().enumerate() {
        if *d != 0 {
            // The bit-space move: the crust STOCK is nudged by the conservative delta in its own raw bits, the
            // same bit-arithmetic redistribution the bombardment applies. The delta sums to zero, so the crust
            // mass bit-sum is invariant (mass moved, never created or lost). The saturating add cannot saturate
            // here, and the reason is the cap rather than the magnitudes: every receiver's share is at most its
            // own room to the cap and every source only sheds, so no column leaves the range `[0, m_cap]`, whose
            // upper end is a representable `Fixed` by construction. The conservation is therefore exact.
            next.crust_areal_mass[i] =
                Fixed::from_bits(next.crust_areal_mass[i].to_bits().saturating_add(*d));
            // And the geometry follows the ledger: the thickness is re-read off the moved stock, never moved
            // independently of it. A refusal holds the previous thickness rather than reporting relief the stock
            // does not back.
            if let Some(t) = thickness_km_from_areal_mass(next.crust_areal_mass[i], rho_kg) {
                next.crust_thickness_km[i] = t;
            }
        }
    }
    next
}

/// The star-aging parameters the deep-time run reads to derive the main-sequence brightening: the star's ZAMS
/// ANCHOR (its zero-age luminosity ratio and its mass ratio, both the stellar front-end's outputs at age 0), plus
/// the two reserved-with-basis constants of the nuclear-timescale-and-brightening law. The mechanism derives the
/// lifetime and the current luminosity from these four values; nothing here is an authored brightening curve. Both
/// constants are per-star DATA (a field, not a hardcoded inline value), so a world's star with different nuclear
/// physics is a data row, never a rewrite (admit-the-alien).
#[derive(Clone, Copy, Debug)]
pub struct StarAgingParams {
    /// `L_zams / L_sun`, the star's ZERO-AGE main-sequence luminosity ratio: the stellar front-end's output at age
    /// 0 ([`crate::stellar::luminosity_ratio`]), the anchor the brightening climbs from and recovers at age 0.
    pub zams_luminosity_ratio: Fixed,
    /// `M / M_sun`, the star's mass ratio: the FUEL term of the nuclear timescale (hydrogen fuel scales with mass).
    pub mass_ratio: Fixed,
    /// The SOLAR nuclear timescale (megayears): `t_sun`, the fuel-over-luminosity constant that sets the Sun's
    /// main-sequence lifetime, so every star's lifetime scales from it. Reserved-with-basis (the basis: the Sun's
    /// hydrogen fuel divided by its luminosity, the classic nuclear-timescale figure of order ten gigayears, cited
    /// rather than fabricated). Surfaced, not set here.
    pub solar_nuclear_timescale_myr: Fixed,
    /// The Gough brightening COEFFICIENT (dimensionless): the fraction the rising core mean-molecular-weight drives
    /// the luminosity up over a full main-sequence lifetime, expected positive (a main-sequence star brightens).
    /// Reserved-with-basis (the basis: the Gough 1981 solar-luminosity form `L(t)/L_zams = 1 / (1 - c * t/t_ms)`,
    /// with `c` of order 0.4 reproducing the ~30 to 40 percent zero-age-to-present solar brightening; per-star so a
    /// differently structured star can carry its own). Surfaced, not set here.
    pub brightening_coefficient: Fixed,
}

/// The star's main-sequence LIFETIME (megayears), DERIVED from the nuclear timescale: `t_ms = t_sun * (M/M_sun) /
/// (L_zams/L_sun)`, the fuel (proportional to mass) divided by the burn rate (the luminosity), anchored to the
/// Sun's `t_sun`. Because the ZAMS luminosity climbs steeply with mass (the mass-luminosity relation the stellar
/// front-end derives, `L ~ M^alpha` with `alpha ~ 3.5`), a heavier star has a SHORTER main-sequence life: it holds
/// more fuel but burns it far faster, so the lifetime shrinks as `M / M^alpha`. The lifetime is thus not authored:
/// it emerges from the mass and the ZAMS luminosity the stellar model already supplies, times the one reserved
/// nuclear-timescale anchor. `None` if the mass or the ZAMS luminosity ratio is non-positive (no fuel, or no burn
/// rate to divide by), or the product overflows.
pub fn main_sequence_lifetime_myr(aging: &StarAgingParams) -> Option<Fixed> {
    if aging.mass_ratio <= Fixed::ZERO || aging.zams_luminosity_ratio <= Fixed::ZERO {
        return None;
    }
    aging
        .solar_nuclear_timescale_myr
        .checked_mul(aging.mass_ratio)
        .and_then(|fuel| fuel.checked_div(aging.zams_luminosity_ratio))
}

/// The star's CURRENT luminosity ratio `L(t)/L_sun` at a main-sequence age `star_age_myr`, DERIVED from the ZAMS
/// anchor and the Gough brightening: `L(t)/L_sun = (L_zams/L_sun) / (1 - c * t/t_ms)`, with `t` the star's age,
/// `t_ms` its derived main-sequence lifetime ([`main_sequence_lifetime_myr`]), and `c` the Gough brightening
/// coefficient. As core hydrogen fuses to helium the core's mean molecular weight rises, the core contracts and
/// heats, and the luminosity climbs; the burnt fraction `f = t/t_ms` drives the climb, so the brightening EMERGES
/// from the age against the derived lifetime, never an authored curve. At age 0 the factor is exactly one, so the
/// ZAMS anchor is recovered. `None` once `t >= t_ms`: the star has exhausted its core hydrogen and LEAVES the main
/// sequence, the post-main-sequence giant branch being a separate stellar-evolution arc, capped here rather than
/// extrapolating a regime this law does not describe. Also `None` on a negative age, an undefined lifetime, or a
/// coefficient so large the denominator would hit zero before `t_ms` (a divergent regime the first-grade law does
/// not cover).
pub fn current_luminosity_ratio(aging: &StarAgingParams, star_age_myr: Fixed) -> Option<Fixed> {
    if star_age_myr < Fixed::ZERO {
        return None;
    }
    let t_ms = main_sequence_lifetime_myr(aging)?;
    if t_ms <= Fixed::ZERO || star_age_myr >= t_ms {
        // The star has left the main sequence (or has no defined lifetime): the post-main-sequence arc, not here.
        return None;
    }
    let fraction = star_age_myr.checked_div(t_ms)?; // f = t/t_ms, in [0, 1)
    let drop = aging.brightening_coefficient.checked_mul(fraction)?; // c * f
    let denom = Fixed::ONE.checked_sub(drop)?; // 1 - c * f
    if denom <= Fixed::ZERO {
        // A coefficient large enough to zero the denominator before t_ms diverges: outside the first-grade law.
        return None;
    }
    aging.zams_luminosity_ratio.checked_div(denom)
}

/// The number of convection PROVINCES that span a lateral distance, DERIVED from the convective cell
/// width. Rayleigh-Benard convection cells have a horizontal width of order the convecting-layer DEPTH
/// (an aspect ratio of order one), so the province width is `mantle_depth_m * cell_aspect` and the count
/// spanning `span_m` is `span_m / width`, floored, and at least one. The lateral province SCALE is thus
/// DERIVED from the convective physics, never a hand-set grid size: a deeper mantle makes fewer, wider
/// provinces and a larger planet more of them, so the texture's spatial scale is what the convection IS.
/// `cell_aspect` (the convective cell aspect ratio) is the CALLER's reserved-with-basis value (its basis:
/// the Rayleigh-Benard critical-mode cell aspect, order one, set by the mantle's convective boundary
/// regime), threaded in, never authored here. `None` on a non-physical input (a non-positive span, depth,
/// or aspect, or an overflow).
pub fn provinces_across(span_m: Fixed, mantle_depth_m: Fixed, cell_aspect: Fixed) -> Option<usize> {
    if span_m <= Fixed::ZERO || mantle_depth_m <= Fixed::ZERO || cell_aspect <= Fixed::ZERO {
        return None;
    }
    let width = mantle_depth_m.checked_mul(cell_aspect)?;
    let count = span_m.checked_div(width)?;
    Some(count.to_int().max(1) as usize)
}

/// The rigid-rigid Rayleigh-Benard marginal-stability CRITICAL RAYLEIGH NUMBER, the onset threshold for a mantle
/// layer bounded by rigid (no-slip) top and bottom. Paired with [`RIGID_RIGID_CRITICAL_WAVENUMBER`] as ONE
/// boundary regime: the classical rigid-rigid eigenvalue is {Ra_crit ~ 1707.76, a_c ~ 3.117}. This is an
/// analytic eigenvalue, not an authored knob (Chandrasekhar 1961, Hydrodynamic and Hydromagnetic Stability, ch.
/// II). A future marginal-stability solver would derive {Ra_crit, a_c, regime} jointly from the mantle's actual
/// mechanical boundaries; here the two are the cited rigid-rigid pair, kept together so the onset and the cell
/// scale can never drift into different regimes.
///
/// PINNED, exactly one uncompared instance (owner ruling 2026-07-18). The cited value lives once, in the
/// `convection_scaling.toml` rigid-rigid row; this const carries the exact 1707.762 and the sentinel test
/// `the_rigid_rigid_eigenvalue_is_the_one_cited_row` asserts it equals that row. Corrected from a rounded
/// `from_int(1708)`, a fourth-digit diamond that sat in the path of every convection-onset verdict and agreed
/// with no test because each compared against itself; it moves the deep-time onset by 0.014 percent.
pub const RIGID_RIGID_RA_CRIT: Fixed = Fixed::from_int(1_707_762).div(Fixed::from_int(1000));

/// The rigid-rigid Rayleigh-Benard marginal-stability CRITICAL WAVENUMBER a_c (inverse layer depths), the pair
/// mate of [`RIGID_RIGID_RA_CRIT`]. The horizontal mode that goes unstable first has a_c ~ 3.117 for rigid
/// boundaries (versus a_c = pi/sqrt(2) ~ 2.221 for free-free), so the convecting-cell half-wavelength is `pi /
/// a_c` ~ 1.008 layer depths (versus ~1.414 free-free). The province lateral SCALE reads `pi / a_c` for the cell
/// aspect, so the aspect and the onset threshold are the SAME regime by construction. Cited: Chandrasekhar (1961).
pub const RIGID_RIGID_CRITICAL_WAVENUMBER: Fixed = Fixed::from_int(3117).div(Fixed::from_int(1000));

/// `ln(1e18)`, the bridge between an SI viscosity and the `1e18 Pa*s` unit the megametre-depth Rayleigh
/// number is dimensionally consistent with. Carried as a logarithm because the SI value it converts from
/// (`~5.5e23 Pa*s`) cannot be held at all.
/// `ln(3.15576e13)`, the seconds in a megayear (`31_557_600 s/yr` times `1e6`). A logarithm because the
/// value itself is four orders past `Fixed::MAX`, so the tick length can never be held linearly in seconds.
///
/// CORRECTED 2026-07-19 from `31.083129`, an audit finding. The true value is `31.0828355634`, so every
/// thermal tick was running `0.0293 percent` long. Small, and wrong in a quantity that multiplies every
/// energy in the deep-time run, which is exactly the kind of error that never announces itself.
const LN_SECONDS_PER_MYR: Fixed = Fixed::from_int(310_828_356).div(Fixed::from_int(10_000_000));

/// The Nusselt prefactor `a` for a PURELY INTERNALLY HEATED interior, derived rather than chosen.
///
/// The deep-time column carries a radiogenic `heat_production` and no basal core-flux term, so its
/// internal-heating fraction is 1 and `civsim_physics::convection_scaling` gives the symmetric
/// two-boundary-layer endpoint `2^(-4/3)`, about 0.397. Reading it from the scaling table rather than
/// writing 0.397 here keeps the one home the residue ruling established: the day this model gains a basal
/// flux term, the fraction becomes a real derived quantity and this call site changes by passing it.
// @derives: the deep-time Nusselt prefactor <- the convection-scaling band at the model's own internal-heating fraction
fn nusselt_prefactor_internally_heated() -> Option<Fixed> {
    civsim_physics::convection_scaling::ConvectionScaling::standard()
        .ok()?
        .nusselt_prefactor_at_internal_fraction(Fixed::ONE)
}

/// One province's [`SiColumnParams`], composed entirely from DERIVED inputs.
///
/// # What this used to be
///
/// It returned a scaled operating point carrying a tagged FIXTURE CLUSTER: `density = 1`,
/// `thermal_conductivity = 2`, `thermal_expansion_ppm = 30`, `thermal_diffusivity = 0.01`,
/// `specific_heat = 10`, `viscosity = 1`, `radius = 1`, stepped with a dimensionless `dt = 1`. Its own
/// comment recorded that the stored diffusivity disagreed with `k / (rho c_p)` by twentyfold and that
/// nothing compared them, and declared the conflict rather than resolving it: "the geotherm arc REPLACES
/// the whole cluster, and the pins move ONCE then, with a ledger entry."
///
/// This is that replacement. Every field is now the column's own derived property, and the redundancy is
/// gone by construction rather than by agreement: the diffusivity is computed from the three facts it is
/// made of instead of stored beside them.
///
/// # Two clocks became one
///
/// The retired kernel advanced convection by a dimensionless tick while the crust grew by `dt_myr`, so a
/// single stepper ran two unrelated clocks with nothing relating them. That was invisible while the
/// thermal values were fixtures, because no rate in the system was in real units. `dt_myr` now sets both:
/// the tick length enters as `ln_dt_s` and the heat terms are per-tick ENERGIES on that same clock.
///
/// # Why the tick is a logarithm
///
/// One megayear is `3.1557e13 s` against a `Fixed::MAX` of `2.1e9`, so the tick length cannot be held in
/// seconds at all, and a 20 Myr tick is `6.3e14`. It is carried as `ln(dt)` and never exponentiated. The
/// radiogenic budget has the mirror problem in the other direction: `~5e-12 W/kg` is `0.02` of one `Fixed`
/// ulp and quantizes to exactly zero as a rate, so it is carried as the energy that rate delivers over one
/// tick, where it is a comfortable `~340 J/kg`.
///
/// `heat_production_j_per_kg` is the per-province radiogenic ENERGY per tick, whose lateral spread is what
/// makes the provinces diverge; `reference_temperature_k` is the cold reference the contrast is measured
/// against. Returns `None` when an input is non-physical or a composition is unrepresentable, which
/// refuses the whole province field rather than falling back to the retired cluster. Deterministic.
// @derives: one province's SI convection column <- the derived thermal cluster, the planet's own depth and gravity, and the per-province radiogenic energy
pub fn province_column_params(
    convecting_depth_mm: Fixed,
    surface_gravity_m_s2: Fixed,
    heat_production_j_per_kg: Fixed,
    reference_temperature_k: Fixed,
    dt_myr: Fixed,
    derived: &crate::geodynamics::ColumnThermalProperties,
) -> Option<SiColumnParams> {
    let depth_m = convecting_depth_mm.checked_mul(Fixed::from_int(1_000_000))?;
    // ln(dt in seconds) = ln(dt_myr) + ln(3.1557e13). Composed in logs because the product is `6.3e14`
    // for a 20 Myr tick, five orders past `Fixed::MAX`, and the tick length is an INPUT so no reordering
    // of a linear form would rescue it.
    let ln_dt_s = dt_myr.ln().checked_add(LN_SECONDS_PER_MYR)?;
    Some(SiColumnParams {
        reference_temperature_k,
        // THE FIXTURE CLUSTER IS RETIRED. These five were `density = 1`, `thermal_conductivity = 2`,
        // `thermal_expansion_ppm = 30`, `thermal_diffusivity = 0.01` and `specific_heat = 10`: a DECLARED
        // CONFLICT rather than a parameterization, whose own tag recorded that the stored diffusivity
        // disagreed with `k / (rho c_p)` by twentyfold with nothing comparing them. They are the column's
        // DERIVED properties now, minimized from the world's own composition at its own state, and the
        // redundancy is gone by construction: the diffusivity is computed from the three facts it is made
        // of rather than stored beside them.
        density_kg_m3: derived.density_kg_m3,
        thermal_conductivity_w_m_k: derived.thermal_conductivity_w_m_k,
        thermal_expansion_ppm: derived.thermal_expansion_ppm_per_k,
        specific_heat_j_kg_k: derived.specific_heat_j_kg_k,
        thermal_diffusivity_m2_s: derived.thermal_diffusivity()?,
        // Never exponentiated: the SI value is `~5e23 Pa*s`.
        ln_viscosity: derived.viscosity.ln_viscosity_primary,
        gravity_m_s2: surface_gravity_m_s2,
        depth_m,
        // The buoyant parcel scale, DERIVED from the column's own convective cell rather than the former
        // `radius = 1` fixture: `pi / a_c` layer depths, the rigid-rigid critical mode's half-wavelength.
        parcel_radius_m: Fixed::PI
            .checked_div(RIGID_RIGID_CRITICAL_WAVENUMBER)
            .and_then(|x| x.checked_mul(depth_m))?,
        heat_production_j_per_kg,
        ln_rayleigh_critical: RIGID_RIGID_RA_CRIT.ln(),
        ln_dt_s,
        // THE NUSSELT PREFACTOR, DERIVED from the world's own heating configuration rather than authored.
        //
        // This was `Fixed::ONE` with a comment claiming it was "the value convection_scaling carries for
        // this regime". Both halves were wrong, and an audit caught it. `1.0` is the BASAL endpoint, the
        // single-boundary-layer case at internal-heating fraction `f = 0`, which is the OPPOSITE of this
        // model's regime; and the value is not carried as a constant at all, it is derived by
        // `nusselt_prefactor_at_internal_fraction`, which the owner's residue ruling of 2026-07-18 added
        // precisely so the convention selects on the world's state instead of being picked.
        //
        // The deep-time interior is PURELY INTERNALLY HEATED: it carries `heat_production` and no basal
        // core-flux term, so `f = 1` and the prefactor is the symmetric two-boundary-layer endpoint
        // `2^(-4/3)`, about `0.397`. The module's own documentation warns that using `1.0` here cools the
        // interior about 2.5 times too fast, which is a factor sitting directly on the deep-time evolution
        // the province texture emerges from.
        nusselt_prefactor: nusselt_prefactor_internally_heated()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The emplaced-crust density (kg per cubic metre) for the scaffolds, a representative mafic crust. It lives
    /// ONCE here because the melt transaction and the support-bound collapse both convert through it (in kg/m^3
    /// and in g/cm^3 respectively), and a scaffold that stated it twice could pass while disagreeing with itself
    /// about what a kilometre of crust weighs, which is the one thing the ledger exists to make impossible.
    const CRUST_DENSITY_KG_M3: i32 = 2900;
    /// The mantle density (kg per cubic metre) the crust floats on and the melt source is drawn from.
    const MANTLE_DENSITY_KG_M3: i32 = 3300;
    /// The convecting-mantle depth (kilometres) the scaffold's source stock is derived over, Mars-class.
    const CONVECTING_DEPTH_KM: i32 = 1500;

    /// The scaffold's initial SOURCE STOCK, through the ledger's own unit bridge: a Mars-class convecting mantle
    /// (1500 km at 3300 kg per cubic metre) is `4.95e6` megagrams per square metre. Derived here rather than
    /// written down, so the scaffold starts where the caller's derived interior structure would put it.
    fn source_stock() -> Fixed {
        areal_mass_mg_per_m2(
            Fixed::from_int(CONVECTING_DEPTH_KM),
            Fixed::from_int(MANTLE_DENSITY_KG_M3),
        )
        .expect("a Mars-class mantle column is representable in megagrams per square metre")
    }

    /// A fresh planet for the scaffolds: `n` uniform columns at `temperature_k`, no crust, and the derived source
    /// stock above under every column.
    fn young_state(n: usize, temperature_k: i32) -> DeepTimeState {
        DeepTimeState::young(n, Fixed::from_int(temperature_k), source_stock())
    }

    /// A representative DERIVED thermal cluster for the deep-time scaffolds, at the magnitudes a
    /// Mars-class forsterite column derives. Constructed rather than derived because these tests
    /// vary ONE input (the radiogenic budget) and should not carry a full assemblage minimization.
    fn test_cluster() -> crate::geodynamics::ColumnThermalProperties {
        crate::geodynamics::ColumnThermalProperties {
            density_kg_m3: Fixed::from_ratio(33545, 10),
            thermal_conductivity_w_m_k: Fixed::from_ratio(2461, 1000),
            specific_heat_j_kg_k: Fixed::from_int(1241),
            thermal_expansion_ppm_per_k: Fixed::from_ratio(259, 10),
            viscosity: civsim_physics::convective_viscosity::ViscosityBand {
                ln_viscosity_min: Fixed::from_ratio(5466, 100),
                ln_viscosity_max: Fixed::from_ratio(5466, 100),
                ln_viscosity_primary: Fixed::from_ratio(5466, 100),
                eval_temperature_k: Fixed::from_int(1600),
                eval_pressure_gpa: Fixed::from_ratio(1117, 100),
            },
        }
    }

    /// The realization digest must be REPRODUCIBLE: the same realized state folds to the same receipt. This is
    /// the property a physics baseline pin rests on, and it is the one the Chaos Protocol makes available for a
    /// seeded draw (the same seed through the same measure realizes the same world).
    #[test]
    fn the_realization_digest_reproduces_on_an_identical_state() {
        let a = young_state(6, 1600);
        let b = young_state(6, 1600);
        assert_eq!(
            a.realization_digest(),
            b.realization_digest(),
            "an identical realized state must fold to an identical receipt"
        );
        let other = young_state(6, 1601);
        assert_ne!(
            a.realization_digest(),
            other.realization_digest(),
            "a different initial temperature is a different world"
        );
    }

    /// A receipt that silently omits a field cannot catch a regression in that field, and its passing would be
    /// worse than no receipt at all: it would read as coverage while proving nothing. So every field the state
    /// carries is perturbed in turn and the digest must move for each. A field added to DeepTimeState without
    /// being folded into the digest fails here, which is the point.
    #[test]
    fn the_realization_digest_covers_every_field_it_claims_to() {
        let base = young_state(4, 1600);
        let d0 = base.realization_digest();

        let mut s = base.clone();
        s.elapsed_myr = Fixed::from_int(7);
        assert_ne!(d0, s.realization_digest(), "elapsed_myr is not covered");

        let mut s = base.clone();
        s.star_age_start_myr = Fixed::from_int(9);
        assert_ne!(
            d0,
            s.realization_digest(),
            "star_age_start_myr is not covered"
        );

        let mut s = base.clone();
        s.impact_count += 1;
        assert_ne!(d0, s.realization_digest(), "impact_count is not covered");

        // The bombardment's two RESIDUAL counts. A world owing withheld strikes is mid-bombardment, and a world
        // carrying unrealized events has a reservoir and a crater law that do not meet; neither is the same
        // world as one whose bombardment closed cleanly at the same crater count.
        let mut s = base.clone();
        s.deferred_impacts += 1;
        assert_ne!(
            d0,
            s.realization_digest(),
            "deferred_impacts is not covered"
        );

        let mut s = base.clone();
        s.unrealized_impacts += 1;
        assert_ne!(
            d0,
            s.realization_digest(),
            "unrealized_impacts is not covered"
        );

        let mut s = base.clone();
        s.columns[1].temperature = Fixed::from_int(1700);
        assert_ne!(
            d0,
            s.realization_digest(),
            "a column temperature is not covered"
        );

        let mut s = base.clone();
        s.columns[1].convecting = !s.columns[1].convecting;
        assert_ne!(
            d0,
            s.realization_digest(),
            "the convection onset latch is not covered"
        );

        let mut s = base.clone();
        s.crust_thickness_km[2] = Fixed::from_int(30);
        assert_ne!(
            d0,
            s.realization_digest(),
            "the derived crust thickness is not covered"
        );

        let mut s = base.clone();
        s.impact_relief_m[3] = Fixed::from_int(500);
        assert_ne!(
            d0,
            s.realization_digest(),
            "the basin impact relief is not covered"
        );

        let mut s = base.clone();
        s.craters.push(CraterRow {
            u: Fixed::from_ratio(1, 4),
            v: Fixed::from_ratio(1, 3),
            diameter_m: Fixed::from_int(1200),
            depth_m: Fixed::from_int(200),
            age_myr: Fixed::from_int(11),
        });
        assert_ne!(
            d0,
            s.realization_digest(),
            "the crater rows are not covered"
        );

        let mut s = base.clone();
        s.crust_areal_mass[2] = Fixed::from_int(87_000);
        assert_ne!(
            d0,
            s.realization_digest(),
            "the crust areal-mass stock is not covered"
        );

        let mut s = base.clone();
        s.source_areal_mass[1] = Fixed::from_int(1_000_000);
        assert_ne!(
            d0,
            s.realization_digest(),
            "the source areal-mass stock is not covered"
        );
    }

    /// The vectors are POSITIONAL, one entry per lateral cell, so the receipt must distinguish two worlds that
    /// hold the same values in different cells. A digest that sorted or otherwise discarded index order would
    /// call a hot pole and a hot equator the same planet.
    #[test]
    fn the_realization_digest_is_positional_not_a_multiset() {
        let mut a = young_state(4, 1600);
        let mut b = young_state(4, 1600);
        a.crust_thickness_km[0] = Fixed::from_int(40);
        b.crust_thickness_km[3] = Fixed::from_int(40);
        assert_ne!(
            a.realization_digest(),
            b.realization_digest(),
            "the same values in different cells are different worlds"
        );
    }

    #[test]
    fn the_rigid_rigid_eigenvalue_is_the_one_cited_row() {
        // ONE UNCOMPARED INSTANCE (owner ruling 2026-07-18): the rigid-rigid critical Rayleigh lives once, in the
        // cited convection_scaling.toml row that convection onset and the boundary layer both read through
        // ColumnParams::ra_crit. This const carries the same value and is pinned to that row here, so a rounding
        // diamond like the retired from_int(1708) cannot slip back in agreeing only with itself.
        let cited = civsim_physics::convection_scaling::ConvectionScaling::standard()
            .expect("convection_scaling.toml is vendored")
            .critical_rayleigh(civsim_physics::convection_scaling::BoundaryCondition::RigidRigid)
            .expect("the rigid-rigid row is present");
        assert!(
            (RIGID_RIGID_RA_CRIT - cited).abs() < Fixed::from_ratio(1, 100),
            "RIGID_RIGID_RA_CRIT {} must equal the cited convection_scaling rigid-rigid row {}",
            RIGID_RIGID_RA_CRIT.to_f64_lossy(),
            cited.to_f64_lossy()
        );
    }

    // A representable-scaled mantle column parameter set, mirroring the convection tests: the values are the
    // engine-scaled illustrative mantle (depth and viscosity are the representable-scaled forms the convection
    // kernel documents), enough to exercise the deep-time evolution deterministically. `ra_crit` high keeps a
    // column conductive so its relaxation is monotone and easy to assert; `heat_production` is the per-column
    // knob the lateral-variation test varies.
    fn mantle_params(heat_production_j_per_kg: i32) -> SiColumnParams {
        // Built through the real constructor from the DERIVED cluster, so these lateral-variation tests run
        // the same path production does. The old version hand-wrote the fixture cluster (`density = 1`,
        // `specific_heat = 10`, a `thermal_diffusivity` twentyfold off `k/(rho c_p)`), which meant it could
        // not have caught the fixture's own incoherence.
        province_column_params(
            Fixed::from_ratio(15, 10),
            Fixed::from_ratio(37, 10),
            Fixed::from_int(heat_production_j_per_kg),
            Fixed::from_int(300),
            Fixed::from_int(20),
            &test_cluster(),
        )
        .expect("the scaffold column resolves")
    }

    // The melt-and-crust parameters, the McKenzie-Bickle peridotite values the seam-6 melt-column test uses
    // (solidus 1373 K, 130 K/GPa; adiabat 15.5 K/GPa; productivity 0.12/GPa; source 3300 kg/m^3; g ~10), plus a
    // 100 Myr processing time. A column above the ~1373 K surface solidus makes crust; a cooled sub-solidus one
    // makes none.
    fn melt_params() -> MeltParams {
        MeltParams {
            solidus_surface_k: Fixed::from_int(1373),
            solidus_slope_k_per_gpa: Fixed::from_int(130),
            adiabat_slope_k_per_gpa: Fixed::from_ratio(155, 10),
            productivity_per_gpa: Fixed::from_ratio(12, 100),
            source_density_kg_per_m3: Fixed::from_int(MANTLE_DENSITY_KG_M3),
            crust_density_kg_per_m3: Fixed::from_int(CRUST_DENSITY_KG_M3),
            gravity_m_per_s2: Fixed::from_int(10),
            processing_time_myr: Fixed::from_int(100),
        }
    }

    // The star-aging parameters. The two constants are ILLUSTRATIVE stand-ins for the owner's reserved values,
    // chosen at the physically-anchored figures only to exercise the mechanism: `t_sun` = 10 Gyr (10,000 Myr, the
    // solar nuclear timescale) and `c` = 0.4 (the Gough 1981 coefficient). The ZAMS luminosity and mass ratios are
    // the per-star anchors the caller feeds from the stellar front-end.
    fn aging_params(zams_luminosity_ratio: Fixed, mass_ratio: Fixed) -> StarAgingParams {
        StarAgingParams {
            zams_luminosity_ratio,
            mass_ratio,
            solar_nuclear_timescale_myr: Fixed::from_int(10_000),
            brightening_coefficient: Fixed::from_ratio(4, 10),
        }
    }

    #[test]
    fn the_star_brightens_monotonically_over_its_main_sequence_life() {
        // The core burns hydrogen, its mean molecular weight rises, and the luminosity climbs: at every later age
        // the star is brighter than before, the surface-warming drive of the deep-time run.
        let star = aging_params(Fixed::ONE, Fixed::ONE); // Sun-like: t_ms = 10,000 Myr
        let mut prev = current_luminosity_ratio(&star, Fixed::ZERO).expect("on the main sequence");
        for age_myr in [500, 1000, 2000, 4000, 6000, 8000, 9000, 9500, 9900] {
            let l = current_luminosity_ratio(&star, Fixed::from_int(age_myr))
                .expect("still on the main sequence before t_ms");
            assert!(
                l > prev,
                "the star brightens as it ages: at {age_myr} Myr L/L_sun = {} was not above the prior {}",
                l.to_f64_lossy(),
                prev.to_f64_lossy()
            );
            prev = l;
        }
    }

    #[test]
    fn the_zams_anchor_is_recovered_at_age_zero() {
        // At age 0 the Gough factor is exactly one, so the current luminosity is the stellar front-end's ZAMS
        // output unchanged: the brightening climbs FROM the anchor, it does not replace it.
        let star = aging_params(Fixed::from_ratio(7, 4), Fixed::from_ratio(6, 5));
        assert_eq!(
            current_luminosity_ratio(&star, Fixed::ZERO),
            Some(Fixed::from_ratio(7, 4)),
            "at age 0 the ZAMS anchor is recovered exactly"
        );
    }

    #[test]
    fn a_more_massive_star_leaves_the_main_sequence_faster() {
        // t_ms ~ M / L and L climbs steeply with M (the mass-luminosity relation the stellar front-end derives), so
        // a heavier star holds more fuel but burns it far faster and leaves the main sequence sooner. The ZAMS
        // luminosities come from the real stellar model to show the derivation composes end to end.
        use crate::stellar::luminosity_ratio;
        let alpha = Fixed::from_ratio(35, 10); // illustrative mass-luminosity slope (~3.5), as the stellar tests use
        let lambda = Fixed::ZERO; // solar metallicity: the metallicity factor is one
        let sun_l = luminosity_ratio(Fixed::ONE, Fixed::ONE, alpha, lambda).expect("sun L");
        let heavy_l =
            luminosity_ratio(Fixed::from_int(2), Fixed::ONE, alpha, lambda).expect("heavy L");
        let sun = aging_params(sun_l, Fixed::ONE);
        let heavy = aging_params(heavy_l, Fixed::from_int(2));
        let sun_life = main_sequence_lifetime_myr(&sun).expect("sun lifetime");
        let heavy_life = main_sequence_lifetime_myr(&heavy).expect("heavy lifetime");
        assert!(
            heavy_life < sun_life,
            "a heavier star burns its fuel faster and leaves the main sequence sooner, got heavy {} vs sun {}",
            heavy_life.to_f64_lossy(),
            sun_life.to_f64_lossy()
        );
        // And the heavier star, having a shorter t_ms, brightens FASTER: at the same age it is a larger fraction of
        // the way through its life, so a larger fraction of the way up the brightening curve.
        let age = Fixed::from_int(1000);
        let sun_f = current_luminosity_ratio(&sun, age)
            .expect("sun still on MS")
            .checked_div(sun_l)
            .expect("sun brightening factor");
        let heavy_f = current_luminosity_ratio(&heavy, age)
            .expect("heavy still on MS")
            .checked_div(heavy_l)
            .expect("heavy brightening factor");
        assert!(
            heavy_f > sun_f,
            "the shorter-lived heavier star brightens faster per unit time, got {} vs {}",
            heavy_f.to_f64_lossy(),
            sun_f.to_f64_lossy()
        );
    }

    #[test]
    fn the_star_leaves_the_main_sequence_at_its_lifetime() {
        // Just before t_ms the star is still on the main sequence; at and past t_ms it has exhausted its core
        // hydrogen and LEAVES, so the brightening caps to None (the post-main-sequence branch is a separate arc).
        let star = aging_params(Fixed::ONE, Fixed::ONE); // t_ms = 10,000 Myr
        assert!(
            current_luminosity_ratio(&star, Fixed::from_int(9_999)).is_some(),
            "just before t_ms the star is still on the main sequence"
        );
        assert!(
            current_luminosity_ratio(&star, Fixed::from_int(10_000)).is_none(),
            "at t_ms the star leaves the main sequence"
        );
        assert!(
            current_luminosity_ratio(&star, Fixed::from_int(12_000)).is_none(),
            "past t_ms is the post-main-sequence regime, not extrapolated here"
        );
    }

    #[test]
    fn the_brightening_is_deterministic() {
        // A pure derivation replays bit-for-bit, the determinism the canon requires.
        let star = aging_params(Fixed::from_ratio(3, 2), Fixed::from_ratio(5, 4));
        let a = current_luminosity_ratio(&star, Fixed::from_int(3_000));
        let b = current_luminosity_ratio(&star, Fixed::from_int(3_000));
        assert_eq!(a, b, "the brightening replays deterministically");
    }

    #[test]
    fn the_star_ages_on_the_deep_time_clock() {
        // The star and the planet share one clock: as the run steps, the star's age advances through elapsed_myr
        // and its luminosity climbs, the drive that warms the surface over the run.
        let star = aging_params(Fixed::ONE, Fixed::ONE);
        let params = [mantle_params(5)];
        let mut state = young_state(2, 2000);
        let l0 = state
            .stellar_luminosity_ratio(&star)
            .expect("on the main sequence at the start");
        assert_eq!(
            l0,
            Fixed::ONE,
            "a fresh run begins at the star's ZAMS anchor"
        );
        for _ in 0..20 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
        }
        assert_eq!(
            state.star_age_myr(),
            Fixed::from_int(2000),
            "the star's age tracks the deep-time clock (20 ticks of 100 Myr)"
        );
        let l1 = state
            .stellar_luminosity_ratio(&star)
            .expect("still on the main sequence");
        assert!(
            l1 > l0,
            "the star brightens as the deep-time run advances, warming the surface, got {} vs {}",
            l1.to_f64_lossy(),
            l0.to_f64_lossy()
        );
    }

    #[test]
    fn a_run_can_begin_with_an_already_aged_star() {
        // The per-run start age: a run beginning mid-life carries the star's prior age, so at the same elapsed time
        // its star is further up the brightening curve than a run around a zero-age star.
        let star = aging_params(Fixed::ONE, Fixed::ONE);
        let fresh = young_state(1, 1800);
        let aged = fresh.clone().with_star_start_age(Fixed::from_int(5000));
        assert_eq!(
            aged.star_age_myr(),
            Fixed::from_int(5000),
            "the run begins with the star already aged"
        );
        assert!(
            aged.stellar_luminosity_ratio(&star).expect("aged, on MS")
                > fresh.stellar_luminosity_ratio(&star).expect("fresh, on MS"),
            "the already-aged star is brighter at the same elapsed time"
        );
    }

    #[test]
    fn a_massless_or_dark_star_has_no_defined_lifetime() {
        // Fail-loud guards: a non-positive mass (no fuel) or a non-positive ZAMS luminosity (no burn rate) has no
        // defined main-sequence lifetime, routed to None rather than a fabricated value.
        assert_eq!(
            main_sequence_lifetime_myr(&aging_params(Fixed::ONE, Fixed::ZERO)),
            None,
            "no mass, no fuel, no lifetime"
        );
        assert_eq!(
            main_sequence_lifetime_myr(&aging_params(Fixed::ZERO, Fixed::ONE)),
            None,
            "no luminosity, no burn rate, no lifetime"
        );
        assert_eq!(
            current_luminosity_ratio(&aging_params(Fixed::ONE, Fixed::ONE), Fixed::from_int(-100)),
            None,
            "a negative age is not a main-sequence age"
        );
    }

    #[test]
    fn the_interior_evolves_over_deep_time() {
        // A hot young mantle relaxes toward its conductive steady state over successive ticks: the temperature
        // MOVES (the world is not static), and the clock accumulates.
        let start = young_state(4, 2000);
        let params = [mantle_params(5)];
        let mut state = start.clone();
        for _ in 0..5 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
        }
        assert!(
            state.columns[0].temperature != start.columns[0].temperature,
            "the interior temperature evolves over deep time, not static"
        );
        assert!(
            state.columns[0].temperature < start.columns[0].temperature,
            "a hot mantle with modest radiogenic heat cools toward its steady state"
        );
        assert_eq!(
            state.elapsed_myr,
            Fixed::from_int(500),
            "the deep-time clock accumulates (5 ticks of 100 Myr)"
        );
    }

    #[test]
    fn lateral_variation_emerges_from_per_column_heat() {
        // The provinces: two columns start identical but carry different radiogenic budgets, so after deep time
        // they hold different temperatures. The lateral variation is the WRITTEN record of each column's history,
        // never an authored map (the field started uniform).
        let start = young_state(2, 2000);
        assert_eq!(
            start.columns[0], start.columns[1],
            "a fresh planet starts laterally uniform"
        );
        let params = [mantle_params(5), mantle_params(400)];
        let mut state = start;
        for _ in 0..8 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
        }
        assert!(
            state.columns[0].temperature != state.columns[1].temperature,
            "columns with different radiogenic heat diverge into provinces, got {} vs {}",
            state.columns[0].temperature.to_f64_lossy(),
            state.columns[1].temperature.to_f64_lossy()
        );
        assert!(
            state.columns[1].temperature > state.columns[0].temperature,
            "the more radiogenic column stays hotter"
        );
    }

    #[test]
    fn the_crust_grows_from_volcanism_and_the_provinces_thicken_where_the_mantle_is_hot() {
        // The volcanism: a hot mantle builds crust over deep time (the accumulated derived thickness that retires
        // the 30 km fixture), and a hotter, more-radiogenic column builds THICKER crust than its cooler neighbour,
        // so the crust field is the written record of the interior's melt history, the provinces in relief.
        let start = young_state(2, 2000);
        assert_eq!(
            start.crust_thickness_km[0],
            Fixed::ZERO,
            "a fresh planet has no crust yet"
        );
        let params = [mantle_params(5), mantle_params(600)];
        let mut state = start;
        for _ in 0..10 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
        }
        assert!(
            state.crust_thickness_km[0] > Fixed::ZERO,
            "the interior melt builds crust over deep time"
        );
        assert!(
            state.crust_thickness_km[1] > state.crust_thickness_km[0],
            "the hotter, more-radiogenic column builds thicker crust (a province), got {} vs {}",
            state.crust_thickness_km[1].to_f64_lossy(),
            state.crust_thickness_km[0].to_f64_lossy()
        );
    }

    #[test]
    fn a_sub_solidus_mantle_builds_no_crust() {
        // A mantle colder than its surface solidus (1373 K) makes no melt, so it builds no crust: the volcanism is
        // the interior's own readout, not an authored floor of crust.
        assert_eq!(
            crust_growth(
                Fixed::from_int(1000),
                Fixed::ZERO,
                &melt_params(),
                Fixed::from_int(100)
            ),
            Fixed::ZERO,
            "a sub-solidus column makes no melt and no crust"
        );
        assert!(
            crust_growth(
                Fixed::from_int(1800),
                Fixed::ZERO,
                &melt_params(),
                Fixed::from_int(100)
            ) > Fixed::ZERO,
            "a super-solidus column with no crust yet does build crust"
        );
    }

    #[test]
    fn the_crust_saturates_at_the_columns_equilibrium_and_never_un_forms() {
        // The derive-first bound: a super-solidus column relaxes toward the equilibrium crust the melt column
        // supports and stops there (the finite fusible source depletes), so the surface relief cannot run away.
        // First find the equilibrium the column supports (the full melt-column crust the FIRST increment closes
        // toward from zero), then prove a crust already at or above it builds no more.
        let hot = Fixed::from_int(1800);
        let melt = melt_params();
        let dt = Fixed::from_int(100);
        // The equilibrium crust the column supports, read straight from the melt column.
        let equilibrium = adiabatic_melt_column(
            hot,
            melt.solidus_surface_k,
            melt.solidus_slope_k_per_gpa,
            melt.adiabat_slope_k_per_gpa,
            melt.productivity_per_gpa,
            melt.source_density_kg_per_m3,
            melt.gravity_m_per_s2,
        )
        .expect("a super-solidus column has a melt column")
        .crust_thickness_km;
        assert!(
            equilibrium > Fixed::ZERO,
            "the hot column supports some crust"
        );
        // AT the equilibrium: the source is spent, no more crust.
        assert_eq!(
            crust_growth(hot, equilibrium, &melt, dt),
            Fixed::ZERO,
            "a crust at the column's equilibrium builds no more (the finite source is depleted)"
        );
        // ABOVE the equilibrium (a column that has since cooled): no negative step, the crust does not un-form.
        assert_eq!(
            crust_growth(
                hot,
                equilibrium.saturating_add(Fixed::from_int(50)),
                &melt,
                dt
            ),
            Fixed::ZERO,
            "a crust above the equilibrium neither grows nor un-forms"
        );
        // BELOW the equilibrium: it grows, but by a bounded step (never overshooting the equilibrium in one tick
        // for dt <= processing_time), the relaxation the province spread stays finite under.
        let below = equilibrium.checked_div(Fixed::from_int(2)).unwrap();
        let step = crust_growth(hot, below, &melt, dt);
        assert!(step > Fixed::ZERO, "a thin hot column still grows");
        assert!(
            below.saturating_add(step) <= equilibrium.saturating_add(Fixed::from_ratio(1, 1000)),
            "one tick does not overshoot the equilibrium (bounded relaxation)"
        );
    }

    // --- the mass ledger slice ---

    #[test]
    fn the_areal_mass_unit_bridge_round_trips_and_the_ceiling_it_was_chosen_for_is_real() {
        // THE UNIT, computed rather than asserted from memory. A kilometre of crust at 2900 kg per cubic metre
        // carries 2900 megagrams per square metre, because the 1000 m/km scaling the thickness up and the
        // 1000 kg/Mg scaling the mass down cancel exactly.
        let rho = Fixed::from_int(CRUST_DENSITY_KG_M3);
        let thirty_km = Fixed::from_int(30);
        let mass = areal_mass_mg_per_m2(thirty_km, rho).expect("representable");
        let expected = 30.0 * f64::from(CRUST_DENSITY_KG_M3);
        assert!(
            (mass.to_f64_lossy() - expected).abs() < 1e-6,
            "30 km of crust at {CRUST_DENSITY_KG_M3} kg/m^3 is {expected} Mg/m^2, got {}",
            mass.to_f64_lossy()
        );
        // And it round-trips exactly at these magnitudes, which is what lets the thickness be the stock's readout
        // rather than a second accumulator.
        assert_eq!(
            thickness_km_from_areal_mass(mass, rho),
            Some(thirty_km),
            "the areal mass reads back as the thickness that made it"
        );

        // THE CEILING THE UNIT WAS CHOSEN FOR, measured against the real Fixed::MAX rather than recalled. The
        // scaffold's own convecting mantle in kilograms per square metre is past the fixed-point ceiling, so the
        // source stock could not be held in SI at all; in megagrams per square metre it fits with room to spare.
        let kg_per_m2 = f64::from(CONVECTING_DEPTH_KM) * 1000.0 * f64::from(MANTLE_DENSITY_KG_M3);
        let ceiling = Fixed::MAX.to_f64_lossy();
        assert!(
            kg_per_m2 > ceiling,
            "the premise for the unit choice must hold: {kg_per_m2} kg/m^2 should exceed the {ceiling} ceiling"
        );
        let stock = source_stock();
        assert!(
            stock.to_f64_lossy() < ceiling / 100.0,
            "and the same column in Mg/m^2 ({}) sits far inside it",
            stock.to_f64_lossy()
        );
        // A degenerate layer refuses rather than returning a fabricated mass.
        assert!(areal_mass_mg_per_m2(thirty_km, Fixed::ZERO).is_none());
        assert!(thickness_km_from_areal_mass(mass, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_melt_transaction_conserves_the_ledger_to_the_bit() {
        // THE INVARIANT. Crust is no longer a thickness that appears: every megagram of it was debited from the
        // column's own source, so `crust + source` is unchanged, EXACTLY, in raw fixed-point bits, at every
        // column and at every tick of a long run. This is the property that makes recycling representable: there
        // are two stocks, and mass only ever moves between them.
        let n = 6usize;
        let start = young_state(n, 2000);
        let params = [
            mantle_params(5),
            mantle_params(120),
            mantle_params(300),
            mantle_params(600),
            mantle_params(50),
            mantle_params(900),
        ];
        let totals_before: Vec<i128> = (0..n)
            .map(|i| {
                start.crust_areal_mass[i].to_bits() as i128
                    + start.source_areal_mass[i].to_bits() as i128
            })
            .collect();
        let mut state = start.clone();
        for tick in 0..40 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(50))
                .expect("steps");
            for (i, opening) in totals_before.iter().enumerate() {
                let now = state.crust_areal_mass[i].to_bits() as i128
                    + state.source_areal_mass[i].to_bits() as i128;
                assert_eq!(
                    now, *opening,
                    "column {i}'s ledger moved at tick {tick}: crust plus source must be invariant to the bit"
                );
                assert!(
                    state.source_areal_mass[i] >= Fixed::ZERO,
                    "column {i} spent more source than it held at tick {tick}"
                );
            }
        }
        // The run must have MOVED mass, else the invariant above is the invariant of a field that did nothing.
        assert!(
            state.crust_areal_mass.iter().any(|m| *m > Fixed::ZERO),
            "the transaction has to have moved mass for its conservation to mean anything"
        );
        assert!(
            state
                .source_areal_mass
                .iter()
                .zip(start.source_areal_mass.iter())
                .any(|(now, before)| now < before),
            "and the source has to have paid for it"
        );
    }

    #[test]
    fn the_thickness_is_the_crust_stocks_readout_not_a_second_accumulator() {
        // The crust exists ONCE, as a mass. The thickness the isostasy floats is that mass divided by the
        // emplaced-crust density, re-read wherever the stock moves, so the two representations cannot drift
        // apart tick by tick. This is a structural gate: it fails the moment a path moves the thickness without
        // moving the stock behind it.
        let melt = melt_params();
        let params = [mantle_params(5), mantle_params(400), mantle_params(900)];
        let mut state = young_state(3, 2000);
        for _ in 0..30 {
            state = step_deep_time(&state, &params, &melt, Fixed::from_int(50)).expect("steps");
        }
        for (i, m) in state.crust_areal_mass.iter().enumerate() {
            assert_eq!(
                state.crust_thickness_km[i],
                thickness_km_from_areal_mass(*m, melt.crust_density_kg_per_m3).expect("reads back"),
                "column {i}'s thickness is not the readout of its crust stock"
            );
        }
        assert!(
            state.crust_thickness_km.iter().any(|t| *t > Fixed::ZERO),
            "and the run built crust, so the identity is over a field that moved"
        );
    }

    #[test]
    fn a_spent_source_stops_the_crust_and_never_goes_negative() {
        // THE FINITE SOURCE, as a reservoir rather than as a clamp. Given a source stock far smaller than the
        // crust the column's melt column would support, the transaction can only pay out what the source holds:
        // the source empties exactly, the crust holds exactly what the source lost, and neither stock goes
        // negative or is topped up from nowhere. Before the ledger this limit was unrepresentable, because the
        // saturation was a comparison against an equilibrium and nothing was being spent.
        let melt = melt_params();
        // A deliberately tiny source: one kilometre of crust's worth of mass, against a hot column whose melt
        // column supports far more.
        let tiny = areal_mass_mg_per_m2(Fixed::ONE, melt.crust_density_kg_per_m3).expect("small");
        let mut state = DeepTimeState::young(1, Fixed::from_int(2000), tiny);
        let params = [mantle_params(900)];
        for _ in 0..60 {
            state = step_deep_time(&state, &params, &melt, Fixed::from_int(50)).expect("steps");
            assert!(
                state.source_areal_mass[0] >= Fixed::ZERO,
                "the source never goes negative"
            );
            assert_eq!(
                state.crust_areal_mass[0].to_bits() as i128
                    + state.source_areal_mass[0].to_bits() as i128,
                tiny.to_bits() as i128,
                "and the pair still sums to what the column began with"
            );
        }
        assert_eq!(
            state.source_areal_mass[0],
            Fixed::ZERO,
            "a hot column against a tiny source spends it to exactly zero"
        );
        assert_eq!(
            state.crust_areal_mass[0], tiny,
            "and the crust holds exactly the mass the source lost, no more"
        );
        // The crust it stands as is the one kilometre that mass buys, which is the cap BITING: the melt column
        // for this column supports far more than a kilometre, so without the source stock it would have grown on.
        let unbounded_equilibrium = adiabatic_melt_column(
            state.columns[0].temperature,
            melt.solidus_surface_k,
            melt.solidus_slope_k_per_gpa,
            melt.adiabat_slope_k_per_gpa,
            melt.productivity_per_gpa,
            melt.source_density_kg_per_m3,
            melt.gravity_m_per_s2,
        )
        .expect("a super-solidus column has a melt column")
        .crust_thickness_km;
        assert!(
            unbounded_equilibrium > state.crust_thickness_km[0],
            "the melt column supports {} km, so the {} km the source could pay for is the source binding",
            unbounded_equilibrium.to_f64_lossy(),
            state.crust_thickness_km[0].to_f64_lossy()
        );
    }

    #[test]
    fn a_ledger_that_does_not_cover_every_column_fails_loud() {
        // The transaction needs a stock per column to balance over. A caller whose ledger is short is refused
        // rather than partly transacted, which would be mass appearing in the columns that had no stock.
        let mut state = young_state(4, 1800);
        state.source_areal_mass.pop();
        assert!(
            step_deep_time(
                &state,
                &[mantle_params(50)],
                &melt_params(),
                Fixed::from_int(50)
            ) == Err(DeepTimeRefusal::LedgerDoesNotCoverEveryColumn),
            "a short source stock fails loud, and SAYS which invariant broke"
        );
        let mut state = young_state(4, 1800);
        state.crust_areal_mass.pop();
        assert!(
            step_deep_time(
                &state,
                &[mantle_params(50)],
                &melt_params(),
                Fixed::from_int(50)
            ) == Err(DeepTimeRefusal::LedgerDoesNotCoverEveryColumn),
            "a short crust stock fails loud, and SAYS which invariant broke"
        );
    }

    /// The parameters must cover ONE column (the laterally uniform broadcast) or EVERY column. Three rows
    /// against four columns used to fall through to row zero for all four, delivering a laterally uniform world
    /// to a caller who had asked for a varied one, with nothing in the result to say so.
    #[test]
    fn a_parameter_set_matching_neither_one_column_nor_every_column_fails_loud() {
        let state = young_state(4, 1800);
        let melt = melt_params();
        let dt = Fixed::from_int(50);
        // Both LEGAL shapes still step, so the guard admits what the contract documents.
        assert!(
            step_deep_time(&state, &[mantle_params(50)], &melt, dt).is_ok(),
            "one row broadcasts to every column"
        );
        let per_column: Vec<_> = (0..4).map(|i| mantle_params(50 + i)).collect();
        assert!(
            step_deep_time(&state, &per_column, &melt, dt).is_ok(),
            "one row per column is the lateral-variation shape"
        );
        // Anything else is a mismatch, and it reports BOTH counts rather than broadcasting row zero.
        let short: Vec<_> = (0..3).map(|i| mantle_params(50 + i)).collect();
        assert_eq!(
            step_deep_time(&state, &short, &melt, dt),
            Err(DeepTimeRefusal::ColumnParamsLengthMismatch {
                supplied: 3,
                columns: 4
            }),
            "a parameter set covering neither one column nor every column fails loud"
        );
    }

    /// The melt conversion has ONE failure and TWO physical zeros, and the single catch-all arm pooled them, so
    /// an overflowing conversion moved zero mass and produced a state identical to a quiescent column's. The
    /// failure now refuses; both physical zeros still step, which is the half that would break if the refusal
    /// were over-eager.
    #[test]
    fn an_unrepresentable_melt_conversion_refuses_where_the_physical_zeros_step() {
        let dt = Fixed::from_int(50);
        // A crust density large enough that the asked thickness times it leaves the representable window. The
        // asked thickness does not move with it, because `crust_growth` integrates the melt column at the
        // SOURCE density, so this isolates the conversion.
        let mut melt = melt_params();
        melt.crust_density_kg_per_m3 = Fixed::from_int(2_000_000_000);
        assert_eq!(
            step_deep_time(&young_state(3, 1800), &[mantle_params(50)], &melt, dt),
            Err(DeepTimeRefusal::MeltMassUnrepresentable { index: 0 }),
            "a melt mass that cannot be FORMED refuses, where it used to move zero and read as quiescent"
        );

        // PHYSICAL ZERO ONE: a sub-solidus column asks for no melt. Zero is the right answer, not a failure.
        let cold = step_deep_time(
            &young_state(3, 1000),
            &[mantle_params(50)],
            &melt_params(),
            dt,
        )
        .expect("a mantle below its own surface solidus makes no melt, which is not a refusal");
        assert!(
            cold.crust_areal_mass.iter().all(|&m| m == Fixed::ZERO),
            "no melt, no crust, and no refusal"
        );

        // PHYSICAL ZERO TWO: a source already spent pays nothing, the finite-source limit as a reservoir.
        let mut spent = young_state(3, 1800);
        for s in spent.source_areal_mass.iter_mut() {
            *s = Fixed::ZERO;
        }
        let spent = step_deep_time(&spent, &[mantle_params(50)], &melt_params(), dt)
            .expect("a spent source pays nothing, which is the finite source, not a refusal");
        assert!(
            spent.crust_areal_mass.iter().all(|&m| m == Fixed::ZERO),
            "a spent source builds no crust, and no refusal"
        );
    }

    /// A credit or debit that cannot land refuses. Holding BOTH stocks preserved the pairing, which is why it
    /// was written, and it also asserted "no melt moved this tick", which the failed arithmetic cannot support.
    #[test]
    fn a_transfer_that_cannot_be_represented_refuses_rather_than_holding_both_stocks() {
        // A crust stock at the fixed-point ceiling under a zero thickness readout: an incoherent pair, and the
        // caller mismatch the refusal exists for. The zero thickness makes the melt column ask for a full
        // increment, and the credit onto a ceiling-valued stock cannot be represented.
        let mut state = young_state(3, 1800);
        state.crust_areal_mass[1] = Fixed::MAX;
        assert_eq!(
            step_deep_time(
                &state,
                &[mantle_params(50)],
                &melt_params(),
                Fixed::from_int(50)
            ),
            Err(DeepTimeRefusal::TransferUnrepresentable { index: 1 }),
            "a credit that cannot land refuses, and names the column"
        );
    }

    /// The clock refuses rather than railing. A saturating add returned a state reporting that the tick had
    /// advanced geological time when it had not, and every later tick would have railed at the same instant.
    #[test]
    fn a_clock_that_cannot_advance_refuses_rather_than_railing() {
        let mut state = young_state(3, 1800);
        state.elapsed_myr = Fixed::MAX;
        assert_eq!(
            step_deep_time(
                &state,
                &[mantle_params(50)],
                &melt_params(),
                Fixed::from_int(50)
            ),
            Err(DeepTimeRefusal::ClockOverflow),
            "a clock at the ceiling refuses, where saturating reported a tick that advanced no time"
        );
        // The magnitude, so the arm is not mistaken for a reachable one: a clock standing at the age of the
        // universe is four orders below the rail and steps without complaint.
        let mut aged = young_state(3, 1800);
        aged.elapsed_myr = Fixed::from_int(13_800);
        assert!(
            step_deep_time(
                &aged,
                &[mantle_params(50)],
                &melt_params(),
                Fixed::from_int(50)
            )
            .is_ok(),
            "13.8 Gyr is nowhere near the 2.147e9 Myr ceiling; the arm guards the arithmetic, not the physics"
        );
    }

    /// The unit bridge's doc claimed `None` on a non-positive INPUT while the guard admitted a zero thickness,
    /// and the CODE was right: a layer of no thickness has no mass. Refusing there would break the melt
    /// transaction's own physical zero, because a sub-solidus column asks for zero kilometres every tick.
    #[test]
    fn a_zero_thickness_layer_has_no_mass_and_only_a_negative_one_refuses() {
        let rho = Fixed::from_int(CRUST_DENSITY_KG_M3);
        assert_eq!(
            areal_mass_mg_per_m2(Fixed::ZERO, rho),
            Some(Fixed::ZERO),
            "no thickness, no mass: the right answer rather than a refusal"
        );
        assert_eq!(
            areal_mass_mg_per_m2(Fixed::from_int(-1), rho),
            None,
            "a negative thickness is geometrically meaningless and refuses"
        );
        assert_eq!(
            areal_mass_mg_per_m2(Fixed::from_int(10), Fixed::ZERO),
            None,
            "a material of no density does not exist and refuses"
        );
        assert_eq!(
            areal_mass_mg_per_m2(Fixed::from_int(10), Fixed::from_int(-1)),
            None,
            "a negative density refuses"
        );
    }

    #[test]
    fn the_tick_is_deterministic() {
        let start = young_state(6, 1800);
        let params = [mantle_params(50)];
        let a = step_deep_time(&start, &params, &melt_params(), Fixed::from_int(100)).expect("a");
        let b = step_deep_time(&start, &params, &melt_params(), Fixed::from_int(100)).expect("b");
        assert_eq!(
            a, b,
            "the tick is a pure function of the state and parameters"
        );
    }

    #[test]
    fn an_empty_parameter_set_fails_loud() {
        let start = young_state(3, 1500);
        assert!(
            step_deep_time(&start, &[], &melt_params(), Fixed::from_int(100))
                == Err(DeepTimeRefusal::NoColumnParams),
            "no parameters to step against fails loud, never a silent no-op"
        );
    }

    #[test]
    fn the_province_count_derives_from_the_convective_scale() {
        // The lateral province scale is DERIVED, not a hand-set grid: the count spanning a distance is the
        // distance over the convective cell width (mantle depth times the aspect ratio). A deeper mantle
        // makes fewer, wider provinces; a larger span more of them.
        let depth = Fixed::from_int(1_500_000); // ~1500 km convecting mantle
        let aspect = Fixed::from_ratio(141, 100); // ~sqrt(2), the free-free critical-mode aspect
        let circumference = Fixed::from_int(21_000_000); // ~2*pi*3340 km
        let n = provinces_across(circumference, depth, aspect).expect("derives");
        assert!(
            (7..=12).contains(&n),
            "the province count is circumference / (depth * aspect), got {n}"
        );
        // A deeper mantle (wider cells) yields strictly fewer provinces across the same span.
        let deeper = provinces_across(circumference, Fixed::from_int(3_000_000), aspect).unwrap();
        assert!(
            deeper < n,
            "a deeper mantle makes fewer, wider provinces, got {deeper} vs {n}"
        );
        // Degenerate inputs fail loud rather than fabricating a scale.
        assert!(provinces_across(Fixed::ZERO, depth, aspect).is_none());
        assert!(provinces_across(circumference, Fixed::ZERO, aspect).is_none());
        assert!(provinces_across(circumference, depth, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_province_column_wires_the_derived_depth_gravity_and_heat() {
        // The derived per-planet inputs thread into the column params (the depth retires the depth = 1
        // fixture), and a hotter radiogenic budget is carried through so the provinces can diverge.
        let depth_mm = Fixed::from_ratio(15, 10); // 1.5 Mm derived convecting depth
        let g = Fixed::from_ratio(37, 10); // Mars-class surface gravity
        let cool = province_column_params(
            depth_mm,
            g,
            Fixed::from_int(5),
            Fixed::from_int(300),
            Fixed::from_int(20),
            &test_cluster(),
        )
        .expect("the cool column resolves");
        let hot = province_column_params(
            depth_mm,
            g,
            Fixed::from_int(400),
            Fixed::from_int(300),
            Fixed::from_int(20),
            &test_cluster(),
        )
        .expect("the hot column resolves");
        assert_eq!(
            cool.depth_m,
            depth_mm.checked_mul(Fixed::from_int(1_000_000)).unwrap(),
            "the derived depth is wired in as SI metres, not a fixture"
        );
        assert_eq!(
            cool.gravity_m_s2, g,
            "the derived surface gravity is wired in"
        );
        assert!(
            hot.heat_production_j_per_kg > cool.heat_production_j_per_kg,
            "the per-province radiogenic budget varies"
        );
        // THE FIXTURE CLUSTER IS GONE from these columns, and this asserts it rather than trusting the
        // constructor: a density of 1 kg/m^3 and a specific heat of 10 J/kg/K were the tagged conflict.
        assert!(
            cool.density_kg_m3 > Fixed::from_int(1000),
            "the density is the DERIVED one, not the fixture's 1"
        );
        assert!(
            cool.specific_heat_j_kg_k > Fixed::from_int(100),
            "the specific heat is the DERIVED one, not the fixture's 10"
        );
        // And the diffusivity is COMPUTED from the three facts rather than stored beside them, so the
        // twentyfold disagreement the old cluster carried cannot exist.
        let implied = cool
            .thermal_conductivity_w_m_k
            .checked_div(cool.density_kg_m3)
            .and_then(|x| x.checked_div(cool.specific_heat_j_kg_k))
            .expect("representable");
        assert_eq!(
            cool.thermal_diffusivity_m2_s, implied,
            "kappa must BE k/(rho c_p), not a second independent answer to the same question"
        );

        // The two share the same operating point apart from the varied heat, so a step diverges them.
        let cool_state = convection_step_si(
            &ColumnState {
                temperature: Fixed::from_int(1588),
                convecting: false,
            },
            &cool,
        )
        .expect("the cool column steps");
        let hot_state = convection_step_si(
            &ColumnState {
                temperature: Fixed::from_int(1588),
                convecting: false,
            },
            &hot,
        )
        .expect("the hot column steps");
        assert!(
            hot_state.temperature > cool_state.temperature,
            "the more-radiogenic province stays hotter after a step, got {} vs {}",
            hot_state.temperature.to_f64_lossy(),
            cool_state.temperature.to_f64_lossy()
        );
    }

    // --- the bombardment slice ---

    // A moon-like target and a competent-rock coupling, the crater law's own illustrative fixtures (reserved
    // rows the law reads, not authored floor values). Mirrors the impact_event and crater tests.
    fn impact_target() -> Target {
        Target {
            gravity: Fixed::from_ratio(162, 100),
            strength: Fixed::from_int(10_000_000),
            density: Fixed::from_int(2500),
        }
    }
    fn impact_coupling() -> CraterCoupling {
        CraterCoupling {
            velocity_exponent: Fixed::from_ratio(55, 100),
            density_exponent: Fixed::from_ratio(4, 10),
            efficiency_coefficient: Fixed::from_ratio(2, 10),
            strength_coefficient: Fixed::ONE,
            bowl_aspect: Fixed::from_ratio(2, 10),
            eject_fraction: Fixed::from_ratio(5, 10),
        }
    }
    // A flux config whose size range resolves on the test grid (2 km cells): impactor radii 300..1500 m open
    // few-cell to ~ten-cell craters, so every drawn impact leaves a mark. `reservoir` and `tau` set the
    // bombardment intensity and its decline; `cap` is the determinism-and-cost bound.
    fn flux_params(reservoir: i32, tau_myr: i32, cap: u32) -> ImpactFluxParams {
        ImpactFluxParams {
            reservoir_body_count: Fixed::from_int(reservoir),
            sweep_timescale_myr: Fixed::from_int(tau_myr),
            differential_slope: Fixed::from_ratio(35, 10),
            min_impactor_radius_m: Fixed::from_int(300),
            max_impactor_radius_m: Fixed::from_int(1500),
            impact_velocity_m_s: Fixed::from_int(17_000),
            impactor_density: Fixed::from_int(3000),
            target: impact_target(),
            coupling: impact_coupling(),
            ejecta: EjectaFan {
                speed: Fixed::from_int(200),
                elevation_angle: Fixed::HALF_PI.div(Fixed::from_int(2)), // 45 degrees, the max-range angle
                azimuths: 24,
            },
            forces: BallisticForces {
                gravity: Fixed::from_ratio(162, 100),
                cell_size: Fixed::from_int(2000),
                step_cap: 200,
            },
            per_tick_impact_cap: cap,
        }
    }

    #[test]
    fn the_bombardment_accumulates_craters_and_conserves_mass() {
        let (w, h) = (41usize, 41usize);
        let n = w * h;
        let flux = flux_params(30, 100, 64);
        let params = [mantle_params(50)];
        let mut state = young_state(n, 2000);
        for tick in 0..20u64 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(50))
                .expect("steps");
            state = bombard_tick(&state, w, h, &flux, 0x00C0_FFEE, tick, Fixed::from_int(50));
        }
        assert!(
            state.impact_count > 0,
            "the bombardment carved craters over deep time"
        );
        assert!(
            state.impact_relief_m.iter().any(|&r| r != Fixed::ZERO),
            "the surface carries the accumulated craters and blankets"
        );
        // Mass conserves to the bit: every impact delta sums to exactly zero, so the accumulated impact relief,
        // starting from zero, sums to exactly zero (the bowls balance the blankets, in raw field bits).
        let sum: i128 = state
            .impact_relief_m
            .iter()
            .map(|r| r.to_bits() as i128)
            .sum();
        assert_eq!(sum, 0, "the impact relief conserves mass to the bit");
        // Both signs are present: a crater bowl dug below the datum and an ejecta blanket rose above it, so
        // material was MOVED, never merely added.
        assert!(
            state.impact_relief_m.iter().any(|&r| r < Fixed::ZERO),
            "a crater bowl dug below the datum"
        );
        assert!(
            state.impact_relief_m.iter().any(|&r| r > Fixed::ZERO),
            "an ejecta blanket rose above the datum"
        );
    }

    #[test]
    fn the_flux_declines_with_epoch_early_heavy_late_quiescent() {
        let (w, h) = (41usize, 41usize);
        let n = w * h;
        // Cap high so it does not bind; the decline is the accretion tail's own, tau = 100 Myr over a 1 Gyr run.
        let flux = flux_params(24, 100, 100);
        let params = [mantle_params(50)];
        let mut state = young_state(n, 1600);
        let (mut early, mut late) = (0u64, 0u64);
        for tick in 0..20u64 {
            let before = state.impact_count;
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(50))
                .expect("steps");
            state = bombard_tick(&state, w, h, &flux, 0x0000_BEEF, tick, Fixed::from_int(50));
            let added = state.impact_count - before;
            if tick < 5 {
                early += added;
            }
            if tick >= 15 {
                late += added;
            }
        }
        assert!(early > 0, "the early accretion tail delivers impacts");
        assert!(
            early > late,
            "the bombardment is heavy early and quiescent late (early {early} vs late {late})"
        );
    }

    #[test]
    fn the_bombardment_is_deterministic() {
        let (w, h) = (41usize, 41usize);
        let n = w * h;
        let flux = flux_params(30, 100, 64);
        let params = [mantle_params(50)];
        let base = young_state(n, 1800);
        let run = |seed: u64| {
            let mut s = base.clone();
            for tick in 0..10u64 {
                s = step_deep_time(&s, &params, &melt_params(), Fixed::from_int(50))
                    .expect("steps");
                s = bombard_tick(&s, w, h, &flux, seed, tick, Fixed::from_int(50));
            }
            s
        };
        let a = run(0x0000_ABCD);
        let b = run(0x0000_ABCD);
        assert_eq!(
            a.impact_relief_m, b.impact_relief_m,
            "the same world reproduces the same crater field"
        );
        assert_eq!(
            a.craters, b.craters,
            "the same world reproduces the same discrete crater rows"
        );
        assert_eq!(a.impact_count, b.impact_count, "and the same crater count");
        let c = run(0x0000_1234);
        assert!(
            c.craters != a.craters || c.impact_count != a.impact_count,
            "a different world seed bombards differently"
        );
    }

    #[test]
    fn the_bombardment_shares_the_surface_with_the_volcanism() {
        let (w, h) = (41usize, 41usize);
        let n = w * h;
        let flux = flux_params(30, 100, 64);
        let params = [mantle_params(50)];
        let base = young_state(n, 1800);
        // Without bombardment: pure interior-and-volcanism evolution.
        let mut quiet = base.clone();
        for _ in 0..10 {
            quiet = step_deep_time(&quiet, &params, &melt_params(), Fixed::from_int(50))
                .expect("steps");
        }
        // With bombardment on the same clock.
        let mut struck = base.clone();
        for tick in 0..10u64 {
            struck = step_deep_time(&struck, &params, &melt_params(), Fixed::from_int(50))
                .expect("steps");
            struck = bombard_tick(&struck, w, h, &flux, 0x0000_5EED, tick, Fixed::from_int(50));
        }
        // The bombardment touches NEITHER the interior nor the volcanic crust (it is a separate step term).
        assert_eq!(
            quiet.columns, struck.columns,
            "bombardment leaves the interior untouched"
        );
        assert_eq!(
            quiet.crust_thickness_km, struck.crust_thickness_km,
            "bombardment leaves the volcanic crust untouched"
        );
        // But the surface relief differs: the quiet world has no craters, the struck one does.
        assert!(
            quiet.impact_relief_m.iter().all(|&r| r == Fixed::ZERO),
            "the un-bombarded surface has no craters"
        );
        assert!(
            struck.impact_relief_m.iter().any(|&r| r != Fixed::ZERO),
            "the bombarded surface carries craters"
        );
        // Impacts and volcanism co-exist on ONE surface: the provinces built crust AND the impacts carved it.
        assert!(
            struck.crust_thickness_km.iter().any(|&c| c > Fixed::ZERO),
            "the provinces built volcanic crust over the same run"
        );
    }

    #[test]
    fn the_bombardment_is_bounded_and_soft_on_degenerate_input() {
        let (w, h) = (20usize, 20usize);
        let n = w * h;
        let flux = flux_params(1000, 100, 5); // a huge reservoir against a tiny per-tick cap
        let params = [mantle_params(50)];
        let mut state = young_state(n, 1800);
        state =
            step_deep_time(&state, &params, &melt_params(), Fixed::from_int(50)).expect("steps");
        let struck = bombard_tick(&state, w, h, &flux, 0x1, 0, Fixed::from_int(50));
        assert!(
            struck.impact_count - state.impact_count <= 5,
            "the per-tick cap bounds the strike count regardless of the flux intensity"
        );
        // A grid that does not match the province field: no change (soft, never a panic).
        let mismatch = bombard_tick(&state, w + 1, h, &flux, 0x1, 0, Fixed::from_int(50));
        assert_eq!(
            mismatch.impact_relief_m, state.impact_relief_m,
            "a grid mismatch leaves the surface unchanged"
        );
        assert_eq!(mismatch.impact_count, state.impact_count);
        // A non-positive tick duration draws nothing.
        let zero = bombard_tick(&state, w, h, &flux, 0x1, 0, Fixed::ZERO);
        assert_eq!(
            zero.impact_count, state.impact_count,
            "a zero-duration tick draws nothing"
        );
    }

    /// The cap is a COMPUTE bound, so what it cannot admit is WITHHELD and re-offered rather than deleted. The
    /// truncation discarded the remainder while the flux's analytic decline went on counting those bodies as
    /// swept, so the reservoir's budget leaked by exactly the amount the cap bit.
    #[test]
    fn the_cap_withholds_the_impacts_it_cannot_admit_and_re_offers_them() {
        let (w, h) = (20usize, 20usize);
        let n = w * h;
        // A large reservoir against a tiny cap, so the cap binds hard on the first tick.
        let flux = flux_params(1000, 100, 5);
        let params = [mantle_params(50)];
        let dt = Fixed::from_int(50);
        let state =
            step_deep_time(&young_state(n, 1800), &params, &melt_params(), dt).expect("steps");
        let first = bombard_tick(&state, w, h, &flux, 0x1, 0, dt);
        let struck = first.impact_count - state.impact_count;
        assert!(struck <= 5, "the cap still bounds the per-tick work");
        assert!(
            first.deferred_impacts > 0,
            "what the cap could not admit is queued, never discarded"
        );

        // THE BUDGET CLOSES over the tick, against the offer computed from the flux model itself rather than
        // from a remembered number: everything the tail delivered either struck, is queued, or is counted
        // unrealizable. The one-event slack is the seeded Bernoulli on the expectation's fractional remainder.
        let swept = tail_rate_fraction(Fixed::ZERO, flux.sweep_timescale_myr)
            .and_then(|r0| {
                tail_rate_fraction(state.elapsed_myr, flux.sweep_timescale_myr)
                    .and_then(|r1| r0.checked_sub(r1))
            })
            .expect("the tail sweeps a fraction over the first tick");
        let expected = flux
            .reservoir_body_count
            .checked_mul(swept)
            .expect("the expected strike count");
        let floor_offer = expected.to_int().max(0) as u64;
        let accounted =
            struck + first.deferred_impacts + (first.unrealized_impacts - state.unrealized_impacts);
        assert!(
            accounted == floor_offer || accounted == floor_offer + 1,
            "the reservoir's budget must close: {accounted} accounted against an offer of {floor_offer} \
             (+1 for the Bernoulli)"
        );

        // THE QUEUE IS LIVE AFTER THE TAIL IS SPENT, which is the whole point of withholding: a body the cap
        // held back is one the reservoir already delivered, so it must still strike once the tail supplies
        // nothing. Note the queue GROWS while the tail out-delivers the cap (388 against a cap of 5 above); it
        // drains only when new arrivals fall below the cap, and this is that regime.
        let mut spent_tail =
            step_deep_time(&young_state(n, 1800), &params, &melt_params(), dt).expect("steps");
        spent_tail.elapsed_myr = Fixed::from_int(100_000); // far past the tail's decay
        spent_tail.deferred_impacts = 7;
        let drained = bombard_tick(&spent_tail, w, h, &flux, 0x1, 99, dt);
        assert!(
            drained.impact_count > spent_tail.impact_count,
            "the queue keeps delivering after the tail itself is spent"
        );
        assert_eq!(
            drained.deferred_impacts, 2,
            "the queue draws down by exactly what the cap admits (7 held, 5 admitted, 2 left)"
        );
    }

    /// An event the world's own parameters cannot realize used to `continue` with no trace, so a deleted impact
    /// was invisible: `impact_count` advances only after a strike fully lands, so a broken reservoir and a quiet
    /// epoch produced the same state.
    #[test]
    fn an_impact_the_world_cannot_realize_is_counted_rather_than_deleted() {
        let (w, h) = (20usize, 20usize);
        let n = w * h;
        // A DISORDERED size range, so the size-frequency draw refuses every strike: every admitted event is
        // unrealizable rather than merely unlucky.
        let mut flux = flux_params(50, 100, 8);
        flux.min_impactor_radius_m = Fixed::from_int(1500);
        flux.max_impactor_radius_m = Fixed::from_int(300);
        let dt = Fixed::from_int(50);
        let state = step_deep_time(
            &young_state(n, 1800),
            &[mantle_params(50)],
            &melt_params(),
            dt,
        )
        .expect("steps");
        let after = bombard_tick(&state, w, h, &flux, 0x1, 0, dt);
        assert_eq!(
            after.impact_count, state.impact_count,
            "no crater is fabricated out of a refused size draw"
        );
        assert_eq!(
            after.unrealized_impacts, 8,
            "every event the cap admitted is COUNTED as unrealized, one per refused draw"
        );
        assert!(
            after.deferred_impacts > 0,
            "and the events the cap never admitted stay queued, not deleted"
        );
    }

    /// A tail rate outside its own domain is not a swept reservoir. `unwrap_or(Fixed::ZERO)` on the interval's
    /// END rate made the swept fraction MAXIMAL, so a refusal produced the LARGEST possible bombardment: alone
    /// among the fallbacks here it failed upward.
    #[test]
    fn a_clock_outside_the_tail_rates_domain_draws_nothing_rather_than_everything() {
        let (w, h) = (20usize, 20usize);
        let n = w * h;
        let flux = flux_params(1000, 100, 64);
        let mut state = young_state(n, 1800);
        state.elapsed_myr = Fixed::ZERO
            .checked_sub(Fixed::from_int(10))
            .expect("a negative clock is representable");
        let after = bombard_tick(&state, w, h, &flux, 0x1, 0, Fixed::from_int(50));
        assert_eq!(
            after.impact_count, state.impact_count,
            "a clock outside the tail rate's domain draws NOTHING, where it used to draw a full capped tick"
        );
        assert_eq!(
            after.deferred_impacts, 0,
            "and queues nothing, because nothing was delivered"
        );
    }

    /// A surface that cannot be expressed in metres is a tick that cannot be bombarded. The kilometre-to-metre
    /// conversion fell back to zero, so a column whose crust overflowed the conversion was handed to the
    /// ballistic arc as a datum-level plain and the ejecta flew over sea level where the world holds a highland.
    #[test]
    fn a_surface_that_cannot_be_expressed_in_metres_does_not_bombard_a_datum_level_plain() {
        let (w, h) = (20usize, 20usize);
        let n = w * h;
        let flux = flux_params(1000, 100, 8);
        let dt = Fixed::from_int(50);
        let state = step_deep_time(
            &young_state(n, 1800),
            &[mantle_params(50)],
            &melt_params(),
            dt,
        )
        .expect("steps");
        // THE CONTROL: this state does draw, so the hold below is the conversion and not a quiet tick.
        let drew = bombard_tick(&state, w, h, &flux, 0x5EED, 0, dt);
        assert!(
            drew.impact_count > state.impact_count,
            "the control tick draws impacts"
        );
        // One column whose kilometres cannot be carried in metres (3e6 km at 1000 m/km leaves the window).
        let mut tall = state.clone();
        tall.crust_thickness_km[7] = Fixed::from_int(3_000_000);
        assert!(
            tall.crust_thickness_km[7].checked_mul(M_PER_KM).is_none(),
            "the fixture must overflow the conversion, or the test proves nothing"
        );
        let held = bombard_tick(&tall, w, h, &flux, 0x5EED, 0, dt);
        assert_eq!(
            held.impact_count, tall.impact_count,
            "a surface that cannot be formed is not bombarded over a substituted zero"
        );
        assert_eq!(
            held.craters.len(),
            tall.craters.len(),
            "and no crater row is emitted against it either"
        );
    }

    /// The impact heat posting is an OPEN EDGE carried in the TYPE rather than in a comment alone, and the
    /// refusal is EARNED by the arithmetic: a representative impactor from this module's own flux scaffold
    /// cannot have its kinetic energy formed in Q32.32. That is the disclosure's claim, checked rather than
    /// restated.
    #[test]
    fn impact_kinetic_energy_refuses_at_planetary_magnitudes_and_says_why() {
        let flux = flux_params(30, 100, 64);
        let body = Impactor {
            radius: flux.max_impactor_radius_m,
            velocity: flux.impact_velocity_m_s,
            density: flux.impactor_density,
        };
        assert_eq!(
            impact_kinetic_energy_j(body),
            Err(ImpactHeatRefusal::UnrepresentableInFixedPoint),
            "a 1.5 km body at 17 km/s carries energy far past the fixed-point ceiling"
        );
        // A degenerate impactor is a DIFFERENT refusal, so a consumer can tell bad data from a magnitude wall.
        assert_eq!(
            impact_kinetic_energy_j(Impactor {
                radius: Fixed::ZERO,
                velocity: flux.impact_velocity_m_s,
                density: flux.impactor_density,
            }),
            Err(ImpactHeatRefusal::DegenerateImpactor),
            "a body of no radius has no defined energy, and says so differently"
        );
        // And where it CAN be formed the law is the kinetic energy and not merely an overflow generator:
        // E = (2/3) * pi * r^3 * rho * U^2, which at unit radius, density and speed is 2.0944 J.
        let unit = impact_kinetic_energy_j(Impactor {
            radius: Fixed::ONE,
            velocity: Fixed::ONE,
            density: Fixed::ONE,
        })
        .expect("a unit body is representable");
        let want = 2.0 / 3.0 * std::f64::consts::PI;
        assert!(
            (unit.to_f64_lossy() - want).abs() < 1e-6,
            "E(r=1, rho=1, U=1) must be (2/3)pi = {want}, got {}",
            unit.to_f64_lossy()
        );
    }

    #[test]
    fn every_impact_records_a_row_and_the_cross_scale_rule_gates_the_raster() {
        // THE CROSS-SCALE WRITE RULE (rows not rasters, keep the large-basin feedback). Every drawn impact whose
        // crater resolves records a discrete ROW; only a crater at or above the convective cell size ALSO
        // rasterizes into the coarse province field. Run the same bombardment against two cell sizes.
        let (w, h) = (41usize, 41usize);
        let n = w * h;
        let params = [mantle_params(50)];
        let run = |cell_size: Fixed| -> DeepTimeState {
            let mut flux = flux_params(30, 100, 64);
            flux.forces.cell_size = cell_size;
            let mut state = young_state(n, 1800);
            for tick in 0..10u64 {
                state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(50))
                    .expect("steps");
                state = bombard_tick(&state, w, h, &flux, 0x00C0_FFEE, tick, Fixed::from_int(50));
            }
            state
        };
        // A cell far larger than any crater the flux draws (the ~thousand-km province scale against kilometre-
        // class craters): every crater is SUB-CELL, so it writes a ROW only and the coarse field stays untouched.
        let sub_cell = run(Fixed::from_int(10_000_000)); // a 10,000 km cell: no crater reaches it
        assert!(
            !sub_cell.craters.is_empty(),
            "a sub-cell impact still records a discrete crater row"
        );
        assert_eq!(
            sub_cell.craters.len() as u64,
            sub_cell.impact_count,
            "the crater count is the number of rows"
        );
        assert!(
            sub_cell.impact_relief_m.iter().all(|&r| r == Fixed::ZERO),
            "a sub-cell crater writes NO raster into the coarse province field (rows not rasters)"
        );
        // A cell far below the crater sizes (a 2 km cell against kilometre-class craters): the SAME craters now
        // exceed the cell size, so each ALSO rasterizes into the province field (the large-basin feedback path).
        let basin = run(Fixed::from_int(2000));
        assert!(!basin.craters.is_empty(), "the same impacts record rows");
        assert!(
            basin.impact_relief_m.iter().any(|&r| r != Fixed::ZERO),
            "a crater at or above the cell size rasterizes into the province field (the cross-scale feedback)"
        );
        // The large-basin raster still conserves mass to the bit (the excavated bowl equals the deposited blanket).
        let sum: i128 = basin
            .impact_relief_m
            .iter()
            .map(|r| r.to_bits() as i128)
            .sum();
        assert_eq!(sum, 0, "the large-basin raster conserves mass to the bit");
        // The two runs drew the SAME craters (same seed and draw stream); only the raster gating differs.
        assert_eq!(
            sub_cell.craters, basin.craters,
            "the cross-scale rule gates the raster, not the row draw: the rows are identical"
        );
    }

    // --- the support-bound collapse slice ---

    // The crust MECHANICAL-STRENGTH parameters for the support-bound collapse tests. The crust and mantle
    // densities are representative silicate values; the crust shear modulus (~44 GPa) and the per-class strength
    // knockdown (~0.015) are ILLUSTRATIVE stand-ins for the crust's own DERIVED shear modulus and the owner's
    // reserved-with-basis knockdown, chosen at physically-anchored crustal-rock figures only to exercise the
    // derive-down: they drive [`derived_crust_yield_pa`] through the REAL [`operative_shear_strength_gpa`], so the
    // chain is proven and the reserved 1e8 Pa literal is not used. g is Mars-class. This lands a support bound of
    // order the ~8 to 10 km class-grade value the reserved 1e8 gave, now from the crust's OWN strength.
    fn support_bound_params() -> SupportBoundParams {
        SupportBoundParams {
            // The SAME crust and mantle densities the melt transaction reads, in this struct's grams per cubic
            // centimetre, off the one constant, so the collapse and the ledger cannot disagree about what a
            // kilometre of crust weighs.
            crust_density: Fixed::from_int(CRUST_DENSITY_KG_M3),
            mantle_density: Fixed::from_int(MANTLE_DENSITY_KG_M3),
            crust_shear_modulus_gpa: Fixed::from_int(44), // ~44 GPa (illustrative crustal shear modulus)
            strength_knockdown: Fixed::from_ratio(15, 1000), // ~0.015 (illustrative reserved-with-basis stand-in)
            gravity_m_per_s2: Fixed::from_ratio(37, 10),     // 3.7 m/s^2, Mars-class
        }
    }

    /// Give a scaffold state a crust field by THICKNESS, through the ledger rather than around it: the crust
    /// stock is the areal mass those thicknesses carry and the thickness field is read back off the stock, so the
    /// state a collapse test starts from is one the melt transaction could have produced. Writing
    /// `crust_thickness_km` alone would leave the stock at zero, and the collapse (which moves the stock) would
    /// then have nothing to move, so the test would pass while proving nothing.
    fn with_crust_thickness_km(state: &mut DeepTimeState, thickness_km: &[Fixed]) {
        let rho = Fixed::from_int(CRUST_DENSITY_KG_M3);
        state.crust_areal_mass = thickness_km
            .iter()
            .map(|t| areal_mass_mg_per_m2(*t, rho).expect("a scaffold crust is representable"))
            .collect();
        state.crust_thickness_km = state
            .crust_areal_mass
            .iter()
            .map(|m| thickness_km_from_areal_mass(*m, rho).expect("and reads back"))
            .collect();
    }

    // Each column's Airy isostatic relief (elevation above the field datum, kilometres): (max relief, min relief,
    // amplitude), the physical relief the support bound governs.
    fn relief_stats(s: &DeepTimeState, p: &SupportBoundParams) -> (f64, f64, f64) {
        let elev: Vec<Fixed> = s
            .crust_thickness_km
            .iter()
            .map(|t| {
                airy_isostatic_elevation(p.crust_density, p.mantle_density, *t)
                    .expect("the crust floats")
            })
            .collect();
        let datum = relief_datum(&elev)
            .expect("the datum resolves")
            .to_f64_lossy();
        let mut max = f64::MIN;
        let mut min = f64::MAX;
        for e in &elev {
            let r = e.to_f64_lossy() - datum;
            max = max.max(r);
            min = min.min(r);
        }
        (max, min, max - min)
    }

    // The support bound (km) the derived crust yield lands at, recomputed independently for the assertions.
    fn support_bound_km(p: &SupportBoundParams) -> f64 {
        let sigma_y_pa = derived_crust_yield_pa(p.crust_shear_modulus_gpa, p.strength_knockdown)
            .expect("the crust yield derives")
            .to_f64_lossy();
        sigma_y_pa / (p.crust_density.to_f64_lossy() * p.gravity_m_per_s2.to_f64_lossy()) / 1000.0
    }

    #[test]
    fn the_support_bound_reads_the_crust_derived_yield_not_the_reserved_literal() {
        // THE DERIVE-DOWN: the bound reads the crust's OWN operative shear strength (the Frenkel ideal G/(2*pi)
        // scaled by the per-class knockdown), NOT the reserved 1e8 Pa literal. A stiffer crust derives a higher
        // yield, so the bound TRACKS the crust's strength rather than a constant.
        let soft = derived_crust_yield_pa(Fixed::from_int(44), Fixed::from_ratio(15, 1000))
            .expect("derives");
        let stiff = derived_crust_yield_pa(Fixed::from_int(80), Fixed::from_ratio(15, 1000))
            .expect("derives");
        assert!(
            stiff > soft,
            "a stiffer crust derives a higher yield strength, got {} vs {}",
            stiff.to_f64_lossy(),
            soft.to_f64_lossy()
        );
        // The derived value is of order the class-grade crustal yield (~1e8 Pa ~ 100 MPa, the frictional-brittle
        // bound the reserved literal encoded), now READ from the crust's own strength: a cross-validation.
        assert!(
            soft.to_f64_lossy() > 1e7 && soft.to_f64_lossy() < 1e9,
            "the derived crustal yield lands at the ~1e8 Pa class-grade scale, got {} Pa",
            soft.to_f64_lossy()
        );
        // A degenerate strength routes to None (no fabricated yield), the fail-loud escalation.
        assert!(
            derived_crust_yield_pa(Fixed::ZERO, Fixed::from_ratio(15, 1000)).is_none(),
            "no shear modulus, no derived yield"
        );
        assert!(
            derived_crust_yield_pa(Fixed::from_int(44), Fixed::ZERO).is_none(),
            "no knockdown, no derived yield"
        );
    }

    #[test]
    fn the_support_bound_collapse_relaxes_relief_and_conserves_mass() {
        let params = support_bound_params();
        let bound_km = support_bound_km(&params);
        // A physical crustal bound of order the class-grade value, from the crust's OWN strength.
        assert!(
            bound_km > 1.0 && bound_km < 30.0,
            "the derived support bound is a physical few-to-tens-of-km relief, got {bound_km:.2} km"
        );

        // A relief field with one column standing far above the datum (a tall volcanic province) over a baseline
        // crust, so its isostatic relief exceeds the support bound and must collapse.
        let n = 25usize;
        let mut thicknesses = vec![Fixed::from_int(30); n]; // 30 km baseline crust
        thicknesses[12] = Fixed::from_int(200); // a 200 km province, unsupportably tall
        let mut state = young_state(n, 1800);
        with_crust_thickness_km(&mut state, &thicknesses);

        let (max_before, _min_before, amp_before) = relief_stats(&state, &params);
        assert!(
            max_before > bound_km,
            "the tall province starts OVER the support bound ({max_before:.2} km relief vs {bound_km:.2} km bound)"
        );

        // The conserved crust MASS bit-sum before the collapse. The stock is what the ledger conserves, so it is
        // what the invariant is asserted on; the thickness is its readout.
        let sum_before: i128 = state
            .crust_areal_mass
            .iter()
            .map(|m| m.to_bits() as i128)
            .sum();

        let relaxed = relax_to_support_bound(&state, &params);

        // MASS conserved to the bit: the redistribution moves crust between columns, never creating or destroying
        // it (the same discipline the bombardment delta uses, the bit-sum invariant).
        let sum_after: i128 = relaxed
            .crust_areal_mass
            .iter()
            .map(|m| m.to_bits() as i128)
            .sum();
        assert_eq!(
            sum_before, sum_after,
            "the support-bound collapse conserves the crust mass bit-sum exactly (mass moved, never created or lost)"
        );
        // And it is a LATERAL move within the crust: the source stock is not touched, so the ledger's whole
        // crust-plus-source total is invariant across the collapse as well as within it.
        assert_eq!(
            state.source_areal_mass, relaxed.source_areal_mass,
            "the collapse moves crust that already exists; it neither extracts from nor returns to the source"
        );
        // The GEOMETRY FOLLOWED THE LEDGER, which is the property that stops a column from holding less
        // topography and the same mass. Every moved column's thickness is its moved stock's own readout.
        let rho = Fixed::from_int(CRUST_DENSITY_KG_M3);
        for (i, m) in relaxed.crust_areal_mass.iter().enumerate() {
            assert_eq!(
                relaxed.crust_thickness_km[i],
                thickness_km_from_areal_mass(*m, rho).expect("reads back"),
                "column {i}'s thickness is not the readout of its crust stock"
            );
        }

        // The relief is now WITHIN the support bound: the over-bound province collapsed to the bound (to within
        // the sub-nanometre apportionment residual), the tall topography relaxed by lower-crustal flow.
        let (max_after, _min_after, amp_after) = relief_stats(&relaxed, &params);
        assert!(
            max_after <= bound_km + 1e-6,
            "every column's relief is within the derived support bound after the collapse, got {max_after:.4} km vs {bound_km:.4} km"
        );
        // It collapsed TO the bound (not far below): the province relaxed to the supportable height, not to zero.
        assert!(
            max_after > bound_km - 1e-2,
            "the province relaxed to the bound, not far below it, got {max_after:.4} km vs {bound_km:.4} km"
        );
        // The relief amplitude fell (the peak came down and the lows filled with the shed crust).
        assert!(
            amp_after < amp_before,
            "the collapse reduced the relief amplitude, got {amp_after:.2} km vs {amp_before:.2} km"
        );
    }

    #[test]
    fn the_collapse_is_deterministic() {
        let params = support_bound_params();
        let mut state = young_state(16, 1800);
        let mut th = vec![Fixed::from_int(25); 16];
        th[3] = Fixed::from_int(180);
        th[10] = Fixed::from_int(150);
        with_crust_thickness_km(&mut state, &th);
        let a = relax_to_support_bound(&state, &params);
        let b = relax_to_support_bound(&state, &params);
        assert_eq!(
            a.crust_areal_mass, b.crust_areal_mass,
            "the collapse is a pure function of the state and parameters, replaying bit-for-bit"
        );
        assert_eq!(
            a.crust_thickness_km, b.crust_thickness_km,
            "and its thickness readout replays with it"
        );
        assert_ne!(
            a.crust_areal_mass, state.crust_areal_mass,
            "the scaffold is over the bound, so this test is comparing two real collapses"
        );
    }

    #[test]
    fn a_relief_already_within_the_bound_is_a_no_op() {
        // A gently varying crust whose isostatic relief is already below the support bound: no column collapses,
        // so the field returns unchanged (dormant), never a fabricated move.
        let params = support_bound_params();
        let mut state = young_state(9, 1800);
        let gentle: Vec<Fixed> = [30, 32, 31, 29, 33, 30, 31, 32, 30]
            .iter()
            .map(|k| Fixed::from_int(*k))
            .collect();
        with_crust_thickness_km(&mut state, &gentle);
        let (max_before, _, _) = relief_stats(&state, &params);
        assert!(
            max_before < support_bound_km(&params),
            "the field starts within the support bound"
        );
        let relaxed = relax_to_support_bound(&state, &params);
        assert_eq!(
            relaxed.crust_areal_mass, state.crust_areal_mass,
            "a supportable relief moves no crust"
        );
        assert_eq!(
            relaxed.crust_thickness_km, state.crust_thickness_km,
            "a supportable relief is left unchanged"
        );
    }

    #[test]
    fn a_foundering_crust_denser_than_the_mantle_is_left_unchanged() {
        // A crust denser than the mantle (k <= 0) FOUNDERS rather than standing as topography: the tall-relief
        // collapse does not apply (delamination is a separate regime), so the field is returned unchanged.
        let mut params = support_bound_params();
        params.crust_density = Fixed::from_int(3600); // 3600 > 3300 kg/m^3 mantle: founders
        let mut state = young_state(9, 1800);
        let mut th = vec![Fixed::from_int(30); 9];
        th[4] = Fixed::from_int(200);
        with_crust_thickness_km(&mut state, &th);
        let relaxed = relax_to_support_bound(&state, &params);
        assert_eq!(
            relaxed.crust_areal_mass, state.crust_areal_mass,
            "a foundering crust is not collapsed as topography"
        );
    }

    #[test]
    fn the_collapse_is_soft_on_a_degenerate_yield_or_gravity() {
        let base = support_bound_params();
        let mut state = young_state(9, 1800);
        let mut th = vec![Fixed::from_int(30); 9];
        th[4] = Fixed::from_int(200);
        with_crust_thickness_km(&mut state, &th);
        // A zero shear modulus: no derived yield, so no bound, so no collapse (soft, unchanged).
        let no_strength = SupportBoundParams {
            crust_shear_modulus_gpa: Fixed::ZERO,
            ..base
        };
        assert_eq!(
            relax_to_support_bound(&state, &no_strength).crust_areal_mass,
            state.crust_areal_mass,
            "a crust with no derived strength is left unchanged (fail-soft, no fabricated bound)"
        );
        // A non-positive gravity: no gravitational load, no bound, unchanged.
        let no_g = SupportBoundParams {
            gravity_m_per_s2: Fixed::ZERO,
            ..base
        };
        assert_eq!(
            relax_to_support_bound(&state, &no_g).crust_areal_mass,
            state.crust_areal_mass,
            "a non-positive gravity is left unchanged"
        );
        // The scaffold IS over the bound at the real parameters, so the two soft refusals above are refusals and
        // not a field that had nothing to collapse either way.
        assert_ne!(
            relax_to_support_bound(&state, &base).crust_areal_mass,
            state.crust_areal_mass,
            "at the real parameters this scaffold does collapse, so the refusals above are discriminating"
        );
    }
}
