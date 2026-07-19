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

//! The interior convection-evolution subsystem (genesis-forward geology arc): it composes the merged mantle
//! floor law-forms (`crates/physics/src/laws.rs`) into one per-step column update and drives it to a bounded
//! steady state with C's fixed-cap iterative solver (`civsim_world::solve::fixed_cap_solve`). The floor stays
//! a set of single physical relations; the composition into a per-step evolution lives here, the same split
//! the productivity and matter-cycle subsystems use.
//!
//! One step reads a column's temperature contrast with its cold reference, forms the buoyancy source
//! ([`laws::thermal_density_anomaly`]) and the Rayleigh number ([`laws::rayleigh_number`]), latches the
//! convection onset ([`laws::threshold_latch`], so convection fires once the Rayleigh number crosses the
//! derived critical value and stays on), and evolves the column temperature ([`laws::internal_heat_evolution`])
//! under the radiogenic heat production minus the conductive surface loss (the Fourier flux
//! [`laws::conduction`] over the column mass, so the loss grows with the contrast and gives the restoring
//! force the steady state relaxes onto) plus, once convecting, the convective heat the buoyant flow carries
//! out ([`laws::stokes_velocity`] feeding [`laws::heat_advection`]). No authored convection knob: the onset
//! is the derived critical Rayleigh number, the flow the derived Stokes 2/9, the buoyancy the real material
//! thermal expansion. Determinism holds by construction: fixed-point kernels, a monotone latch, and C's
//! bounded integer-residual solve (never an unbounded until-converged spin), so the solve tolerance and cap
//! are a determinism bound, not a physical knob.
//!
//! Byte-neutral: this subsystem is defined and unit-tested against a SYNTHETIC column state but armed by no
//! scenario, so the canonical pins hold. The resident-field wiring (reading and writing A's `GeodynamicColumn`)
//! and the plate-domain identity (C's `civsim_world::label` connected-components) are the follow-on slices,
//! sequenced behind A's contract reaching main.

use civsim_core::Fixed;
use civsim_physics::laws;
use civsim_world::solve::{fixed_cap_solve, SolveOutcome};
use civsim_world::terrain::{classify_relief, relief_datum, TerrainRelief};

use civsim_foundation::material::{GeodynamicColumn, GeodynamicField};

/// The resident state of one interior column the convection solve evolves: its temperature and whether
/// convection has begun. The convection flag is the one-way Rayleigh-onset latch (once the Rayleigh number
/// has crossed the critical value it stays set), so the state records that the column has entered the
/// convecting regime, a relic the memoryless present-to-present kernels could not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnState {
    /// The column temperature (K).
    pub temperature: Fixed,
    /// Whether convection has begun (the one-way Rayleigh-onset latch).
    pub convecting: bool,
}

/// One interior column's fixed physical parameters for a convection step. Every field is a floor value or a
/// per-world datum the runner would read from the resident geodynamic state; here they are supplied by the
/// caller (the synthetic column under test, or a future `GeodynamicColumn` read). The engine bounds
/// (`ra_max`, `v_max`, `flux_max`) are representable caps and the solve tolerance is a determinism bound,
/// not physical knobs.
#[derive(Clone, Copy, Debug)]
pub struct ColumnParams {
    /// The cold reference temperature the column loses heat toward and its buoyancy contrast is taken against.
    pub reference_temperature: Fixed,
    /// Bulk density (kg/m^3).
    pub density: Fixed,
    /// Thermal conductivity (W/(m*K)).
    pub thermal_conductivity: Fixed,
    /// Volumetric thermal expansion (ppm/K), the real material value.
    pub thermal_expansion_ppm: Fixed,
    /// Gravity (m/s^2).
    pub gravity: Fixed,
    /// The convecting-layer depth (representable-scaled length; raw SI mantle depth overflows Q32.32).
    pub depth: Fixed,
    /// The buoyant PARCEL radius, the Stokes sphere radius [`civsim_physics::laws::stokes_velocity`] takes,
    /// NOT the planet's radius. Flagged 2026-07-18 because the fixture-cluster replacement was scoped to
    /// derive this field from "the planet's own radius", which is a category error: `v ~ drho g r^2 / eta` is
    /// the terminal velocity of a sphere of radius `r` settling through the fluid, so the planet's radius here
    /// would inflate `r^2` by roughly `(R/d)^2` and overflow the linear kernel besides. The derived source
    /// this struct already carries is the CONVECTIVE CELL scale: [`Self::ra_crit_wavenumber`] gives the
    /// cell half-wavelength as `pi / a_c` layer depths, so the parcel radius derives from `depth` and `a_c`
    /// and stays in the same regime as the onset threshold by construction.
    pub radius: Fixed,
    /// Dynamic viscosity (representable-scaled Pa*s).
    ///
    /// THE REPRESENTATION NOTE, measured 2026-07-18 so the next attempt does not rediscover it. A real mantle
    /// viscosity does not fit Q32.32: the built [`civsim_physics::convective_viscosity`] solve returns
    /// `ln(eta) ~ 53.9` for a Mars-class interior at 1600 K and 10.7 GPa, which is `eta ~ 2.5e23 Pa*s` against
    /// a `Fixed::MAX` of `2.1e9`. That is NOT fatal to this field, because `depth` is already declared in
    /// MEGAMETRES: the Rayleigh number is dimensionless, so a viscosity expressed in the matching `1e18 Pa*s`
    /// unit makes the linear kernel compute the TRUE dimensionless `Ra`. Checked numerically: the scaled pair
    /// reproduces the log-domain [`civsim_physics::laws::ln_rayleigh_number`] at `Ra ~ 1.66e4`, about ten
    /// times the rigid-rigid critical value, so the province convects without touching the `ra_max` guard. The
    /// blocker on this field is therefore the DIFFUSIVITY it needs as a solve input, never the representation.
    ///
    /// AND THE MEASUREMENT THAT SETTLES THE UNIT QUESTION FOR THE WHOLE KERNEL, taken 2026-07-19, because the
    /// ruled plan for the fixture-cluster replacement was "a FULL SI/log-space lift" and that plan cannot
    /// work. In SI-SECONDS the kernel's SOURCE TERM vanishes: a mantle's radiogenic heating is `~5e-12 W/kg`
    /// against a Q32.32 resolution of `2.33e-10`, which is `0.02 ulp`, so `H` and the conductive loss `L`
    /// both quantize to ZERO and `T + ((H - L)/c) dt` never moves the temperature. The failure is not a loss
    /// of precision at the margin, it is the physics disappearing at the source. At the other end of the same
    /// kernel `eta`, `d^3` and `rho * d` all overflow. SI-seconds has no window that holds this problem.
    ///
    /// The same four quantities in MEGAMETRES and MEGAYEARS land mid-window: `H` reads `158 J/(kg Myr)`, the
    /// convective velocity `9.5e-2 Mm/Myr`, `d^3` about `24 Mm^3`, and `kappa` about `3.2e-5 Mm^2/Myr`. So
    /// the correct target is a DECLARED unit system rather than SI, which is what this kernel already runs in
    /// informally. The original complaint was never that these units are wrong, it was that a bare value with
    /// no declared scale carries no correctness; declaring the scale answers it, and SI-ifying the kernel
    /// would break the physics to satisfy a preference for familiar units.
    pub viscosity: Fixed,
    /// Thermal diffusivity (m^2/s), k/(rho*c).
    ///
    /// DECLARED CONFLICT, TAGGED NOT FIXED (owner ruling 2026-07-16). This field is REDUNDANT: `kappa` is
    /// `k / (rho * c_p)`, and [`civsim_physics::laws::thermal_diffusivity`] already derives it from three
    /// fields this same struct carries. So the struct stores a SECOND, INDEPENDENT answer to a question it
    /// already contains the answer to, which is the redundant-parameter case: two numbers encoding one fact,
    /// with no gate comparing them.
    ///
    /// THEY DISAGREE BY 20x, the first time anyone looked. Every caller (including the production
    /// [`crate::deeptime::province_column_params`]) passes `density = 1`, `thermal_conductivity = 2`,
    /// `specific_heat = 10`, and `thermal_diffusivity = 0.01`. The banked law gives `2 / (1 * 10) = 0.2`.
    ///
    /// THE QUESTION IS UNDECIDABLE AT THIS SITE, AND THAT UNDECIDABILITY IS THE FINDING: no per-quantity scale
    /// is declared here, and a bare value with no declared scale carries no correctness at all, only a value.
    /// Four quantities in four undeclared scale systems is not a unit system; it is four independent claims
    /// with no shared referee. So neither 0.01 nor 0.2 can be called right from here, and no scale archaeology
    /// is owed on a fixture that was never anchored to anything.
    ///
    /// THE RESOLUTION IS REPLACEMENT, NOT ARBITRATION, and it lands with the geotherm arc rather than now:
    /// `rho` derives (the composition wire), `c_p` is Dulong-Petit on the banked mean atomic mass, `k` is the
    /// Hofmeister derived form, and `kappa` becomes COMPUTED-NEVER-STORED with this field RETIRED, all SI and
    /// newtyped. Correcting the fixture by fiat first would move production output TWICE for one truth, so the
    /// conflict is tagged here, the arc lands, and the pins move ONCE with a ledger entry recording the fixture
    /// cluster it replaced. The resulting shift (roughly 2.7x on the derived lid thickness, since `Ra` goes as
    /// `1/kappa` and the boundary layer as `Ra^(-1/3)`) is EXPECTED, and it is refereed by the relief and lid
    /// hindcast rows, never by either fixture value.
    ///
    /// THE REPLACEMENT WAS ATTEMPTED (2026-07-18) AND IS BLOCKED ON ONE ABSENT CITED COLUMN, measured rather
    /// than assumed, so the next attempt starts from the finding instead of repeating the survey. Three of the
    /// four thermal quantities DO derive today, on a forsterite mantle at the 300 K / 1 bar reference frame:
    /// `rho = 3223.2 kg/m^3` (the assemblage density over the derived mantle composition), `c_p = 1241.1
    /// J/(kg*K)` (Dulong-Petit on the assemblage mean atomic mass 0.020099 kg/mol), and `alpha_V = 40.1 ppm/K`
    /// (the Grueneisen relation at the cited measured `gamma = 1.29 +/- 0.05`). The fourth, `k`, does NOT.
    ///
    /// WHY `k` IS BLOCKED, and it is a DATA gap rather than a machinery gap. Both rungs of
    /// [`civsim_materials::conductivity`] key on ATOMS PER PRIMITIVE CELL, which the ladder's own docstring
    /// marks `(DATA)`. No data file in this repo carries that column, for any phase: it is a function
    /// parameter and a struct field and nothing else, and the only construction of a `PhaseConductivity` in
    /// the tree is a test fixture that writes the count as a literal. The top rung is blocked twice over,
    /// because no per-phase `kappa_298` column exists either. So the geotherm arc's scoping claim that the
    /// class variable "was already banked and already in Slack's own signature" holds for the SIGNATURE and
    /// fails for the BANK, and the sibling claim that "the geotherm's minerals have measured anchors" is
    /// false as of this writing. Both are corrected in `docs/working/GEOTHERM_ARC_SCOPE.md`.
    ///
    /// WHY IT IS LOAD-BEARING RATHER THAN A ROUNDING DETAIL. The count enters Slack's magnitude as
    /// `n^(-2/3)`. For forsterite, atoms per FORMULA UNIT is 7 and the true primitive cell (Pbnm, `Z = 4`) is
    /// 28, and the estimator at 1600 K reads 15.15 against 6.01 W/(m*K) across that range: a 2.5x spread set
    /// by a column nobody has. Substituting the formula-unit count for the cell count would be a quantity
    /// substitution wearing a derivation's clothes, so it is refused here. `k` then gates `kappa`, and `kappa`
    /// gates the [`civsim_physics::convective_viscosity`] solve (which takes it as an input), so ONE missing
    /// column holds three of the seven cluster fields. The cluster moves as a whole or not at all, per the
    /// ruling above: a partial replacement would widen this field's declared conflict from 20x to roughly
    /// 20000x (`0.01` stored against `k / (3223.2 * 1241.1)`) and would move the pins twice for one truth.
    ///
    /// A SECOND HOLE IN THE ESTIMATOR RUNG, found the same way. `PhaseConductivity::estimator_band` is
    /// caller-supplied and never defaulted, by deliberate design, but Slack's band is DECLARED numerically
    /// only for the simple class (roughly 3x symmetric); for a complex cell it is one-sided with no stated
    /// magnitude beyond "several-fold" and the one exhibit (rutile, ~43 against a measured ~9). A silicate
    /// mantle is the complex class, so the caller has no cited width to pass. Measured consequence: the
    /// aggregate returns `band = 0` on a value uncertain by several-fold, a silent zero-width claim the
    /// struct's own docstring warns about.
    ///
    /// WHAT UNBLOCKS IT: a cited per-phase `atoms_per_primitive_cell` column (a crystallographic count, from
    /// the space group and `Z`) for the registry's phases, and, for the measured rung the front lane wants, a
    /// cited per-phase `kappa_298` column. The estimator rung alone would ship a 2x-to-5x-high `k` into the
    /// run path with nothing comparing it, which is the overlap-sentinel case the ladder was built to make
    /// impossible. A cheaper-looking substitution is refused for a further reason worth recording: keying the
    /// count off atoms per FORMULA UNIT does not merely mis-value the magnitude, it makes four of the
    /// registry's eight phases (quartz 3, corundum 5, hematite 5, enstatite 5) land inside the `2 < n < 6`
    /// band the cited calibration does not place, so they REFUSE outright while their true cell counts (9,
    /// 10, 10, 80) are all comfortably in the complex class. The substitution breaks the mechanism for half
    /// the registry rather than degrading it.
    pub thermal_diffusivity: Fixed,
    /// Specific heat capacity (J/(kg*K)).
    pub specific_heat: Fixed,
    /// Radiogenic heat production (W/kg), the source term.
    pub heat_production: Fixed,
    /// The derived critical Rayleigh number (marginal-stability eigenvalue), the onset threshold.
    pub ra_crit: Fixed,
    /// The critical WAVENUMBER a_c of the SAME marginal-stability eigenvalue as `ra_crit`: the horizontal mode
    /// that goes unstable first, in units of inverse layer depth. It is carried alongside `ra_crit` so the two
    /// describe ONE boundary regime by construction (a rigid-rigid layer has the pair {Ra_crit ~ 1708, a_c ~
    /// 3.117}, a free-free layer {~657.5, ~2.221}), never a rigid `ra_crit` paired with a free-free aspect. The
    /// convecting-cell half-wavelength is `pi / a_c` layer depths, so a downstream lateral-scale derivation
    /// (the province cell aspect) reads `pi / a_c` rather than an independently-authored aspect. The convection
    /// step itself does not read this field; it is the wavenumber half of the eigenvalue, kept with the number
    /// half so a future marginal-stability solver can supply {Ra_crit, a_c, regime} jointly.
    pub ra_crit_wavenumber: Fixed,
    /// The representable Rayleigh cap (an engine bound).
    pub ra_max: Fixed,
    /// The representable velocity cap (an engine bound).
    pub v_max: Fixed,
    /// The representable conductive-flux cap (an engine bound).
    pub flux_max: Fixed,
    /// The representable convective-stress cap (an engine bound), the ceiling on the driving stress the
    /// lid-mobilization read compares to `mat.yield_strength`.
    pub stress_max: Fixed,
    /// The tick duration.
    pub dt: Fixed,
}

impl ColumnParams {
    /// The buoyant PARCEL radius this column's convection carries, DERIVED from the column's own convective
    /// cell scale rather than stored as a seventh fixture.
    ///
    /// The marginal-stability eigenvalue pair fixes both the onset threshold and the cell geometry: the
    /// critical wavenumber `a_c` sets a cell half-wavelength of `pi / a_c` layer depths, so a parcel rising
    /// through a layer of thickness `d` has the scale `r = d * pi / a_c`. Reading [`Self::ra_crit_wavenumber`]
    /// rather than a module constant is what keeps the parcel in the SAME regime as the onset threshold by
    /// construction: a column carrying a free-free eigenvalue pair gets the free-free cell scale, with no
    /// second place for the two to drift apart.
    ///
    /// This is the same geometry the viewer's province lateral scale uses (`pi / a_c`, the rigid-rigid pair
    /// giving roughly 1.008), stated here so the relationship is visible rather than rediscovered.
    ///
    /// Returns `None` when the wavenumber is non-positive or the product leaves the representable window,
    /// which is a refusal rather than a fallback radius.
    // @derives: the buoyant parcel radius <- the column's own layer depth + its critical wavenumber (cell half-wavelength)
    pub fn parcel_radius(&self) -> Option<Fixed> {
        if self.ra_crit_wavenumber <= Fixed::ZERO {
            return None;
        }
        Fixed::PI
            .checked_div(self.ra_crit_wavenumber)?
            .checked_mul(self.depth)
    }
}

/// The DERIVED thermal properties of one interior column, in SI, all of them together or none of them.
///
/// This type is how the atomicity ruling on [`ColumnParams::thermal_diffusivity`] stops being a comment and
/// starts being a constraint the compiler carries: the cluster "moves as a whole or not at all", so the
/// derivation produces every field or it refuses naming what is missing. A caller cannot take the three that
/// derive today and leave the rest as fixtures, which is the partial replacement the ruling forbids because
/// it would widen the declared conflict from 20x to roughly 20000x.
///
/// The diffusivity is deliberately ABSENT rather than stored. It is `k / (rho * c_p)` exactly, so holding it
/// would be a fourth copy of three facts already here, which is the redundant-parameter defect the ruling was
/// written about. [`Self::thermal_diffusivity`] computes it.
///
/// SI throughout, because this states the PHYSICS. What representation the kernel runs in is a separate
/// question with a separate answer, recorded on [`ColumnParams`]: an SI-second kernel cannot work, since a
/// mantle's radiogenic heating is `~5e-12 W/kg` against a Q32.32 resolution of `2.33e-10`, which quantizes
/// the source term of the heat balance to ZERO. The conversion to the kernel's declared scale belongs at that
/// boundary, stated, rather than smuggled into these values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnThermalProperties {
    /// Density (kg/m^3), from the stable assemblage's own phases and their molar volumes.
    pub density_kg_m3: Fixed,
    /// Lattice-plus-radiative thermal conductivity (W/(m*K)) at the column's temperature, by the two-rung
    /// ladder over the assemblage's volume census.
    pub thermal_conductivity_w_m_k: Fixed,
    /// Specific heat (J/(kg*K)) by Dulong-Petit over the assemblage's own mean atomic mass.
    pub specific_heat_j_kg_k: Fixed,
    /// Volumetric thermal expansivity in parts per million per kelvin, the unit the kernel's
    /// [`civsim_physics::laws::thermal_density_anomaly`] reads.
    pub thermal_expansion_ppm_per_k: Fixed,
    /// The NATURAL LOG of dynamic viscosity (ln Pa*s). Held as a logarithm because the value itself cannot be
    /// represented: an interior viscosity is `~1e21 Pa*s` against a `Fixed::MAX` of `~2.1e9`.
    pub ln_viscosity_pa_s: Fixed,
}

impl ColumnThermalProperties {
    /// Thermal diffusivity `kappa = k / (rho * c_p)` (m^2/s), COMPUTED and never stored, so it cannot drift
    /// from the three facts it is made of. `None` if the product leaves the representable window.
    // @derives: a column's thermal diffusivity <- its own conductivity, density and specific heat
    pub fn thermal_diffusivity(&self) -> Option<Fixed> {
        let rho_cp = self.density_kg_m3.checked_mul(self.specific_heat_j_kg_k)?;
        if rho_cp <= Fixed::ZERO {
            return None;
        }
        self.thermal_conductivity_w_m_k.checked_div(rho_cp)
    }
}

/// Why a column's thermal derivation refused, NAMING the join that is missing so the refusal is a work list
/// rather than a dead end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnDerivationRefusal {
    /// The composition reached no stable assemblage at this temperature and pressure.
    NoAssemblage,
    /// A quantity the assemblage should supply did not resolve; names which.
    NoQuantity {
        /// The quantity, for example `density` or `mean_atomic_mass`.
        quantity: String,
    },
    /// The conductivity ladder refused, carrying its own reason (a phase with no banked column, no rung, or
    /// an unplaceable cell count).
    Conductivity(String),
    /// The expansivity join refused, carrying its own reason (a phase with no banked gamma, bulk modulus or
    /// registry row).
    Expansivity(String),
    /// A JOIN this derivation needs has no implementation yet. This is the honest frontier rather than a
    /// data gap: the pieces exist and nothing composes them. Names the join and what it would take.
    NoJoinYet {
        /// The quantity that cannot be assembled.
        quantity: String,
        /// What composing it requires.
        needs: String,
    },
}

impl std::fmt::Display for ColumnDerivationRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnDerivationRefusal::NoAssemblage => write!(
                f,
                "the composition reaches no stable assemblage at these conditions, so no column property \
                 can be derived from it"
            ),
            ColumnDerivationRefusal::NoQuantity { quantity } => {
                write!(f, "the assemblage did not resolve its {quantity}")
            }
            ColumnDerivationRefusal::Conductivity(reason) => {
                write!(f, "the conductivity ladder refused: {reason}")
            }
            ColumnDerivationRefusal::Expansivity(reason) => {
                write!(f, "the expansivity join refused: {reason}")
            }
            ColumnDerivationRefusal::NoJoinYet { quantity, needs } => write!(
                f,
                "{quantity} has no assembled derivation yet; it needs {needs}. Refused rather than \
                 defaulted: a fixture here would move the cluster PARTIALLY, which the atomicity ruling \
                 forbids because the derived and fixture halves would then span two unit systems"
            ),
        }
    }
}

impl std::error::Error for ColumnDerivationRefusal {}

/// The banked reference tables a column derivation reads, grouped so the derivation takes ONE borrow rather
/// than four positional ones.
///
/// They travel together because every one of them is consulted for every phase in a census, and a caller
/// holding three of the four has nothing useful. Grouping them also removes the positional hazard: four
/// same-shaped `&Table` parameters in a row are four chances to transpose two of them, and the compiler
/// catches none of it.
pub struct BankedTables<'a> {
    /// The candidate-phase thermodynamic registry (compositions, molar volumes).
    pub registry: &'a civsim_physics::petrology_data::PhaseRegistry,
    /// The periodic table (atomic masses, valences).
    pub periodic: &'a civsim_physics::periodic::PeriodicTable,
    /// The cited crystallographic and `kappa_298` column.
    pub conductivity: &'a civsim_physics::phase_conductivity::PhaseConductivityTable,
    /// The two-rung Grueneisen table.
    pub gruneisen: &'a civsim_physics::gruneisen::GruneisenTable,
    /// The mineral elastic-moduli table.
    pub moduli: &'a civsim_physics::mineral_moduli::MineralModuli,
}

/// Derive every thermal property of an interior column from the world's OWN composition, ATOMICALLY.
///
/// The chain, each link a banked derivation rather than a value: the composition minimizes to a stable
/// assemblage; the assemblage's phases and molar volumes give its density; its mean atomic mass gives the
/// Dulong-Petit specific heat; its volume census runs the two-rung conductivity ladder; and the diffusivity
/// falls out of those three rather than being stored beside them.
///
/// TWO OF THE SEVEN STILL REFUSE, and that is the point of returning a `Result` rather than a struct of
/// options. The expansivity needs an assemblage-level join over the Grueneisen gamma, the molar heat
/// capacity, the bulk modulus and the molar volume, and those live in two tables (`mineral_moduli` and the
/// phase registry) with no function joining them by phase key. The viscosity needs the creep-candidate wire
/// into `convective_viscosity::solve_ln_effective_viscosity`, which additionally wants the diffusivity this
/// function computes and a mid-layer pressure the province call site does not yet form. Until both land, this
/// refuses, the fixture cluster stays whole, and the pins do not move. That is the ruling enforced by a type.
// @derives: an interior column's thermal properties <- the world's own composition through the banked assemblage, ladder and Dulong-Petit derivations
pub fn derive_column_thermal_properties(
    composition: &[(String, Fixed)],
    temperature_k: Fixed,
    pressure_bar: Fixed,
    tables: &BankedTables<'_>,
) -> Result<ColumnThermalProperties, ColumnDerivationRefusal> {
    let missing = |q: &str| ColumnDerivationRefusal::NoQuantity {
        quantity: q.to_string(),
    };
    let assemblage = civsim_physics::petrology::stable_assemblage(
        composition,
        temperature_k,
        pressure_bar,
        tables.registry,
    )
    .ok_or(ColumnDerivationRefusal::NoAssemblage)?;

    // Density: the registry's own molar volumes and masses, in g/cm^3, lifted to kg/m^3.
    let density_g_cm3 = civsim_physics::petrology::assemblage_density(
        &assemblage,
        tables.registry,
        tables.periodic,
    )
    .ok_or_else(|| missing("density"))?;
    let density_kg_m3 = density_g_cm3
        .checked_mul(Fixed::from_int(1000))
        .ok_or_else(|| missing("density"))?;

    // Specific heat: Dulong-Petit over the assemblage's own mean atomic mass.
    let mean_atomic_mass = civsim_physics::petrology::assemblage_mean_atomic_mass_kg_per_mol(
        &assemblage,
        tables.registry,
        tables.periodic,
    )
    .ok_or_else(|| missing("mean_atomic_mass"))?;
    let specific_heat_j_kg_k =
        civsim_physics::young_thermal::dulong_petit_specific_heat(mean_atomic_mass)
            .ok_or_else(|| missing("specific_heat"))?;

    // Conductivity: the volume census through the two-rung ladder. Volume fractions rather than molar
    // amounts, because a Bruggeman mixture weights by the volume each phase occupies.
    let volume_census =
        civsim_physics::petrology::assemblage_volume_fractions(&assemblage, tables.registry)
            .ok_or_else(|| missing("volume_fractions"))?;
    let mut rows = Vec::with_capacity(volume_census.len());
    for (name, fraction) in &volume_census {
        let row = civsim_materials::conductivity::phase_conductivity_from_banked(
            name,
            tables.conductivity,
            tables.gruneisen,
            tables.registry,
            tables.periodic,
            Fixed::ZERO,
        )
        .map_err(|e| ColumnDerivationRefusal::Conductivity(e.to_string()))?;
        rows.push((row, *fraction));
    }
    let borrowed: Vec<(&civsim_materials::conductivity::PhaseConductivity, Fixed)> =
        rows.iter().map(|(r, f)| (r, *f)).collect();
    let thermal_conductivity_w_m_k =
        civsim_materials::conductivity::assemblage_conductivity(&borrowed, temperature_k)
            .map_err(|e| ColumnDerivationRefusal::Conductivity(e.to_string()))?
            .ok_or_else(|| missing("conductivity"))?
            .conductivity;

    // THE TWO FRONTIERS, each asked for honestly and each refusing by name. Written as calls rather than as
    // an early return so the three derivations above are EXERCISED on every attempt: a chain that is never
    // run is a chain that rots, and this way a break in the assemblage, ladder or Dulong-Petit path surfaces
    // here rather than waiting for the frontier to close.
    let thermal_expansion_ppm_per_k =
        derive_assemblage_expansivity_ppm_per_k(&volume_census, tables)?;
    let ln_viscosity_pa_s = derive_column_ln_viscosity()?;

    Ok(ColumnThermalProperties {
        density_kg_m3,
        thermal_conductivity_w_m_k,
        specific_heat_j_kg_k,
        thermal_expansion_ppm_per_k,
        ln_viscosity_pa_s,
    })
}

/// The assemblage's volumetric expansivity in PPM per kelvin, the unit the kernel's
/// [`civsim_physics::laws::thermal_density_anomaly`] reads.
///
/// FRONTIER CLOSED 2026-07-19. The join now exists as
/// [`civsim_materials::properties::assemblage_volumetric_expansivity_per_k`], which derives each phase's
/// `alpha = gamma C_v / (K V_m)` from four banked columns and mixes them by volume. Its magnitude is checked
/// against the MEASURED forsterite expansivity (roughly 40 ppm/K at mantle temperature) reproduced from
/// columns never fitted to it, which is what makes that a check rather than a circular one. Worth recording
/// as the reason to derive it: the fixture being replaced reads 30 ppm/K, and forsterite derives near 40.
///
/// The per-kelvin result is scaled to ppm here, at the boundary where the kernel's unit is declared, rather
/// than inside the derivation, so the physics function returns the physical quantity and this conversion is
/// visible at the site that needs it.
fn derive_assemblage_expansivity_ppm_per_k(
    volume_census: &[(String, Fixed)],
    tables: &BankedTables<'_>,
) -> Result<Fixed, ColumnDerivationRefusal> {
    let per_k = civsim_materials::properties::assemblage_volumetric_expansivity_per_k(
        volume_census,
        tables.registry,
        tables.moduli,
        tables.gruneisen,
    )
    .map_err(|e| ColumnDerivationRefusal::Expansivity(e.to_string()))?;
    per_k
        .checked_mul(Fixed::from_int(1_000_000))
        .ok_or_else(|| ColumnDerivationRefusal::NoQuantity {
            quantity: "thermal_expansion_ppm_per_k".to_string(),
        })
}

/// The column's log viscosity (ln Pa*s). THE SECOND FRONTIER, refusing by name.
///
/// [`civsim_physics::convective_viscosity::solve_ln_effective_viscosity`] is built and returns exactly this
/// quantity, so the obstacle is its inputs rather than the solve. It takes creep candidates that nothing
/// assembles for a province, a `thermal_diffusivity` that [`ColumnThermalProperties::thermal_diffusivity`]
/// can now supply, and an `eval_pressure_gpa` the province call site does not form (composable there as
/// `rho g d / 2`, since the depth, gravity and density are all in scope, but not composed today).
fn derive_column_ln_viscosity() -> Result<Fixed, ColumnDerivationRefusal> {
    Err(ColumnDerivationRefusal::NoJoinYet {
        quantity: "ln_viscosity_pa_s".to_string(),
        needs: "the creep-candidate wire into `convective_viscosity::solve_ln_effective_viscosity`, plus a \
                mid-layer `eval_pressure_gpa` the province call site does not yet form"
            .to_string(),
    })
}

/// One convection-evolution step: compose the merged floor law-forms into the next column state.
// @derives[column_convection]: the interior column temperature and convection-onset state <- the merged floor law-forms (thermal_density_anomaly, rayleigh_number, threshold_latch, stokes_velocity, heat_advection, internal_heat_evolution, conduction) over the column's own physical parameters; no authored convection knob (Ra_crit is the derived marginal-stability eigenvalue, the Stokes coefficient the derived 2/9, the buoyancy the real material thermal expansion). A NEW derivation (not a retired-floor replacement), now covered by the liveness gate broadened to any derived output and any input source (task #46): the derive_gate registry carries a column_convection row (category new-derivation) whose probe perturbs the ColumnParams heat_production (a resident-field input) and asserts the stepped temperature responds.
pub fn convection_step(state: &ColumnState, p: &ColumnParams) -> ColumnState {
    // The column's temperature contrast with its cold reference drives buoyancy, conduction, and advection.
    let delta_t = state.temperature - p.reference_temperature;

    // Buoyancy source: the thermal density excess (negative, and rising, when the column is hotter).
    let delta_rho = laws::thermal_density_anomaly(p.density, p.thermal_expansion_ppm, delta_t);

    // The Rayleigh number and the one-way convection-onset latch (fires once Ra crosses the derived Ra_crit).
    let rayleigh = laws::rayleigh_number(
        delta_rho,
        p.gravity,
        p.depth,
        p.viscosity,
        p.thermal_diffusivity,
        p.ra_max,
    );
    let convecting = laws::threshold_latch(rayleigh, p.ra_crit, state.convecting);

    // Conductive surface loss as specific power: the Fourier flux over the column mass per area, so the loss
    // grows with the contrast, the restoring force the steady state relaxes onto.
    let flux = laws::conduction(
        p.thermal_conductivity,
        Fixed::ONE,
        state.temperature,
        p.reference_temperature,
        p.depth,
        p.flux_max,
    );
    let mass_per_area = p.density.checked_mul(p.depth).unwrap_or(Fixed::MAX);
    let conductive_loss = if mass_per_area > Fixed::ZERO {
        flux.checked_div(mass_per_area).unwrap_or(Fixed::ZERO)
    } else {
        Fixed::ZERO
    };

    // Convective loss: once convecting, the buoyant flow carries heat out, augmenting conduction.
    let convective_loss = if convecting {
        let velocity = laws::stokes_velocity(delta_rho, p.gravity, p.radius, p.viscosity, p.v_max);
        laws::heat_advection(velocity, p.specific_heat, delta_t, p.depth)
    } else {
        Fixed::ZERO
    };

    let total_loss = conductive_loss.saturating_add(convective_loss);
    let temperature = laws::internal_heat_evolution(
        state.temperature,
        p.heat_production,
        total_loss,
        p.specific_heat,
        p.dt,
    );
    ColumnState {
        temperature,
        convecting,
    }
}

/// The column's CONDUCTIVE LOSS COEFFICIENT `k / (rho * d^2)` (per second, per kelvin of contrast): the
/// specific power a column sheds per kelvin it sits above its cold reference, read OFF THE COLUMN the kernel
/// will itself run rather than restated at a caller. It is exactly the coefficient [`convection_step`]
/// composes when it divides the Fourier flux by the column's mass per area, so a consumer that needs the
/// conductive balance (the interior-thermostat closure, which sets the radiogenic base so a column's
/// steady state lands on the world's own solidus) reads one home instead of carrying a second copy of the
/// column's conductivity and density.
///
/// WHY IT EXISTS AS A FUNCTION. The thermostat previously restated `conductivity` and `density` at its own
/// call site, so the SAME two facts lived in two places with nothing comparing them: the redundant-parameter
/// diamond this repo keeps paying for, one level up from the `kappa` versus `k / (rho * c_p)` conflict tagged
/// on [`ColumnParams::thermal_diffusivity`]. Collapsing the copies makes divergence structurally impossible
/// rather than test-detectable, and it means the geotherm arc's replacement of the fixture cluster moves the
/// thermostat with the kernel by construction. `the_loss_coefficient_twins_the_kernels_own_conductive_loss`
/// is the twin that proves the composition matches the kernel's.
///
/// `None` when the depth or the volumetric mass is non-positive (no conductive path), or on an arithmetic
/// overflow. Deterministic fixed-point.
pub fn conductive_loss_coefficient(p: &ColumnParams) -> Option<Fixed> {
    let mass_per_area = p.density.checked_mul(p.depth)?;
    if mass_per_area <= Fixed::ZERO || p.depth <= Fixed::ZERO {
        return None;
    }
    // k / (rho * d^2), composed in the kernel's own association: the Fourier flux `k * dT / d` over the mass
    // per area `rho * d`.
    p.thermal_conductivity
        .checked_div(p.density.checked_mul(p.depth.checked_mul(p.depth)?)?)
}

/// The continuous interior read-outs of one column, the CONTINUOUS state the resident contract stores (gate
/// ruling, #176): the stepped interior temperature, the Rayleigh number (the convective vigor a consumer
/// reads to derive whether the column convects), and the convective driving stress (the lid-mobilization
/// quantity). No discrete "convecting" flag is carried: the discrete condition is derived from the Rayleigh
/// number against the critical value at each consumer site, so convection can begin and, on a cooling world,
/// cease.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnReadout {
    /// The stepped interior temperature (K).
    pub temperature: Fixed,
    /// The Rayleigh number (dimensionless), the continuous convective vigor.
    pub rayleigh: Fixed,
    /// The convective driving stress (Pa) the interior flow exerts on the base of the lithosphere.
    pub convective_stress: Fixed,
}

/// Read the continuous interior quantities of a column and its stepped temperature, composing the same floor
/// law-forms [`convection_step`] threads, plus [`civsim_physics::laws::convective_stress`] for the
/// lid-driving stress. The Rayleigh number and the stress are evaluated at the input state (the buoyancy the
/// step responds to), and the temperature is the stepped result, so the three form the column's continuous
/// state going out of the tick. Deterministic fixed-point.
pub fn column_readout(state: &ColumnState, p: &ColumnParams) -> ColumnReadout {
    let delta_t = state.temperature - p.reference_temperature;
    let delta_rho = laws::thermal_density_anomaly(p.density, p.thermal_expansion_ppm, delta_t);
    let rayleigh = laws::rayleigh_number(
        delta_rho,
        p.gravity,
        p.depth,
        p.viscosity,
        p.thermal_diffusivity,
        p.ra_max,
    );
    let velocity = laws::stokes_velocity(delta_rho, p.gravity, p.radius, p.viscosity, p.v_max);
    // The shear length is the thermal BOUNDARY LAYER thickness, DERIVED (gate ruling, #176): the boundary layer
    // thins with convective vigor as `depth * (Ra_crit / Ra)^(1/3)`, so a vigorous mantle (Ra of order 1e6,
    // against a planetary Ra_crit of 1707.762) shears over a layer about a TENTH of its depth, concentrating
    // the driving stress. The derivation MOVED to
    // `laws::thermal_boundary_layer` when the LID GEOTHERM became its second consumer: the driving stress and
    // the geotherm must agree about how thick the lid is, so they read ONE law rather than two copies of the
    // same expression. Byte-identical across the move (the same operations in the same order).
    let length_scale = laws::thermal_boundary_layer(p.depth, rayleigh, p.ra_crit);
    let convective_stress =
        laws::convective_stress(p.viscosity, velocity, length_scale, p.stress_max);
    let next = convection_step(state, p);
    ColumnReadout {
        temperature: next.temperature,
        rayleigh,
        convective_stress,
    }
}

/// Drive the convection step to a bounded steady state with C's fixed-cap iterative solve: at most `cap`
/// steps, stopping the moment the integer temperature-change residual falls to or below `threshold`. Both
/// `cap` and `threshold` are determinism bounds (the solver terminates by construction, never an unbounded
/// until-converged spin), not physical knobs. Returns the solve outcome (the final state, the iteration
/// count, and whether the residual crossed the threshold within the cap).
pub fn convection_solve(
    initial: ColumnState,
    p: &ColumnParams,
    cap: u32,
    threshold: u64,
) -> SolveOutcome<ColumnState> {
    fixed_cap_solve(
        initial,
        cap,
        threshold,
        |s| convection_step(s, p),
        |a, b| a.temperature.to_bits().abs_diff(b.temperature.to_bits()),
    )
}

/// A column's decaying radiogenic heat source paired with its thermal state, for the secular thermal
/// history: over geological time the heat-producing isotope reservoir spends down (the memory primitive
/// [`laws::radiogenic_decay`]), so the radiogenic heat production ([`laws::radiogenic_heat`] over the
/// reservoir) falls and the interior cools, the spent-world relaxation the static-source convection step
/// cannot express on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecularState {
    /// The column's thermal state.
    pub column: ColumnState,
    /// The heat-producing isotope reservoir (concentration), spending down over the clock.
    pub reservoir: Fixed,
}

/// One secular step over a world-clock tick: decay the isotope reservoir, recompute the radiogenic heat
/// production it now supplies, and apply the convection step under that (falling) production. `decay_constant`
/// is the isotope's per-tick decay rate and `specific_heat_production` its heat per unit reservoir; the
/// step reuses [`convection_step`] with the recomputed heat production, so the whole convection composition
/// (buoyancy, onset, flow, advection, conduction) runs under a source that dies over deep time.
// @derives: the interior column's secular thermal history <- radiogenic_decay (the isotope reservoir spending down over the world clock) feeding radiogenic_heat (the falling heat production) into the convection step, so the interior warms under radiogenic heating and cools as the sources decay; no authored cooling knob, the source history is the decaying reservoir
pub fn secular_step(
    state: &SecularState,
    p: &ColumnParams,
    decay_constant: Fixed,
    specific_heat_production: Fixed,
) -> SecularState {
    let reservoir = laws::radiogenic_decay(state.reservoir, decay_constant, p.dt);
    let heat_production = laws::radiogenic_heat(reservoir, specific_heat_production);
    let column = convection_step(
        &state.column,
        &ColumnParams {
            heat_production,
            ..*p
        },
    );
    SecularState { column, reservoir }
}

/// March the secular step over `ticks` world-clock ticks, returning the interior's state after that span of
/// geological time. Deterministic and bounded by `ticks` (never an unbounded spin), the time-marching
/// counterpart to the fixed-`H` relaxation [`convection_solve`].
pub fn secular_history(
    initial: SecularState,
    p: &ColumnParams,
    decay_constant: Fixed,
    specific_heat_production: Fixed,
    ticks: u64,
) -> SecularState {
    let mut state = initial;
    for _ in 0..ticks {
        state = secular_step(&state, p, decay_constant, specific_heat_production);
    }
    state
}

/// Populate one column's INTERIOR fields on A's [`GeodynamicColumn`] contract from the interior chain,
/// SNAPSHOT-APPLY (gate ruling, #176): read the start-of-tick `snapshot` column and return the end-of-tick
/// column. The interior reads only the snapshot (its resident `temperature` and the surface lane's
/// `crustal_density`), so whichever lane evaluates first reads the same values and the boundary is
/// order-independent, no cross-lane evaluation order pinned. The interior writes its continuous state
/// (`temperature`, `rayleigh`, `convective_stress`) and the `isostatic_elevation` it derives by floating the
/// surface-written crust on the world's mantle; the surface lane's own fields pass through unchanged. A
/// missing density or thickness yields the zero-default elevation (the absence convention), never a
/// fabricated one.
///
/// `mantle_density` is DERIVED, never authored (gate ruling from the owner, #176): it is A's petrology kernel
/// [`derive_mantle_density`] over the world's mantle COMPOSITION at the mantle's temperature and pressure, so
/// no density is a bare per-world number. The caller passes the derived value (the derivation is threaded so
/// the boundary stays snapshot-clean). Byte-neutral: no scenario calls this yet, so it is a dormant capability
/// (the interior law-forms' pattern), and the arming (a scenario running it and the surface reading the
/// result) is the separately-sequenced step.
pub fn populate_interior_column(
    snapshot: GeodynamicColumn,
    p: &ColumnParams,
    mantle_density: Fixed,
) -> GeodynamicColumn {
    // The resident interior state carries only the continuous temperature; the discrete convecting condition is
    // derived inside the step from the Rayleigh number (a false prior latch makes the onset reversible, so a
    // cooling column can stop convecting), never a stored flag.
    let state = ColumnState {
        temperature: snapshot.temperature,
        convecting: false,
    };
    let readout = column_readout(&state, p);
    let isostatic_elevation = civsim_physics::geodynamics::airy_isostatic_elevation(
        snapshot.crustal_density,
        mantle_density,
        snapshot.crustal_thickness,
    )
    .unwrap_or(Fixed::ZERO);
    GeodynamicColumn {
        // The surface lane's fields pass through (the interior does not write them, snapshot-apply).
        crustal_density: snapshot.crustal_density,
        crustal_thickness: snapshot.crustal_thickness,
        // The interior lane's writes.
        isostatic_elevation,
        temperature: readout.temperature,
        convective_stress: readout.convective_stress,
        rayleigh: readout.rayleigh,
    }
}

/// Snapshot-apply the interior population over a whole [`GeodynamicField`]: read the start-of-tick `snapshot`
/// field and return the end-of-tick field, each column populated against the snapshot (order-independent, gate
/// ruling #176). The per-world interior parameters and mantle density are supplied by the caller (a future
/// scenario). A column with no resident state is not walked, so an EMPTY field yields an empty field and the
/// pass is byte-neutral over an unarmed geology; the walk is canonical `Coord3` order, so the fold is
/// reproducible and thread-invariant. Called by no scenario yet.
pub fn step_interior_field(
    snapshot: &GeodynamicField,
    p: &ColumnParams,
    mantle_density: Fixed,
) -> GeodynamicField {
    let mut next = GeodynamicField::new();
    for (coord, column) in snapshot.iter() {
        next.set(coord, populate_interior_column(column, p, mantle_density));
    }
    next
}

/// Derive the mantle density from the world's mantle COMPOSITION, never an authored number (gate ruling from
/// the owner, #176): A's petrology kernel ([`civsim_physics::petrology::crustal_density`], a GENERAL
/// composition-to-density derivation despite the crust-specific name) minimizes the stable mineral assemblage
/// of the composition at the mantle's temperature and pressure and reads its mass over volume, so the density
/// is what the material IS under its conditions, neither a fundamental constant nor a bare per-world scalar.
/// The mantle temperature is the interior heat chain's own thermal state (the column temperature the
/// convection evolution carries), and the pressure is the lithostatic pressure at the mantle's depth; a
/// reference-pressure first pass breaks the mild density-depends-on-pressure self-consistency (a short
/// fixed-point refinement is the follow-on, both derivations, nothing authored). Returns `None` when the
/// composition reaches no assemblage or a phase is missing from the data (fail-loud, never a fabricated
/// density). The isostasy floats the crust on this derived mantle density.
pub fn derive_mantle_density(
    mantle_composition: &[(String, Fixed)],
    mantle_temperature: Fixed,
    reference_pressure_bar: Fixed,
    registry: &civsim_physics::petrology_data::PhaseRegistry,
    table: &civsim_physics::periodic::PeriodicTable,
) -> Option<Fixed> {
    civsim_physics::petrology::crustal_density(
        mantle_composition,
        mantle_temperature,
        reference_pressure_bar,
        registry,
        table,
    )
}

/// The CONVECTING-MANTLE DEPTH (metres): the silicate mantle shell thickness `R_planet - R_core`, DERIVED from the
/// planet's own structure, never an authored layer thickness. The core is a sphere of the sinking metal-and-sulfide
/// fraction the differentiation set; from `core_mass / planet_mass = (R_core/R_planet)^3 * (rho_core/rho_mean)` the
/// core radius is `R_core = R_planet * cbrt(core_mass_fraction * rho_mean / rho_core)`, so the shell the interior
/// convection evolves over is `R_planet * (1 - cbrt(core_fraction * rho_mean / rho_core))`. Every input is a
/// capstone derivation: the radius from accretion, and the core mass fraction, the mean density, and the metal-core
/// density from the differentiation and the bulk-density derivation. `None` on a non-physical input (a non-positive
/// radius or density, a core fraction outside `[0, 1]`, or a core no denser than the mean, which could not have
/// sunk) or a degenerate result (a core volume filling the whole planet leaves no mantle), fail-loud, never a
/// fabricated depth. This is the derivation that retires the authored convecting-layer-depth fixture: the mantle
/// thickness the convection reads is what the planet's own structure IS. (The convection kernel's `depth` is a
/// representable-SCALED length, so the run-path wiring scales this SI metres value into the kernel's units; the
/// physical derivation is here, the scale conversion is the units plan's job.)
pub fn convecting_mantle_depth_m(
    planet_radius_m: Fixed,
    core_mass_fraction: Fixed,
    mean_density: Fixed,
    core_density: Fixed,
) -> Option<Fixed> {
    if planet_radius_m <= Fixed::ZERO
        || mean_density <= Fixed::ZERO
        || core_density <= Fixed::ZERO
        || core_mass_fraction < Fixed::ZERO
        || core_mass_fraction > Fixed::ONE
        || core_density <= mean_density
    {
        // A core no denser than the whole-planet mean could not have sunk to a core (differentiation needs the
        // metal denser than the silicate mean): not a differentiated planet, no distinct mantle shell.
        return None;
    }
    // The core VOLUME fraction, core_mass_fraction * (rho_mean / rho_core). A core no denser than the mean could
    // not be a sunk metal core, so the fraction must land in (0, 1); at the bounds there is no distinct mantle.
    let volume_fraction = core_mass_fraction
        .checked_mul(mean_density)?
        .checked_div(core_density)?;
    if volume_fraction <= Fixed::ZERO || volume_fraction >= Fixed::ONE {
        return None;
    }
    let core_radius = planet_radius_m.checked_mul(volume_fraction.cbrt())?;
    let depth = planet_radius_m.checked_sub(core_radius)?;
    if depth <= Fixed::ZERO {
        return None;
    }
    Some(depth)
}

/// The surface elevation a crust of a given COMPOSITION floats at, the composition-to-terrain wire the generated
/// world reads (the R1 override the owner ruled: a tile's terrain DERIVES from the substrate, never fractal
/// noise). It composes the two geology pieces already in the tree: the petrology-derived crustal density of the
/// composition ([`civsim_physics::petrology::crustal_density`], the general composition-to-density derivation),
/// and the Airy isostasy law against the mantle ([`civsim_physics::geodynamics::airy_isostatic_elevation`]). So a
/// tile's elevation is what the material at that place IS, not a noise field. The mantle density and the crustal
/// thickness are the column's own inputs (the interior lane refines the thickness). `None` when the composition
/// reaches no assemblage (fail-loud, never a fabricated elevation) or the isostasy inputs are degenerate.
/// Byte-neutral: no worldgen consumer is wired to it yet (the tile-axis wire is the next capstone slice); this is
/// the derived-elevation primitive that replaces the fractal-noise axis, the visible spine's foundation.
pub fn surface_elevation_from_composition(
    crust_composition: &[(String, Fixed)],
    mantle_density: Fixed,
    crustal_thickness: Fixed,
    temperature_k: Fixed,
    pressure_bar: Fixed,
    registry: &civsim_physics::petrology_data::PhaseRegistry,
    table: &civsim_physics::periodic::PeriodicTable,
) -> Option<Fixed> {
    let crust_density = civsim_physics::petrology::crustal_density(
        crust_composition,
        temperature_k,
        pressure_bar,
        registry,
        table,
    )?;
    civsim_physics::geodynamics::airy_isostatic_elevation(
        crust_density,
        mantle_density,
        crustal_thickness,
    )
}

/// One generated tile's DERIVED terrain: the surface elevation the geology gives it and the relief class it
/// classifies to. The generated-world tile whose terrain is what the substrate says, never fractal noise (Slice 0,
/// the R1 override). The surface material (the stable assemblage at the tile's composition) is the render's paired
/// half, read from the substrate at paint time, not stored here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerivedTile {
    /// The derived surface elevation (metres), from the tile's crust composition through the isostasy.
    pub elevation: Fixed,
    /// The relief class, from crossing the derived references (the field datum and the sea-level reference).
    pub relief: TerrainRelief,
}

/// Generate a field of DERIVED tiles from a per-tile crust composition (Slice 0, the visible spine's tile wire):
/// each tile's elevation derives from its composition ([`surface_elevation_from_composition`]), the relief datum is
/// the field mean ([`civsim_world::terrain::relief_datum`], a derived reference), and each tile's relief classifies
/// by crossing that datum and the `sea_level` reference (a clearly-labelled Slice-0 fixture, retiring when the water
/// budget derives). So the generated world's terrain is what the substrate says at each place, never fractal noise
/// or an authored band table (the R1 override, end to end). `None` when the field is empty or any tile's composition
/// reaches no elevation (fail-loud, never a fabricated tile). Byte-neutral: no scenario builds a world with this yet
/// (the render and scenario arming are the next slice).
#[allow(clippy::too_many_arguments)]
pub fn generate_derived_tiles(
    crust_compositions: &[Vec<(String, Fixed)>],
    mantle_density: Fixed,
    crustal_thickness: Fixed,
    temperature_k: Fixed,
    pressure_bar: Fixed,
    sea_level: Fixed,
    registry: &civsim_physics::petrology_data::PhaseRegistry,
    table: &civsim_physics::periodic::PeriodicTable,
) -> Option<Vec<DerivedTile>> {
    if crust_compositions.is_empty() {
        return None;
    }
    let mut elevations = Vec::with_capacity(crust_compositions.len());
    for composition in crust_compositions {
        elevations.push(surface_elevation_from_composition(
            composition,
            mantle_density,
            crustal_thickness,
            temperature_k,
            pressure_bar,
            registry,
            table,
        )?);
    }
    let datum = relief_datum(&elevations)?;
    Some(
        elevations
            .iter()
            .map(|&elevation| DerivedTile {
                elevation,
                relief: classify_relief(elevation, sea_level, datum),
            })
            .collect(),
    )
}

/// A labelled Slice-0 DEMONSTRATION crust field, the visible spine's scenario stand-in: a `cols` by `rows` grid
/// whose per-tile composition is one of three real rock-forming compositions (light silica SiO2, forsterite
/// Mg2SiO4, and denser periclase MgO), banded lightest-at-the-top. The per-tile composition is the ONLY authored
/// input, and it is a stand-in for what accretion and differentiation will derive in a later slice: this function
/// retires when that chain lands. Everything downstream is DERIVED end to end: each tile's density
/// ([`civsim_physics::petrology::crustal_density`], the stable assemblage the composition minimizes to, never an
/// authored per-rock density), its elevation (Airy isostasy), the field datum ([`relief_datum`], the field mean),
/// and its relief ([`classify_relief`], crossing the datum and the sea-level reference). So a lighter crust floats
/// higher and a denser one sits lower from its chemistry alone, and the viewer can show a frame whose terrain is
/// what the material IS. The isostasy fixtures (a synthetic mantle density, a representable column thickness, the
/// surface conditions, and the sea-level datum) are named below; they are Slice-0 fixtures retiring when the
/// accretion and water-budget chains derive them. `None` if the registry or table fails to load or a composition
/// reaches no assemblage (fail-loud, never a fabricated tile).
pub fn slice0_demo_field(cols: usize, rows: usize) -> Option<Vec<DerivedTile>> {
    let registry = civsim_physics::petrology_data::PhaseRegistry::standard().ok()?;
    let table = civsim_physics::periodic::PeriodicTable::standard().ok()?;
    // The three real compositions, lightest to densest. Each resolves through the registry to its stable
    // assemblage; the density falls out of the chemistry.
    let silica = vec![
        ("Si".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(2)),
    ]; // SiO2 (quartz)
    let forsterite = vec![
        ("Mg".to_string(), Fixed::from_int(2)),
        ("Si".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(4)),
    ]; // Mg2SiO4
    let periclase = vec![
        ("Mg".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(1)),
    ]; // MgO
    let palette = [silica, forsterite, periclase];
    // Band the palette across the rows, lightest at the top: a demonstration arrangement (the authored, labelled
    // stand-in), so the derived elevations vary down the field and the relief bands emerge from the derived
    // references, never a painted terrain.
    let rows = rows.max(1);
    let cols = cols.max(1);
    let mut field = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        let band = (r * palette.len()) / rows; // 0..palette.len()-1, top to bottom
        for _ in 0..cols {
            field.push(palette[band].clone());
        }
    }
    // Slice-0 isostasy fixtures (retire when the accretion and water-budget chains derive them):
    let mantle_density = Fixed::from_ratio(33, 10); // 3.3 g/cm^3, a synthetic mantle reference
    let crustal_thickness = Fixed::from_int(30); // 30 km, a representable column
    let temperature_k = Fixed::from_int(300); // surface conditions
    let pressure_bar = Fixed::from_int(1);
    let sea_level = Fixed::ZERO; // the ocean/land datum fixture
    generate_derived_tiles(
        &field,
        mantle_density,
        crustal_thickness,
        temperature_k,
        pressure_bar,
        sea_level,
        &registry,
        &table,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // A synthetic column: hot relative to a cold reference, with representable-scaled parameters (so the
    // Rayleigh intermediates fit Q32.32). The Rayleigh onset is switched by ra_crit in each test.
    /// THE ATOMICITY, ENFORCED. The ruling on `ColumnParams::thermal_diffusivity` says the cluster moves as
    /// a whole or not at all, and this asserts that the code makes a partial move IMPOSSIBLE rather than
    /// merely discouraged: with two of the seven properties still lacking a join, the derivation refuses,
    /// and the refusal names both the quantity and what composing it requires, so it reads as a work list.
    ///
    /// This test is written to FAIL when the frontier closes, which is deliberate. When the expansivity and
    /// viscosity joins land, this assertion breaks and whoever lands them replaces it with the assertion
    /// that the seven properties derive, having been told exactly where to do it. A test that quietly kept
    /// passing through that change would be a test that stopped meaning anything.
    #[test]
    fn the_column_derivation_refuses_a_partial_cluster_and_names_the_remaining_joins() {
        use civsim_physics::gruneisen::GruneisenTable;
        use civsim_physics::mineral_moduli::MineralModuli;
        use civsim_physics::periodic::PeriodicTable;
        use civsim_physics::petrology_data::PhaseRegistry;
        use civsim_physics::phase_conductivity::PhaseConductivityTable;

        let registry = PhaseRegistry::standard().expect("the phase registry loads");
        let periodic = PeriodicTable::standard().expect("the periodic table loads");
        let conductivity = PhaseConductivityTable::standard().expect("the cited column loads");
        let gruneisen = GruneisenTable::standard().expect("the Grueneisen table loads");
        let moduli = MineralModuli::standard().expect("the moduli table loads");
        // A magnesian olivine mantle, the composition the deep-time columns carry.
        let composition = vec![
            ("Mg".to_string(), Fixed::from_int(2)),
            ("Si".to_string(), Fixed::ONE),
            ("O".to_string(), Fixed::from_int(4)),
        ];

        let refusal = derive_column_thermal_properties(
            &composition,
            Fixed::from_int(1600),
            Fixed::from_int(100_000),
            &BankedTables {
                registry: &registry,
                periodic: &periodic,
                conductivity: &conductivity,
                gruneisen: &gruneisen,
                moduli: &moduli,
            },
        )
        .expect_err("the viscosity join is still open, so the cluster must not move");

        match &refusal {
            ColumnDerivationRefusal::NoJoinYet { quantity, needs } => {
                // THE FRONTIER MOVED, 2026-07-19, and that is what this assertion records. The
                // expansivity join landed, so the derivation now gets PAST it and refuses one step
                // later, at the viscosity. Reaching this arm is the proof that the expansivity
                // derives: a census that could not form an expansivity would have refused earlier.
                assert_eq!(
                    quantity, "ln_viscosity_pa_s",
                    "the expansivity join closed, so the frontier is now the viscosity"
                );
                assert!(
                    needs.contains("solve_ln_effective_viscosity")
                        && needs.contains("eval_pressure_gpa"),
                    "the refusal is a work list, naming the solve and the input the call site lacks: {needs}"
                );
            }
            other => panic!("expected the join frontier, got {other}"),
        }
        // The SECOND frontier refuses on its own terms, so closing the first cannot silently open the
        // cluster. Asserted separately because the derivation short-circuits at the first one it reaches.
        let viscosity_refusal =
            derive_column_ln_viscosity().expect_err("the viscosity join is not wired either");
        match &viscosity_refusal {
            ColumnDerivationRefusal::NoJoinYet { quantity, needs } => {
                assert_eq!(quantity, "ln_viscosity_pa_s");
                assert!(
                    needs.contains("solve_ln_effective_viscosity")
                        && needs.contains("eval_pressure_gpa"),
                    "it names the solve AND the input the call site does not form: {needs}"
                );
            }
            other => panic!("expected the viscosity frontier, got {other}"),
        }
        // And the message explains WHY it refused rather than substituted, since that is the ruling.
        assert!(
            refusal.to_string().contains("atomicity"),
            "the refusal cites the ruling it is enforcing: {refusal}"
        );
    }

    /// The diffusivity is COMPUTED from the three facts it is made of, never stored beside them, which is the
    /// redundant-parameter defect the ruling was written about. Asserted against the algebra rather than a
    /// recorded number: `kappa = k / (rho c_p)`.
    #[test]
    fn the_diffusivity_is_computed_from_its_three_facts_and_never_stored() {
        let p = ColumnThermalProperties {
            density_kg_m3: Fixed::from_int(3300),
            thermal_conductivity_w_m_k: Fixed::from_int(4),
            specific_heat_j_kg_k: Fixed::from_int(1200),
            thermal_expansion_ppm_per_k: Fixed::from_int(30),
            ln_viscosity_pa_s: Fixed::from_int(48),
        };
        let kappa = p.thermal_diffusivity().expect("the three facts compose");
        let expected = 4.0 / (3300.0 * 1200.0);
        assert!(
            (kappa.to_f64_lossy() - expected).abs() < 1e-9,
            "kappa = k/(rho c_p) = {expected:.3e}, read {:.3e}",
            kappa.to_f64_lossy()
        );
        // A silicate mantle sits near 1e-6 m^2/s, which is the magnitude check that would catch a unit slip.
        assert!(
            (1e-7..=1e-5).contains(&kappa.to_f64_lossy()),
            "a silicate diffusivity should land near 1e-6 m^2/s"
        );
    }

    /// The parcel radius derives from the cell geometry the column already carries, so it tracks the
    /// eigenvalue pair rather than sitting beside it. Asserted against the analytic `pi / a_c` rather than
    /// against a recorded number, because a recorded number here would be the fixture this replaces.
    #[test]
    fn the_parcel_radius_derives_from_the_columns_own_cell_scale() {
        let (_, mut p) = column(Fixed::from_int(1000));
        p.depth = Fixed::from_int(3);
        p.ra_crit_wavenumber = crate::deeptime::RIGID_RIGID_CRITICAL_WAVENUMBER;
        let r = p
            .parcel_radius()
            .expect("a positive wavenumber yields a radius");
        let expected = std::f64::consts::PI / p.ra_crit_wavenumber.to_f64_lossy() * 3.0;
        assert!(
            (r.to_f64_lossy() - expected).abs() < 1e-3,
            "r = d * pi / a_c, expected {expected}, read {}",
            r.to_f64_lossy()
        );
        // The rigid-rigid pair puts the half-wavelength near one layer depth, which is the regime check that
        // would catch a wavenumber swapped for the free-free value (sqrt(2), giving ~1.41 depths).
        assert!(
            (0.9..=1.2).contains(&(r.to_f64_lossy() / 3.0)),
            "the rigid-rigid cell is near one layer depth per parcel"
        );

        // A degenerate wavenumber has no cell scale, so it refuses rather than returning a radius.
        p.ra_crit_wavenumber = Fixed::ZERO;
        assert_eq!(p.parcel_radius(), None);
    }

    fn column(ra_crit: Fixed) -> (ColumnState, ColumnParams) {
        let state = ColumnState {
            temperature: Fixed::from_int(400),
            convecting: false,
        };
        let params = ColumnParams {
            reference_temperature: Fixed::from_int(300),
            density: Fixed::ONE,
            thermal_conductivity: Fixed::from_int(2),
            thermal_expansion_ppm: Fixed::from_int(30),
            gravity: Fixed::from_int(10),
            depth: Fixed::ONE,
            radius: Fixed::ONE,
            viscosity: Fixed::ONE,
            thermal_diffusivity: Fixed::from_ratio(1, 100),
            specific_heat: Fixed::from_int(10),
            heat_production: Fixed::from_int(100),
            ra_crit,
            // The rigid-rigid critical wavenumber, the pair mate of the classical rigid Ra_crit; the convection
            // step does not read it, so any value compiles, but the coherent rigid-rigid a_c ~ 3.117 is used.
            ra_crit_wavenumber: Fixed::from_ratio(3117, 1000),
            ra_max: Fixed::from_int(1_000_000),
            v_max: Fixed::from_int(1_000_000),
            flux_max: Fixed::from_int(1_000_000),
            stress_max: Fixed::from_int(1_000_000),
            dt: Fixed::ONE,
        };
        (state, params)
    }

    #[test]
    fn the_loss_coefficient_twins_the_kernels_own_conductive_loss() {
        // THE TWIN that licenses the thermostat to read one home. `conductive_loss_coefficient` claims to be
        // the coefficient `convection_step` composes internally, so it is checked against that composition
        // rather than against a restatement of its own formula (which would be circular). The depth is set
        // AWAY from one, because at `d = 1` the linear and the squared depth coincide and a dropped depth
        // factor would pass unnoticed: the test would prove nothing about the term it exists to pin.
        let (state, mut params) = column(Fixed::from_int(1_000_000_000));
        params.depth = Fixed::from_ratio(18, 10);
        let delta_t = state.temperature - params.reference_temperature;

        // The kernel's own composition, transcribed from `convection_step`: the Fourier flux over the
        // column's mass per area.
        let flux = laws::conduction(
            params.thermal_conductivity,
            Fixed::ONE,
            state.temperature,
            params.reference_temperature,
            params.depth,
            params.flux_max,
        );
        let mass_per_area = params.density.checked_mul(params.depth).unwrap();
        let kernel_loss = flux.checked_div(mass_per_area).unwrap();

        let coefficient = conductive_loss_coefficient(&params).expect("a positive column has one");
        let from_coefficient = coefficient.checked_mul(delta_t).unwrap();
        let gap = (kernel_loss - from_coefficient).abs();
        assert!(
            gap < Fixed::from_ratio(1, 1_000_000),
            "the coefficient must reproduce the kernel's conductive loss: kernel {kernel_loss:?} against \
             coefficient-composed {from_coefficient:?}"
        );

        // A column with no conductive path has no coefficient, refused rather than returned as zero (which a
        // consumer would divide by).
        let mut open = params;
        open.depth = Fixed::ZERO;
        assert!(
            conductive_loss_coefficient(&open).is_none(),
            "a zero-depth column has no conductive loss coefficient"
        );
    }

    #[test]
    fn a_subcritical_column_stays_conductive_and_relaxes() {
        // Ra_crit above any Rayleigh number the column reaches: it never convects and relaxes to the
        // conductive steady state (where radiogenic production balances the Fourier loss).
        let (state, params) = column(Fixed::from_int(1_000_000_000));
        let outcome = convection_solve(state, &params, 10_000, 1);
        assert!(
            outcome.converged,
            "the conductive column relaxes to a steady state"
        );
        assert!(
            !outcome.state.convecting,
            "a subcritical column never enters the convecting regime"
        );
        // It cooled from 400 toward the reference (production below the loss at 400).
        assert!(outcome.state.temperature < Fixed::from_int(400));
        assert!(outcome.state.temperature > Fixed::from_int(300));
    }

    #[test]
    fn a_supercritical_column_convects_and_relaxes_cooler() {
        // Ra_crit at zero: the column convects immediately, so the buoyant flow adds a convective loss and
        // the steady state is cooler than pure conduction.
        let (state, conv_params) = column(Fixed::ZERO);
        let convecting = convection_solve(state, &conv_params, 10_000, 1);
        assert!(
            convecting.state.convecting,
            "a supercritical column convects"
        );
        assert!(convecting.converged);

        let (state, cond_params) = column(Fixed::from_int(1_000_000_000));
        let conductive = convection_solve(state, &cond_params, 10_000, 1);

        assert!(
            convecting.state.temperature < conductive.state.temperature,
            "convection carries extra heat out, so the convecting steady state is cooler"
        );
    }

    #[test]
    fn the_convection_solve_is_deterministic() {
        let (state, params) = column(Fixed::ZERO);
        let a = convection_solve(state, &params, 5_000, 1);
        let b = convection_solve(state, &params, 5_000, 1);
        assert_eq!(
            a.state, b.state,
            "the same synthetic column reproduces the same outcome"
        );
        assert_eq!(a.iterations, b.iterations);
    }

    #[test]
    fn the_convection_onset_latch_is_one_way() {
        // A column already convecting stays convecting even after it has cooled below the onset contrast:
        // the latch never un-fires, so the recorded convecting regime is stable.
        let (_, params) = column(Fixed::from_int(1_000_000_000)); // a high Ra_crit it will not re-cross
        let already = ColumnState {
            temperature: Fixed::from_int(301), // barely above the reference, a tiny Rayleigh number
            convecting: true,
        };
        let next = convection_step(&already, &params);
        assert!(
            next.convecting,
            "the convection latch holds once set, even below the onset threshold"
        );
    }

    #[test]
    fn the_reservoir_decays_and_the_source_dies_over_the_thermal_history() {
        let (column0, params) = column(Fixed::ZERO);
        let decay = Fixed::from_ratio(1, 4); // 25% per tick, exactly representable
        let specific_heat_production = Fixed::from_int(4);
        let initial = SecularState {
            column: column0,
            reservoir: Fixed::from_int(100),
        };
        // One step: the reservoir spends down first-order, 100 -> 75.
        let s1 = secular_step(&initial, &params, decay, specific_heat_production);
        assert_eq!(
            s1.reservoir,
            Fixed::from_int(75),
            "the isotope reservoir spends down first-order"
        );
        // It keeps falling, monotone.
        let s2 = secular_step(&s1, &params, decay, specific_heat_production);
        assert!(
            s2.reservoir < s1.reservoir,
            "the reservoir decays monotonically"
        );
        // Over a long history the source is spent (the reservoir approaches zero).
        let late = secular_history(initial, &params, decay, specific_heat_production, 200);
        assert!(
            late.reservoir < Fixed::from_ratio(1, 100),
            "the heat source is spent after long geological time"
        );
    }

    #[test]
    fn a_decaying_source_leaves_the_interior_cooler_than_a_sustained_one() {
        // A decaying reservoir loses its heat source over time, so the interior cools below what a
        // sustained (non-decaying) source would hold: the spent-world relaxation.
        let (column0, params) = column(Fixed::ZERO);
        let specific_heat_production = Fixed::from_int(4);
        let initial = SecularState {
            column: column0,
            reservoir: Fixed::from_int(100),
        };
        let decaying = secular_history(
            initial,
            &params,
            Fixed::from_ratio(1, 4),
            specific_heat_production,
            300,
        );
        // Zero decay constant: the source never spends down, so it sustains a warmer interior.
        let sustained =
            secular_history(initial, &params, Fixed::ZERO, specific_heat_production, 300);
        assert!(
            decaying.column.temperature < sustained.column.temperature,
            "the interior with a decaying source ends cooler than one with a sustained source"
        );
    }

    #[test]
    fn the_secular_history_is_deterministic() {
        let (column0, params) = column(Fixed::ZERO);
        let initial = SecularState {
            column: column0,
            reservoir: Fixed::from_int(100),
        };
        let a = secular_history(
            initial,
            &params,
            Fixed::from_ratio(1, 4),
            Fixed::from_int(4),
            100,
        );
        let b = secular_history(
            initial,
            &params,
            Fixed::from_ratio(1, 4),
            Fixed::from_int(4),
            100,
        );
        assert_eq!(
            a, b,
            "the same synthetic thermal history reproduces the same outcome"
        );
    }

    // --- The interior column-wiring (#176) ---

    #[test]
    fn the_readout_exposes_the_continuous_state_and_a_hot_column_convects() {
        // A hot column (well above the reference) reaches a super-critical Rayleigh number, so the derived
        // convecting condition (Rayleigh against the critical value) is on, and the readout carries the
        // continuous quantities the contract stores.
        let (state, params) = column(Fixed::from_int(1)); // a low Ra_crit, so the hot column convects
        let readout = column_readout(&state, &params);
        assert!(
            readout.rayleigh > params.ra_crit,
            "the hot column is super-critical"
        );
        assert!(
            readout.convective_stress > Fixed::ZERO,
            "a convecting column exerts a driving stress"
        );
        // The stepped temperature is the convection_step result (the readout reuses it).
        assert_eq!(
            readout.temperature,
            convection_step(&state, &params).temperature
        );
    }

    #[test]
    fn convection_is_reversible_a_cold_column_does_not_convect() {
        // The gate's ruling: no stored convecting flag, so convection can CEASE on a cooling world. A column at
        // the reference temperature has no buoyancy, a sub-critical Rayleigh number, and no convective stress.
        let (_, params) = column(Fixed::from_int(1000));
        let cold = ColumnState {
            temperature: params.reference_temperature,
            convecting: false,
        };
        let readout = column_readout(&cold, &params);
        assert_eq!(
            readout.rayleigh,
            Fixed::ZERO,
            "no contrast, no convective vigor"
        );
        assert_eq!(
            readout.convective_stress,
            Fixed::ZERO,
            "a still interior drives no stress"
        );
    }

    #[test]
    fn populate_writes_the_interior_fields_and_the_snapshot_isostasy() {
        // The interior populates its continuous fields and the isostatic elevation, reading the surface lane's
        // crustal_density from the SNAPSHOT (snapshot-apply). The mantle density here stands in for the
        // petrology-derived value (derive_mantle_density over the mantle composition); the test supplies it
        // directly to isolate the wiring.
        let (_, params) = column(Fixed::from_int(1));
        let mantle_density = Fixed::from_ratio(33, 10); // a derived-density stand-in for the wiring test
        let snapshot = GeodynamicColumn {
            crustal_density: Fixed::from_ratio(265, 100), // written by the surface lane (felsic)
            crustal_thickness: Fixed::from_int(35_000),
            temperature: Fixed::from_int(400),
            ..GeodynamicColumn::default()
        };
        let next = populate_interior_column(snapshot, &params, mantle_density);
        // The surface field passes through unchanged (the interior does not write it).
        assert_eq!(next.crustal_density, snapshot.crustal_density);
        // The interior wrote its continuous state and the isostatic elevation from the snapshot crust.
        assert!(next.rayleigh > Fixed::ZERO);
        assert!(
            next.isostatic_elevation > Fixed::ZERO,
            "a felsic column floats above the reference"
        );
        let expected = civsim_physics::geodynamics::airy_isostatic_elevation(
            snapshot.crustal_density,
            mantle_density,
            snapshot.crustal_thickness,
        )
        .unwrap();
        assert_eq!(
            next.isostatic_elevation, expected,
            "the isostasy reads the snapshot crust and mantle"
        );
    }

    #[test]
    fn the_wiring_convection_is_reversible_a_cooled_column_ceases() {
        // The latch guardrail (gate ruling, #176): the resident contract stores no convecting flag, so a column
        // that once convected does NOT stay convecting against a fallen Rayleigh number. Populate a hot column
        // (it convects, stress positive), then feed its result back cooled to the reference, and the re-populated
        // column reads zero stress: the stress keys off the CURRENT Rayleigh number, reversibly, never a
        // persisted onset latch overriding it.
        let (_, params) = column(Fixed::from_int(1));
        let hot = GeodynamicColumn {
            temperature: Fixed::from_int(2000),
            ..GeodynamicColumn::default()
        };
        let convecting = populate_interior_column(hot, &params, Fixed::from_ratio(33, 10));
        assert!(
            convecting.convective_stress > Fixed::ZERO,
            "the hot column convects and drives a stress"
        );
        // Now the column has cooled to its reference (no contrast): re-populate against that snapshot.
        let cooled = GeodynamicColumn {
            temperature: params.reference_temperature,
            ..convecting
        };
        let ceased = populate_interior_column(cooled, &params, Fixed::from_ratio(33, 10));
        assert_eq!(
            ceased.convective_stress,
            Fixed::ZERO,
            "a cooled column ceases convecting, no latch keeps the stress alive"
        );
        assert_eq!(
            ceased.rayleigh,
            Fixed::ZERO,
            "the vigor falls with the contrast"
        );
    }

    #[test]
    fn the_boundary_layer_thins_with_vigor_so_a_hotter_column_drives_more_stress() {
        // The derived boundary layer L = depth * (Ra_crit / Ra)^(1/3): a more vigorous column has a thinner layer
        // and a higher driving stress, the derive-clean thinning (not the depth reference-pass).
        let (_, params) = column(Fixed::from_int(1));
        let warm = column_readout(
            &ColumnState {
                temperature: Fixed::from_int(600),
                convecting: false,
            },
            &params,
        );
        let hot = column_readout(
            &ColumnState {
                temperature: Fixed::from_int(2000),
                convecting: false,
            },
            &params,
        );
        assert!(
            hot.rayleigh > warm.rayleigh,
            "the hotter column is more vigorous"
        );
        assert!(
            hot.convective_stress > warm.convective_stress,
            "a thinner boundary layer under higher vigor concentrates more driving stress"
        );
    }

    #[test]
    fn an_empty_field_step_is_byte_neutral() {
        // Snapshot-apply over an unarmed geology walks no columns, so it yields an empty field (folds nothing
        // into state_hash), the dormant byte-neutral guarantee.
        let (_, params) = column(Fixed::from_int(1));
        let empty = GeodynamicField::new();
        let next = step_interior_field(&empty, &params, Fixed::from_ratio(33, 10));
        assert!(
            next.is_empty(),
            "an unarmed geology stays empty and byte-neutral"
        );
    }

    #[test]
    fn the_surface_elevation_derives_from_the_crust_composition() {
        // THE R1-OVERRIDE WIRE (the capstone's visible spine, foundation): a tile's elevation is what the material
        // at that place IS, never a noise field. The primitive composes crustal_density (composition -> density)
        // with the Airy isostasy law, so a crust lighter than the mantle floats above the reference. A resolvable
        // pure-silica crust (quartz, ~2.65 g/cm^3) on a synthetically denser mantle (3.3) floats positive, and the
        // primitive equals the two-step composition exactly (no hidden path). The full basaltic-Hadean composition
        // and the worldgen tile-axis wire are the next slice; this proves the derivation.
        let reg = civsim_physics::petrology_data::PhaseRegistry::standard()
            .expect("phase registry loads");
        let table =
            civsim_physics::periodic::PeriodicTable::standard().expect("periodic table loads");
        let crust = [
            ("Si".to_string(), Fixed::from_int(1)),
            ("O".to_string(), Fixed::from_int(2)),
        ];
        let mantle_density = Fixed::from_ratio(33, 10); // 3.3 g/cm^3, denser than the quartz crust
        let thickness = Fixed::from_int(30);
        let t = Fixed::from_int(300);
        let p = Fixed::from_int(1);
        let crust_density = civsim_physics::petrology::crustal_density(&crust, t, p, &reg, &table)
            .expect("the silica composition reaches a density");
        let expected = civsim_physics::geodynamics::airy_isostatic_elevation(
            crust_density,
            mantle_density,
            thickness,
        )
        .expect("the isostasy floats it");
        let got = surface_elevation_from_composition(
            &crust,
            mantle_density,
            thickness,
            t,
            p,
            &reg,
            &table,
        )
        .expect("the derived elevation wire");
        assert_eq!(
            got, expected,
            "the primitive composes crustal_density + airy exactly, no hidden path"
        );
        assert!(
            got > Fixed::ZERO,
            "a crust lighter than the mantle floats above the reference (elevation positive)"
        );
    }

    #[test]
    fn the_derived_tile_field_classifies_relief_from_the_derived_terrain() {
        // THE TILE WIRE (Slice 0, end to end): a two-tile field, a light quartz crust and a denser forsterite crust,
        // derives two elevations, the field-mean datum, and the relief by crossing it. The light crust floats higher
        // (Upland), the denser crust lower (Lowland), so the terrain is what the substrate says, not fractal noise.
        // Raising a sea-level fixture above the low crust makes it Submarine, the ocean/land boundary emerging by
        // crossing the derived-or-fixtured reference. All from composition, no authored terrain.
        let reg = civsim_physics::petrology_data::PhaseRegistry::standard().expect("registry");
        let table = civsim_physics::periodic::PeriodicTable::standard().expect("table");
        let quartz = vec![
            ("Si".to_string(), Fixed::from_int(1)),
            ("O".to_string(), Fixed::from_int(2)),
        ];
        let forsterite = vec![
            ("Mg".to_string(), Fixed::from_int(2)),
            ("Si".to_string(), Fixed::from_int(1)),
            ("O".to_string(), Fixed::from_int(4)),
        ];
        let field = vec![quartz, forsterite];
        let mantle = Fixed::from_ratio(33, 10);
        let thickness = Fixed::from_int(30);
        let t = Fixed::from_int(300);
        let p = Fixed::from_int(1);
        // Sea level at 0: both crusts above it, so relief is the light-vs-dense split about the field datum.
        let tiles =
            generate_derived_tiles(&field, mantle, thickness, t, p, Fixed::ZERO, &reg, &table)
                .expect("the derived tile field");
        assert_eq!(tiles.len(), 2);
        assert!(
            tiles[0].elevation > tiles[1].elevation,
            "the lighter quartz crust floats higher than the denser forsterite"
        );
        assert_eq!(
            tiles[0].relief,
            TerrainRelief::Upland,
            "the higher crust is Upland"
        );
        assert_eq!(
            tiles[1].relief,
            TerrainRelief::Lowland,
            "the lower crust is Lowland"
        );
        // Raise the sea-level fixture above the lower crust: it becomes Submarine (the ocean/land boundary crossing).
        let sea_above_low = tiles[1].elevation.checked_add(Fixed::ONE).unwrap();
        let flooded =
            generate_derived_tiles(&field, mantle, thickness, t, p, sea_above_low, &reg, &table)
                .expect("flooded field");
        assert_eq!(
            flooded[1].relief,
            TerrainRelief::Submarine,
            "raising sea level above the low crust submerges it"
        );
        // An empty field has no tiles (fail-loud, never a fabricated world).
        assert!(
            generate_derived_tiles(&[], mantle, thickness, t, p, Fixed::ZERO, &reg, &table)
                .is_none()
        );
    }

    #[test]
    fn the_slice0_demo_field_derives_all_three_relief_bands_from_real_compositions() {
        // THE VISIBLE SPINE'S SCENARIO: the banded demo field (light silica over forsterite over denser periclase)
        // derives three ordered elevations, so at the sea-level datum the frame shows all three relief classes: the
        // light silica floats to Upland, the forsterite sits Lowland, and the dense periclase sinks below the datum
        // to Submarine. The terrain is what the material is, never painted. All derived from the composition alone.
        let field = slice0_demo_field(4, 6).expect("the demo field derives");
        assert_eq!(field.len(), 24, "a 4 by 6 grid");
        // Deterministic: the same demo replays byte for byte.
        assert_eq!(
            field,
            slice0_demo_field(4, 6).expect("replays"),
            "a pure derivation replays"
        );
        // Elevation falls monotonically down the bands (lighter crust higher): the top row outfloats the bottom.
        assert!(
            field[0].elevation > field[field.len() - 1].elevation,
            "the light top band floats above the dense bottom band"
        );
        // All three relief classes are present: the derived terrain spans ocean, lowland, and upland.
        let has = |r: TerrainRelief| field.iter().any(|t| t.relief == r);
        assert!(has(TerrainRelief::Upland), "the light band is upland");
        assert!(has(TerrainRelief::Lowland), "the middle band is lowland");
        assert!(has(TerrainRelief::Submarine), "the dense band is submarine");
        // A degenerate request still yields a valid (non-empty) field.
        assert!(slice0_demo_field(1, 1).is_some());
    }

    #[test]
    fn the_convecting_mantle_depth_derives_from_the_planet_structure() {
        // An Earth-grade planet (radius 6371 km, ~32.5% core mass, mean density 5.51, metal-core density ~13)
        // derives a silicate mantle shell of the right grade (~2500 to 3200 km), R_planet - R_core, from the core
        // the differentiation set. Not an authored layer thickness: the mantle depth IS the planet's structure.
        let depth = convecting_mantle_depth_m(
            Fixed::from_int(6_371_000),
            Fixed::from_ratio(325, 1000),
            Fixed::from_ratio(551, 100),
            Fixed::from_int(13),
        )
        .expect("an Earth-grade planet has a derived mantle shell");
        let km = depth.to_f64_lossy() / 1000.0;
        assert!(
            (2500.0..=3200.0).contains(&km),
            "the derived mantle shell is Earth-grade, got {km} km"
        );
        // A larger core mass fraction sinks a bigger core and thins the mantle shell, monotonically.
        let bigger_core = convecting_mantle_depth_m(
            Fixed::from_int(6_371_000),
            Fixed::from_ratio(500, 1000),
            Fixed::from_ratio(551, 100),
            Fixed::from_int(13),
        )
        .expect("resolves");
        assert!(
            bigger_core < depth,
            "a larger core mass fraction thins the mantle shell"
        );
    }

    #[test]
    fn a_non_physical_planet_has_no_derived_mantle_depth() {
        // A core no denser than the mean could not have sunk to a core, so no distinct mantle shell (fail-loud).
        assert!(convecting_mantle_depth_m(
            Fixed::from_int(6_371_000),
            Fixed::from_ratio(325, 1000),
            Fixed::from_int(6),
            Fixed::from_int(5),
        )
        .is_none());
        // A non-positive radius and an out-of-range core fraction fail loud.
        assert!(convecting_mantle_depth_m(
            Fixed::ZERO,
            Fixed::from_ratio(3, 10),
            Fixed::from_int(5),
            Fixed::from_int(10)
        )
        .is_none());
        assert!(convecting_mantle_depth_m(
            Fixed::from_int(6_371_000),
            Fixed::from_int(2),
            Fixed::from_int(5),
            Fixed::from_int(10)
        )
        .is_none());
    }
}
