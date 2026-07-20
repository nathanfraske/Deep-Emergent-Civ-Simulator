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
    /// AND THE UNIT QUESTION, MEASURED AND THEN CORRECTED. Read the correction before the measurement,
    /// because the first version of this note overturned an approved plan on a mistake.
    ///
    /// THE MEASUREMENT, which stands. A mantle's radiogenic heating is `~5e-12 W/kg` against a Q32.32
    /// resolution of `2.33e-10`, so as a PER-SECOND RATE it is `0.02 ulp` and quantizes to ZERO. Carried that
    /// way, `H` and the conductive loss both vanish and `T + ((H - L)/c) dt` never moves the temperature. At
    /// the other end of the same kernel `eta`, `d^3`, `r^2`, `9 eta`, `eta kappa`, `eta v` and `rho d` all
    /// overflow. A RAW-LINEAR SI kernel therefore cannot work.
    ///
    /// THE CORRECTION, from an independent audit of this very note. That does NOT make the ruled plan
    /// infeasible, and the first version of this comment said it did. The plan of record was "a FULL SI/
    /// LOG-SPACE lift", and the log-space half is exactly what rescues it: carried as per-tick ENERGY
    /// (`H dt = 157.8 J/kg`, about `6.8e11` ulp) or as a signed logarithm, the same physics is comfortably
    /// representable and yields `dT ~ 0.127 K` per megayear at about `5.5e8` ulp. What was refuted was
    /// SI-seconds LINEAR, which the plan never proposed. Refuting a weaker form of a plan and reporting the
    /// plan dead is the error recorded here so it is not repeated.
    ///
    /// THE LOWER-RISK ROUTE, therefore, keeps SI semantics: hold viscosity, Rayleigh, Stokes and stress in
    /// logs (the machinery already exists in [`civsim_physics::laws::ln_rayleigh_number`] and
    /// [`civsim_physics::laws::ln_stokes_velocity`]), and carry heat as signed per-tick energy or a signed
    /// log until the final temperature increment. `Fixed::exp` bottoms out near `-22`, so `H` at `ln -26` is
    /// never exponentiated; only the integrated increment is. A declared Mm/Myr kernel also works but is a
    /// larger migration: every affected field and cap must convert consistently, and the `dt` wiring does not
    /// do that today. Full nondimensionalization is numerically the strongest option and the biggest rewrite.
    ///
    /// TWO FURTHER CORRECTIONS from the same audit. The conductive loss is NOT the same magnitude as `H`: the
    /// full-depth term is about `1.9e-13`, roughly 27 times smaller, though it still quantizes to zero. And
    /// the velocity band `1e-9` to `3e-9 m/s` is `3.2` to `9.5 cm/yr`, not `1` to `10`.
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
    /// THE UNIT HAZARD, named because the two depths in this file are NOT the same number. This reads
    /// [`Self::depth`], which is the representable-SCALED depth the linear kernel runs on (megametres for a
    /// mantle), while [`ColumnGeometry::layer_depth_m`] is SI metres. A caller that pairs
    /// `ColumnParams.depth = 1` with `ColumnGeometry.layer_depth_m = 1_800_000` gets no complaint from
    /// either type, and feeding THIS radius to the SI [`civsim_physics::laws::ln_stokes_velocity`] would
    /// introduce a `1e6` radius error and a `1e12` velocity error, because the Stokes form is quadratic in
    /// the radius. The returned value therefore carries the SAME scale as `depth` and must be consumed by
    /// the linear kernel only. An SI consumer needs the SI depth, not this.
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
    /// Dynamic viscosity as a BAND in log space (ln Pa*s), carrying its declared primary and the honest
    /// interval around it.
    ///
    /// A LOGARITHM because the value cannot be represented: an interior viscosity is `~1e21 Pa*s` against a
    /// `Fixed::MAX` of `~2.1e9`. A BAND rather than a scalar for a separate and stronger reason: the creep
    /// row's activation volume `V*` is banked as nine determinations spanning a range the source declines to
    /// collapse to a point, so reporting one number would author the very choice the primary refuses to make.
    /// The band's `ln_viscosity_primary` is the declared single-figure read (the low `V*` end, the weakest the
    /// row can be at positive pressure) and `min`/`max` are its uncertainty, never a discarded alternative.
    pub viscosity: civsim_physics::convective_viscosity::ViscosityBand,
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
    /// The creep ladder refused, carrying its own reason (an unfeedable row, a domain violation, or a solve
    /// that did not converge).
    Viscosity(String),
    /// The caller's selection pressure and the pressure the geometry implies disagree, so the bundle would
    /// describe two thermodynamic states at once.
    IncoherentState {
        /// The pressure the caller selected the assemblage at (GPa).
        requested_gpa: Fixed,
        /// The mid-layer lithostatic pressure the geometry implies (GPa).
        implied_gpa: Fixed,
    },
    /// The assemblage came back TRUNCATED: the subset enumeration hit its candidate cap, so the phases are
    /// the best of a searched set rather than of the full one, and any property read off them is provisional.
    TruncatedAssemblage,
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
            ColumnDerivationRefusal::Viscosity(reason) => {
                write!(f, "the creep ladder refused: {reason}")
            }
            ColumnDerivationRefusal::IncoherentState {
                requested_gpa,
                implied_gpa,
            } => write!(
                f,
                "the assemblage was selected at {} GPa while the geometry implies a mid-layer {} GPa, so \
                 the bundle would describe two thermodynamic states at once; refused",
                requested_gpa.to_f64_lossy(),
                implied_gpa.to_f64_lossy()
            ),
            ColumnDerivationRefusal::TruncatedAssemblage => write!(
                f,
                "the stable assemblage hit its candidate cap and is provisional rather than minimized, so \
                 properties derived from it would be confident and possibly wrong; refused"
            ),
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

/// The column's DYNAMICAL state, the inputs the viscosity needs and the material properties do not.
///
/// The split is real rather than cosmetic. Density, conductivity, specific heat and expansivity are MATERIAL
/// properties: give a composition, a temperature and a pressure and they follow. Viscosity is a COLUMN
/// property: the creep ladder's effective value depends on the strain rate the convection itself sets, so it
/// reads the layer depth, the gravity and the buoyancy contrast driving the flow. Bundling them would have
/// hidden that the seventh cluster field is a different KIND of quantity from the other six.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnGeometry {
    /// The convecting-layer depth (m).
    pub layer_depth_m: Fixed,
    /// Surface gravity (m/s^2).
    pub gravity_m_s2: Fixed,
    /// The temperature contrast driving buoyancy (K), interior against the cold reference.
    pub temperature_contrast_k: Fixed,
    /// `ln Ra_crit`, the onset regime's critical Rayleigh number in logs, the SAME one the boundary layer and
    /// the convection onset read.
    pub ln_rayleigh_critical: Fixed,
}

impl ColumnGeometry {
    /// Build a geometry whose SI depth is DERIVED from a column's own scaled depth, so the two cannot
    /// disagree.
    ///
    /// WHY THIS EXISTS RATHER THAN A WARNING. There are two depths in this file and they are not the same
    /// number: [`ColumnParams::depth`] is the representable-SCALED depth the linear kernel runs on
    /// (megametres for a mantle) and [`Self::layer_depth_m`] is SI metres. Supplied independently, a caller
    /// can pair `depth = 1` with `layer_depth_m = 1_800_000` and neither type complains, while the parcel
    /// radius comes out in the former unit and the Stokes velocity consumes the latter. Because Stokes is
    /// QUADRATIC in the radius, that mispairing is a `1e6` radius error and a `1e12` velocity error, and it
    /// would look like physics rather than like a bug.
    ///
    /// Prose could say all that, and today has been a lesson in prose warnings being dropped by exactly the
    /// consumer they were written for. So the SI depth is derived here instead of accepted, and the wire is
    /// expected to build its geometry this way rather than by filling the fields.
    ///
    /// `None` if the scaled depth is non-positive or the conversion leaves the representable window.
    // @derives: a column's SI layer depth <- its own representable-scaled depth (megametres to metres)
    pub fn from_scaled_column(
        params: &ColumnParams,
        temperature_contrast_k: Fixed,
        ln_rayleigh_critical: Fixed,
    ) -> Option<Self> {
        Self::from_scaled_depth(
            params.depth,
            params.gravity,
            temperature_contrast_k,
            ln_rayleigh_critical,
        )
    }

    /// The same invariant from the SCALED depth alone, for callers that do not yet have a
    /// [`ColumnParams`] to read it off.
    ///
    /// This exists because retiring the fixture cluster created a cycle: `province_column_params` now
    /// needs the DERIVED thermal properties, deriving them needs a geometry, and building the geometry
    /// through [`Self::from_scaled_column`] needed the very `ColumnParams` being built. The geometry only
    /// ever reads `depth` and `gravity`, neither of which comes from the derivation, so the cycle is
    /// broken by taking those two directly.
    ///
    /// The defence is unchanged and that is the point of delegating rather than duplicating: there is
    /// still exactly ONE place the megametre-to-metre conversion happens, so the two depths still cannot
    /// drift apart. A second hand-written `* 1_000_000` at a call site is exactly the mispairing this
    /// constructor was built to make unconstructible, and it would be quadratic in the Stokes radius.
    pub fn from_scaled_depth(
        depth_scaled_mm: Fixed,
        gravity_m_s2: Fixed,
        temperature_contrast_k: Fixed,
        ln_rayleigh_critical: Fixed,
    ) -> Option<Self> {
        if depth_scaled_mm <= Fixed::ZERO {
            return None;
        }
        let layer_depth_m = depth_scaled_mm.checked_mul(Fixed::from_int(1_000_000))?;
        Some(Self {
            layer_depth_m,
            gravity_m_s2,
            temperature_contrast_k,
            ln_rayleigh_critical,
        })
    }
}

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
    /// The Mie-Grueneisen-Debye anchor column (`theta_0`, `q`, and the `V_0`/`K_0`/`K_0'` from the same
    /// fit). What lets the expansivity be asked for AT the column's own state rather than at 300 K.
    pub anchors: &'a civsim_physics::thermoelastic_anchors::ThermoelasticAnchors,
}

/// Derive every thermal property of an interior column from the world's OWN composition, ATOMICALLY.
///
/// The chain, each link a banked derivation rather than a value: the composition minimizes to a stable
/// assemblage; one pass of the thermoelastic ladder over its volume census gives the expansivity and the
/// compression AT the requested state; that compression carries the registry's ambient molar volumes to the
/// state, giving the density; the mean atomic mass gives the Dulong-Petit specific heat; the same census
/// runs the two-rung conductivity ladder; and the diffusivity falls out of those three rather than being
/// stored beside them.
///
/// ALL SEVEN NOW DERIVE AT DEPTH, which is what the thermoelastic ladder was built for. They refused until
/// 2026-07-19, and the refusal was correct rather than a placeholder: every input row was an ambient one,
/// and reading those at 1600 K and 100 kbar had produced a number that matched measurement by CANCELLATION.
/// Rung 3 of the ladder solves the equation of state at the requested state instead, so the bundle is
/// computed there rather than borrowed from 300 K.
///
/// THE RESULT IS STILL A `Result`, because refusing remains the correct answer in three cases the wire does
/// not remove: an assemblage that hit the subset-enumeration cap and is provisional rather than minimized; a
/// census the ladder cannot solve whole, since renormalising over the covered phases would substitute their
/// behaviour for the rock's; and a requested pressure that disagrees with the one the column's own density
/// implies, which is the same state-coherence the seven fields promise, checked rather than assumed.
///
/// THE ONE-STATE PROMISE IS STRUCTURAL, not a comment. The expansivity and the density come from ONE ladder
/// pass returning both, because the first version of this wire took the expansivity from the ladder and left
/// the density on the registry's ambient volumes: two volumes for one rock, four percent apart, with nothing
/// comparing them.
// @derives: an interior column's thermal properties <- the world's own composition through the banked assemblage, ladder and Dulong-Petit derivations
pub fn derive_column_thermal_properties(
    composition: &[(String, Fixed)],
    temperature_k: Fixed,
    pressure_bar: Fixed,
    geometry: &ColumnGeometry,
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
    // A TRUNCATED assemblage is provisional, not minimized: `stable_assemblage` sets this flag when its
    // subset enumeration hits the candidate cap, so the returned phases are the best of a SEARCHED set
    // rather than of the full one. Properties derived from it would be confident and possibly wrong, which
    // is the outcome this whole derivation exists to avoid. It refuses instead. The cap is reachable: with
    // fifteen candidate phases over five present elements the subsets of size one through five number
    // 4,943, past the 4,096 limit, so growing the registry alone can trigger it.
    if assemblage.truncated {
        return Err(ColumnDerivationRefusal::TruncatedAssemblage);
    }

    // The volume census, needed before the density now because the density is state-resolved through it.
    // Volume fractions rather than molar amounts, because both the Bruggeman conductivity mixture and the
    // compression average weight by the volume each phase occupies.
    let volume_census =
        civsim_physics::petrology::assemblage_volume_fractions(&assemblage, tables.registry)
            .ok_or_else(|| missing("volume_fractions"))?;

    // ONE LADDER PASS supplies BOTH the expansivity and the compression, so the two cannot describe
    // different volumes for the same rock. `None` means the ladder could not solve the whole census here,
    // and every consumer below falls back to its own ambient path, which refuses out of frame on its own
    // terms rather than answering.
    let state = civsim_materials::thermoelastic::ThermoState {
        temperature_k,
        pressure_bar,
    };
    let pass = ladder_pass_over_census(&volume_census, state, tables);

    // Density: the registry's own molar volumes and masses, in g/cm^3, lifted to kg/m^3, and then CARRIED
    // TO THE REQUESTED STATE by the same solve the expansivity came from.
    //
    // The ambient value alone is wrong at depth and wrong in a way that hides: at 1600 K and 10.7 GPa
    // forsterite's molar volume is 41.94 cm^3/mol against its 43.60 reference, so an ambient density
    // understates the column by about 4 percent and, worse, disagrees with the volume its own expansivity
    // was solved at. `rho(P,T) = rho_0 / sum(phi_i f_i)` follows directly from `rho = M/V` with each phase
    // compressed by `f_i`, using the same ambient volume fractions the census carries.
    let density_g_cm3_ambient = civsim_physics::petrology::assemblage_density(
        &assemblage,
        tables.registry,
        tables.periodic,
    )
    .ok_or_else(|| missing("density"))?;
    let density_g_cm3 = match &pass {
        Some(p) if p.compression > Fixed::ZERO => density_g_cm3_ambient
            .checked_div(p.compression)
            .ok_or_else(|| missing("density_at_state"))?,
        _ => density_g_cm3_ambient,
    };
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

    // Conductivity: the volume census through the two-rung ladder.
    // THE EXPANSIVITY IS DERIVED BEFORE THE CONDUCTIVITY because the conductivity ladder CONSUMES it.
    // Hofmeister's lattice form carries a factor `exp[-(4 gamma + 1/3) * integral(alpha dT)]` from the
    // 298 K anchor to the evaluation temperature, and passing ZERO for that integral silently asserts a
    // phase that does not expand between 298 K and 1600 K. It is not a harmless placeholder: for forsterite
    // the omitted correction is about `exp(-0.287) = 0.751`, so the lattice conductivity came out roughly
    // 33 percent HIGH, and the assemblage about 23 percent high. An audit caught it; the first version of
    // this code passed `Fixed::ZERO` here and called the field "the caller's own", which was true and not
    // an excuse for supplying a wrong value.
    let thermal_expansion_ppm_per_k = derive_assemblage_expansivity_ppm_per_k(
        &volume_census,
        temperature_k,
        pressure_bar,
        tables,
        pass.as_ref(),
    )?;
    // The integral of a CONSTANT expansivity over the anchor-to-evaluation range. Constant-alpha is the
    // honest limit here and is stated rather than hidden: the banked gamma, bulk modulus and molar volume
    // are ambient-frame values, so the expansivity they produce carries no temperature dependence of its
    // own, and integrating it as a constant claims exactly that and no more.
    let anchor_k = civsim_materials::conductivity::hofmeister_reference_temperature_k();
    let expansivity_integral = if temperature_k > anchor_k {
        thermal_expansion_ppm_per_k
            .checked_div(Fixed::from_int(1_000_000))
            .and_then(|per_k| per_k.checked_mul(temperature_k - anchor_k))
            .ok_or_else(|| missing("expansivity_integral"))?
    } else {
        Fixed::ZERO
    };

    // THE CONDUCTIVITY MIXES THE STATE CENSUS, NOT THE AMBIENT ONE. Bruggeman is a VOLUME-fraction mix, so
    // it has to read the phases in the proportions they occupy at this state. The phases do not compress
    // equally, so the ambient census names a different mixture, and averaging over it describes a rock that
    // is not the one at depth. The expansivity above now weights the same way, so the two agree by
    // construction rather than by coincidence. Where the ladder could not answer, the ambient census is the
    // honest fallback: it is what the registry measured, and no state-resolved alternative exists.
    let mixing_census: &[(String, Fixed)] = match pass.as_ref() {
        Some(p) => &p.state_census,
        None => &volume_census,
    };
    let mut rows = Vec::with_capacity(mixing_census.len());
    for (name, fraction) in mixing_census {
        let row = civsim_materials::conductivity::phase_conductivity_from_banked(
            name,
            tables.conductivity,
            tables.gruneisen,
            Some(tables.moduli),
            tables.registry,
            tables.periodic,
            expansivity_integral,
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
    // The viscosity closes LAST because it consumes the other four: the diffusivity it needs is
    // `k / (rho c_p)` and the buoyancy contrast it needs is `rho alpha dT`. That ordering is the reason the
    // ruling called this cluster atomic rather than seven independent values.
    // THE PRESSURE COHERENCE GATE. Two pressures enter this derivation and nothing compared them: the
    // caller's `pressure_bar`, which selects the stable assemblage, and the mid-layer lithostatic pressure
    // the geometry implies, which the creep ladder is evaluated at. A caller could select phases at 1 bar
    // and evaluate viscosity near 46 GPa and get a bundle. That is the same defect the frame gate closes
    // one level up, in its within-bundle form: the seven fields must describe ONE thermodynamic state, and
    // atomicity does not license moving them together when they do not.
    //
    // THE GATE WAS DEAD FOR EVERY ROCKY MANTLE, and the ordering is why. Formed left to right as
    // `rho g d`, the intermediate is about `2.2e10` for a Mars-class column against Q32.32's `2.1e9`
    // ceiling, so `checked_mul` returned `None`, the `if let Some(..)` found nothing, and the gate
    // SKIPPED rather than refusing. Measured, the surviving band is a density below 322 kg/m^3 at the Mars
    // geometry and below 76 at Earth whole-mantle scale: no silicate mantle is within an order of magnitude
    // of either, so the comparison never ran on any column this engine builds. A gate that fails OPEN is
    // worse than no gate, because the passing test reads as evidence.
    //
    // Dividing FIRST keeps every intermediate inside the window, which is the same reassociation
    // `derive_column_viscosity` already carries fifty lines below and the viewer carries at its own call
    // site. An unrepresentable intermediate now REFUSES rather than vanishing.
    let implied_gpa = implied_lithostatic_gpa(density_kg_m3, geometry)
        .ok_or_else(|| missing("implied_lithostatic_pressure"))?;
    let requested_gpa = pressure_bar
        .checked_div(Fixed::from_int(10_000))
        .ok_or_else(|| missing("requested_pressure"))?;
    let off = if implied_gpa > requested_gpa {
        implied_gpa - requested_gpa
    } else {
        requested_gpa - implied_gpa
    };
    // A tenth of the implied pressure, so a rounding-scale difference passes and a
    // wrong-state pairing does not.
    let slack = implied_gpa
        .checked_div(Fixed::from_int(10))
        .unwrap_or(Fixed::ZERO);
    if off > slack {
        return Err(ColumnDerivationRefusal::IncoherentState {
            requested_gpa,
            implied_gpa,
        });
    }

    let viscosity = derive_column_viscosity(
        density_kg_m3,
        thermal_conductivity_w_m_k,
        specific_heat_j_kg_k,
        thermal_expansion_ppm_per_k,
        temperature_k,
        geometry,
    )?;

    Ok(ColumnThermalProperties {
        density_kg_m3,
        thermal_conductivity_w_m_k,
        specific_heat_j_kg_k,
        thermal_expansion_ppm_per_k,
        viscosity,
    })
}

/// How many times the depth-pressure fixed point may iterate before it is called a failure to converge.
///
/// A FIXED trip count rather than a convergence-rate-dependent one, so the solve is a pure function of its
/// inputs. Determinism does not require a constant number of steps, only one that does not depend on how
/// fast the iterate happens to settle, and a hard cap supplies that while making non-convergence a named
/// refusal instead of a hang.
const DEPTH_PRESSURE_ITERATIONS: u32 = 12;

/// Derive a column FROM ITS DEPTH, solving the pressure self-consistently rather than being told it.
///
/// # WHY THIS EXISTS, and why the caller-supplied pressure was never coherent
///
/// [`derive_column_thermal_properties`] takes a pressure AND a geometry. The pressure selects the stable
/// assemblage; the geometry, once the assemblage yields a density, implies a lithostatic pressure of its
/// own. Nothing made the two agree, and the gate that was supposed to compare them overflowed and skipped,
/// so a caller could select phases at one pressure and evaluate the column at a depth implying another. The
/// flagship fixture did exactly that: it asked for 10 GPa at a geometry whose own mass implies 11.17, an
/// 11.7 percent disagreement that the dead gate never reported.
///
/// The disagreement is a real fixed point rather than a bad input. Density depends on the assemblage, the
/// assemblage depends on pressure, and pressure depends on density. Choosing either number by hand authors
/// one side of a loop the physics closes on its own. So the pressure stops being an input here: the caller
/// supplies the depth and gravity it actually has, and the column solves for the pressure its own mass
/// produces.
///
/// # THE GATE IS THE RESIDUAL
///
/// The iteration needs no new machinery, because [`ColumnDerivationRefusal::IncoherentState`] already
/// carries BOTH pressures. A refusal at trial `p` reports the pressure that trial's density implies, which
/// is exactly the next iterate. So the coherence gate doubles as the solver's residual, and a converged
/// answer is by construction one the gate accepts rather than one that bypassed it.
///
/// The map contracts because density responds only weakly to pressure over a rung: a 10 percent pressure
/// error moves the density by well under a percent, which moves the implied pressure by the same fraction
/// again. Starting from the ambient frame it settles in a few steps.
///
/// # THE GATE'S SLACK IS NOT THE STOPPING CRITERION, which a first version of this got wrong
///
/// Iterating only until the coherence gate ACCEPTS settles at the first iterate inside its 10 percent
/// slack rather than at the fixed point. Measured, that returned 10.286 GPa where the fixed point is
/// 11.17: an answer determined by the starting guess and the tolerance rather than by the physics, which
/// is the same defect as a solve whose convergence test bounds the step and not the residual. The loop
/// below runs the map to its own fixed point and consults the gate only at the end, so the slack judges
/// the answer instead of choosing it.
///
/// Returns the converged pressure in bar alongside the properties, so a caller can record the state the
/// bundle actually describes rather than the one it asked for.
// @derives: a column's self-consistent pressure and thermal properties <- its composition, temperature, depth and gravity
pub fn solve_column_at_depth(
    composition: &[(String, Fixed)],
    temperature_k: Fixed,
    geometry: &ColumnGeometry,
    tables: &BankedTables<'_>,
) -> Result<(Fixed, ColumnThermalProperties), ColumnDerivationRefusal> {
    let missing = |q: &str| ColumnDerivationRefusal::NoQuantity {
        quantity: q.to_string(),
    };
    // The ambient frame is the honest starting guess: it is the one state whose properties are measured
    // rather than extrapolated, and the first step replaces it with a physically implied pressure.
    let mut pressure_bar = Fixed::ONE;
    for _ in 0..DEPTH_PRESSURE_ITERATIONS {
        // The implied pressure at this trial, however the gate happens to judge the pair. A refusal
        // already carries it; an acceptance means the density is in hand and the same helper forms it, so
        // the two paths cannot drift on the formula.
        let implied_gpa = match derive_column_thermal_properties(
            composition,
            temperature_k,
            pressure_bar,
            geometry,
            tables,
        ) {
            Ok(properties) => implied_lithostatic_gpa(properties.density_kg_m3, geometry)
                .ok_or_else(|| missing("implied_lithostatic_pressure"))?,
            Err(ColumnDerivationRefusal::IncoherentState { implied_gpa, .. }) => implied_gpa,
            Err(other) => return Err(other),
        };
        let next_bar = implied_gpa
            .checked_mul(Fixed::from_int(10_000))
            .ok_or_else(|| missing("implied_pressure_bar"))?;
        if next_bar == pressure_bar {
            // A bit-exact fixed point. Nothing further to extract, and the gate will accept by
            // construction because the residual is zero.
            break;
        }
        pressure_bar = next_bar;
    }
    // The final evaluation is GATED like any other, so a converged answer is one the coherence check
    // passed rather than one that bypassed it. If the map failed to settle, this is where it says so.
    let properties = derive_column_thermal_properties(
        composition,
        temperature_k,
        pressure_bar,
        geometry,
        tables,
    )?;
    Ok((pressure_bar, properties))
}

/// The mid-layer lithostatic pressure a column's own mass implies, in GPa: `rho g (d/2)`.
///
/// ONE FORMULA, TWO CONSUMERS: the coherence gate that judges a caller's pressure and the fixed point that
/// solves for it. Written separately they would be free to drift, and a solver converging on a residual
/// the gate does not measure is a solver that satisfies nothing.
///
/// The division comes FIRST and that is load-bearing. Formed as `rho g d` the intermediate is about
/// `2.2e10` for a Mars-class column against Q32.32's `2.1e9` ceiling, which is what made the original gate
/// return `None` and skip on every rocky mantle.
// @derives: the mid-layer lithostatic pressure <- the column's density, gravity and layer depth
fn implied_lithostatic_gpa(density_kg_m3: Fixed, geometry: &ColumnGeometry) -> Option<Fixed> {
    geometry
        .layer_depth_m
        .checked_div(Fixed::from_int(2_000_000_000))
        .and_then(|x| x.checked_mul(density_kg_m3))
        .and_then(|x| x.checked_mul(geometry.gravity_m_s2))
}

/// The assemblage's volumetric expansivity in PPM per kelvin, the unit the kernel's
/// [`civsim_physics::laws::thermal_density_anomaly`] reads.
///
/// NOT CLOSED, and the earlier "FRONTIER CLOSED" note here was wrong. The join exists as
/// [`civsim_materials::properties::ambient_assemblage_volumetric_expansivity_per_k`], which derives each phase's
/// `alpha = gamma C_v / (K V_m)` from four banked columns and mixes them by volume. Its magnitude is checked
/// against the MEASURED forsterite expansivity (roughly 40 ppm/K at mantle temperature) reproduced from
/// columns never fitted to it, which is what makes that a check rather than a circular one. Worth recording
/// as the reason to derive it: the fixture being replaced reads 30 ppm/K, and forsterite derives near 40.
///
/// The per-kelvin result is scaled to ppm here, at the boundary where the kernel's unit is declared, rather
/// than inside the derivation, so the physics function returns the physical quantity and this conversion is
/// visible at the site that needs it.
/// What ONE pass of the thermoelastic ladder over the census yields: the state-resolved expansivity and
/// the compression the same solve implies.
///
/// The two travel together BY CONSTRUCTION, and that is the point of the struct rather than two functions.
/// A first version of this wire took the expansivity from the ladder at the column's own state and left the
/// density reading the registry's ambient molar volumes, so a mantle column reported an expansivity solved
/// at 41.94 cm^3/mol beside a density computed from 43.60. Two volumes for one rock, nothing comparing
/// them, in a bundle whose entire ruling is that the seven fields describe ONE thermodynamic state. It is
/// the diamond pattern in miniature, and returning both from one solve makes it unconstructible.
struct LadderPass {
    /// Volume-weighted volumetric expansivity at the requested state (per K), weighted by the STATE
    /// fractions rather than the ambient ones.
    alpha_per_k: Fixed,
    /// `sum(phi_i * V_i(P,T) / V_0i)`, the census-weighted compression factor. Multiply an ambient molar
    /// volume by this to get the state-resolved one; divide an ambient density by it for the same state.
    compression: Fixed,
    /// The census RE-WEIGHTED to the requested state and renormalised to one:
    /// `phi_i(P,T) = phi_i0 f_i / sum_j(phi_j0 f_j)` with `f_i = V_i(P,T)/V_i0`.
    ///
    /// Every volume-weighted consumer must read THIS rather than the ambient census it was called with.
    /// The phases do not compress equally, so the mixture a caller sees at depth is a different mixture by
    /// volume than the one the registry lists at ambient, and a property averaged over the ambient
    /// fractions describes a rock that is not the one at this state.
    state_census: Vec<(String, Fixed)>,
}

/// Run the ladder over the whole census at one state, or report that it could not.
///
/// PARTIAL COVERAGE IS NOT AN ANSWER. If any phase cannot be solved here, renormalising over the rest
/// would silently substitute the covered phases' behaviour for the whole rock, which is an authored
/// assemblage wearing a derivation's clothes. The whole census answers or none of it does.
fn ladder_pass_over_census(
    volume_census: &[(String, Fixed)],
    state: civsim_materials::thermoelastic::ThermoState,
    tables: &BankedTables<'_>,
) -> Option<LadderPass> {
    // THE WEIGHT IS THE STATE FRACTION, NOT THE AMBIENT ONE, and the two are different mixtures.
    //
    // The census arrives as ambient volume fractions `phi_i0`. At the requested state each phase has its
    // own volume ratio `f_i = V_i(P,T)/V_i0`, so the state fractions are `phi_i0 f_i / sum_j(phi_j0 f_j)`.
    // An aggregate expansivity is a VOLUME average, so it must be `sum(phi_i0 f_i alpha_i) /
    // sum(phi_i0 f_i)`. This used to compute `sum(phi_i0 alpha_i) / sum(phi_i0)`, mixing the phases in
    // their ambient proportions while the compression beside it used `f_i` correctly. So the bundle whose
    // whole ruling is that its fields describe ONE state contained two different mixtures, which is the
    // same diamond the `LadderPass` struct was introduced to make unconstructible, one level down.
    //
    // A single-phase census cannot see this: with one phase every weighting collapses to the same number,
    // and the flagship fixture is single-phase forsterite. The derived mantle minimizes to a multi-phase
    // spinel-bearing assemblage, so the `f_i` do differ in production.
    let mut alpha_weighted = Fixed::ZERO;
    let mut state_total = Fixed::ZERO;
    let mut covered = Fixed::ZERO;
    let mut state_weights: Vec<(String, Fixed)> = Vec::with_capacity(volume_census.len());
    for (name, fraction) in volume_census {
        let r = civsim_materials::thermoelastic::response_at(
            name,
            state,
            tables.registry,
            tables.moduli,
            tables.gruneisen,
            tables.anchors,
        )
        .ok()?;
        // The ratio against the phase's OWN reference volume, taken from the registry the census was
        // built from, so the compression is measured against the same basis the density uses.
        let v0 = tables.registry.phase(name)?.molar_volume;
        if v0 <= Fixed::ZERO {
            return None;
        }
        let f = r.molar_volume_cm3.checked_div(v0)?;
        // The phase's share of the STATE volume, before renormalising: `phi_i0 f_i`.
        let weight = f.checked_mul(*fraction)?;
        alpha_weighted = alpha_weighted.checked_add(r.alpha_per_k.checked_mul(weight)?)?;
        state_total = state_total.checked_add(weight)?;
        covered = covered.checked_add(*fraction)?;
        state_weights.push((name.clone(), weight));
    }
    if covered <= Fixed::ZERO || state_total <= Fixed::ZERO {
        return None;
    }
    // Renormalise the state weights to one, so a consumer can use them as fractions directly.
    let mut state_census = Vec::with_capacity(state_weights.len());
    for (name, weight) in state_weights {
        state_census.push((name, weight.checked_div(state_total)?));
    }
    Some(LadderPass {
        // Divided by the STATE total, so numerator and denominator carry the same weighting.
        alpha_per_k: alpha_weighted.checked_div(state_total)?,
        // The compression is the ratio of state volume to ambient volume, so this denominator is the
        // AMBIENT total and stays as it was. The two divisors differ on purpose.
        compression: state_total.checked_div(covered)?,
        state_census,
    })
}

/// The assemblage's volumetric expansivity in ppm/K at the requested state.
///
/// THE LADDER IS ASKED FIRST, AT THE COLUMN'S OWN STATE. This is the seam the thermoelastic ladder was
/// built to close. The ambient join below is correct only inside its rows' 300 K frame, and an interior
/// column is nowhere near it: reading it at 1600 K and 100 kbar returned a number that matched measurement
/// by CANCELLATION, which is the defect on record. It refuses outside its frame now, so before this wire
/// an interior column got no expansivity at all.
fn derive_assemblage_expansivity_ppm_per_k(
    volume_census: &[(String, Fixed)],
    requested_temperature_k: Fixed,
    requested_pressure_bar: Fixed,
    tables: &BankedTables<'_>,
    pass: Option<&LadderPass>,
) -> Result<Fixed, ColumnDerivationRefusal> {
    let per_k = match pass {
        Some(p) => p.alpha_per_k,
        // THE AMBIENT JOIN IS THE FALLBACK, and it refuses on its own terms when the state is outside its
        // frame. Reaching it is therefore not a silent downgrade: either the query is genuinely ambient
        // and this is the right rung, or it refuses and the column refuses with it.
        None => civsim_materials::properties::ambient_assemblage_volumetric_expansivity_per_k(
            volume_census,
            requested_temperature_k,
            requested_pressure_bar,
            tables.registry,
            tables.moduli,
            tables.gruneisen,
        )
        .map_err(|e| ColumnDerivationRefusal::Expansivity(e.to_string()))?,
    };
    per_k
        .checked_mul(Fixed::from_int(1_000_000))
        .ok_or_else(|| ColumnDerivationRefusal::NoQuantity {
            quantity: "thermal_expansion_ppm_per_k".to_string(),
        })
}

/// The column's effective viscosity as a log-space BAND.
///
/// THE ADMITTED CREEP SET IS ONE ROW, and that is a capability statement rather than a simplification.
/// Hirth and Kohlstedt 2003 Table 1 banks five rows; three are WET and refuse because no water fugacity or
/// content is derived anywhere in this engine, and the grain-boundary-sliding row refuses because no grain
/// size is. Dry dislocation creep is what remains, and it is admitted because its grain-size exponent is zero
/// and it needs no water. Those refusals are the rows' own, carried rather than worked around: the wet rows
/// retire into the set the day a water substrate lands.
///
/// THE MID-LAYER PRESSURE is composed here as `rho g d / 2`, the lithostatic pressure at half the layer
/// depth, which is the chord the creep ladder declares it wants. It is formed from the column's OWN derived
/// density and its geometry rather than passed in, so it cannot disagree with the density the rest of the
/// cluster uses.
///
/// THE BUOYANCY CONTRAST is `rho alpha dT` from the same derived pair, so the viscosity, the Rayleigh number
/// and the density anomaly all read one density. Refuses rather than defaulting on any unrepresentable step.
#[allow(clippy::too_many_arguments)]
fn derive_column_viscosity(
    density_kg_m3: Fixed,
    thermal_conductivity_w_m_k: Fixed,
    specific_heat_j_kg_k: Fixed,
    thermal_expansion_ppm_per_k: Fixed,
    temperature_k: Fixed,
    geometry: &ColumnGeometry,
) -> Result<civsim_physics::convective_viscosity::ViscosityBand, ColumnDerivationRefusal> {
    use civsim_physics::convective_viscosity::{effective_viscosity_band, ViscosityInputs};
    use civsim_physics::creep_rows::{
        hk_dry_dislocation, hk_dry_dislocation_activation_volumes, select_activation_volume,
        CreepCandidate, VolumeConstraint,
    };

    let unrepresentable = |what: &str| ColumnDerivationRefusal::NoQuantity {
        quantity: what.to_string(),
    };
    // kappa = k / (rho c_p), computed here from the same three facts the struct exposes it from, so the
    // viscosity cannot run on a different diffusivity than the one the kernel reads.
    let thermal_diffusivity_m2_s = density_kg_m3
        .checked_mul(specific_heat_j_kg_k)
        .filter(|rho_cp| *rho_cp > Fixed::ZERO)
        .and_then(|rho_cp| thermal_conductivity_w_m_k.checked_div(rho_cp))
        .ok_or_else(|| unrepresentable("thermal_diffusivity"))?;

    // The buoyancy contrast the flow runs on, from this column's own density and expansivity.
    let density_anomaly_kg_m3 = civsim_physics::laws::thermal_density_anomaly(
        density_kg_m3,
        thermal_expansion_ppm_per_k,
        geometry.temperature_contrast_k,
    );
    // THE MAGNITUDE IS THE RIGHT QUANTITY HERE, and this says why rather than leaving a bare absolute value
    // to be read as an oversight. The creep solve wants the STRESS the buoyancy contrast applies, which is
    // set by how far the density departs from its reference and not by which way it departs: a rising light
    // parcel and a sinking heavy one of equal contrast drive the same strain rate through the same flow law.
    // `solve_ln_effective_viscosity` states the same convention from the other side, refusing a non-positive
    // contrast outright.
    //
    // WHAT THE MAGNITUDE MUST NOT DECIDE is whether the layer convects at all, which is a question about the
    // SIGN. That verdict lives in `ln_convection_onset` through `laws::buoyancy_drives_convection`, so a
    // stably stratified layer and a negative-expansion mantle are settled there rather than erased here.
    let density_anomaly_kg_m3 = if density_anomaly_kg_m3 < Fixed::ZERO {
        Fixed::ZERO - density_anomaly_kg_m3
    } else {
        density_anomaly_kg_m3
    };

    // Mid-layer lithostatic pressure in GPa: rho g (d/2), then Pa to GPa.
    // ORDER MATTERS HERE, and getting it wrong is not a style question. Formed left to right as
    // `rho g d / 2e9`, the intermediate `rho g d` is about `2.2e10` for a Mars-class mantle and OVERFLOWS
    // Q32.32's `2.1e9` ceiling, so the derivation refuses on a pressure it can perfectly well represent
    // (about 11 GPa). Dividing FIRST keeps every intermediate inside the window: `d / 2e9` is `9e-4`, and
    // the two multiplications climb back to order ten. This is the same representation discipline the
    // Rayleigh number needs, met by reassociation rather than by a wider type.
    let half_depth_scaled = geometry
        .layer_depth_m
        .checked_div(Fixed::from_int(2_000_000_000))
        .ok_or_else(|| unrepresentable("eval_pressure_gpa"))?;
    let eval_pressure_gpa = half_depth_scaled
        .checked_mul(density_kg_m3)
        .and_then(|x| x.checked_mul(geometry.gravity_m_s2))
        .ok_or_else(|| unrepresentable("eval_pressure_gpa"))?;

    // THE DISLOCATION-ONLY determinations, NOT the whole of Table 2. The ninth row of that table is a Si
    // SELF-DIFFUSION measurement (`V* = -2 cm^3/mol`, Bejina et al. 1997), a different mechanism, and its
    // chord covers 5 to 10 GPa. Feeding it to a dislocation column contaminates the bracket exactly where a
    // deep interior sits: at 10 GPa it drags the primary from `ln(eta) 51.2` to `47.9`, a viscosity 27 times
    // too low, and further down it is worse. `hk_dry_dislocation_activation_volumes` exists for this and its
    // docstring says so; this consumer failed to inherit the exclusion until an audit caught it.
    let volumes = hk_dry_dislocation_activation_volumes();
    // THE COVERAGE GATE. `select_activation_volume` reports whether any determination's pressure chord
    // actually COVERS the requested pressure, and when none does it falls back to the table's own extremes
    // and labels the result `UnconstrainedBySource`. That label is produced and then dropped: the solve
    // returns a `ViscosityBand` carrying only numbers, so a caller receives a viscosity with no signal that
    // the source constrains nothing there. Every dry-dislocation chord ends by 15 GPa, and an Earth-like
    // column derives about 47 GPa, so this is not a hypothetical: the previous version returned a confident
    // deep-mantle viscosity extrapolated past every measurement behind it. It refuses instead, which is the
    // same discipline the frame gate applies to the ambient thermoelastic rows.
    match select_activation_volume(&volumes, eval_pressure_gpa) {
        Some(bracket) if bracket.constraint() == VolumeConstraint::CoveredBySource => {}
        Some(_) => {
            return Err(ColumnDerivationRefusal::Viscosity(format!(
                "no activation-volume determination covers {:.1} GPa; the dry-dislocation chords end by \
                 15 GPa, so a viscosity here would be extrapolated past every measurement behind it",
                eval_pressure_gpa.to_f64_lossy()
            )))
        }
        None => {
            return Err(ColumnDerivationRefusal::Viscosity(
                "the activation-volume bracket did not resolve".to_string(),
            ))
        }
    }
    let candidates = [CreepCandidate {
        row: hk_dry_dislocation(),
        volumes: &volumes,
    }];
    let inputs = ViscosityInputs {
        density_anomaly_kg_m3,
        gravity_m_s2: geometry.gravity_m_s2,
        layer_depth_m: geometry.layer_depth_m,
        thermal_diffusivity_m2_s,
        // The ladder's chord is the interior POTENTIAL temperature, NOT the contrast that drives
        // buoyancy. They are different quantities and passing the contrast here would evaluate the creep
        // law at a few hundred kelvin instead of a few thousand.
        eval_temperature_k: temperature_k,
        eval_pressure_gpa,
        ln_rayleigh_critical: geometry.ln_rayleigh_critical,
    };
    effective_viscosity_band(&inputs, &candidates)
        .map_err(|e| ColumnDerivationRefusal::Viscosity(format!("{e:?}")))
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

/// The SI operating point a log-domain convection step runs on: the linear fields the kernel can hold,
/// plus the viscosity as a LOGARITHM because a real one cannot be held at all.
///
/// A Mars-class interior at 1600 K and 10.7 GPa solves to `ln(eta) ~ 54.7`, that is `eta ~ 5e23 Pa*s`
/// against a `Fixed::MAX` of `2.1e9`. The linear [`convection_step`] therefore cannot run on SI values, and
/// that is the whole reason its callers pass a scaled operating point with an authored fixture cluster.
/// Nothing about the PHYSICS required that; it was a representation limit, and the log-domain law forms
/// ([`laws::ln_rayleigh_number`], [`laws::ln_stokes_velocity`]) are the documented way out.
#[derive(Clone, Copy, Debug)]
pub struct LogConvectionInputs {
    /// Density at the column's own state (kg/m^3).
    pub density_kg_m3: Fixed,
    /// Volumetric thermal expansivity (ppm/K).
    pub thermal_expansion_ppm: Fixed,
    /// Surface gravity (m/s^2).
    pub gravity_m_s2: Fixed,
    /// Convecting layer depth (m).
    pub depth_m: Fixed,
    /// The buoyant parcel scale (m), the convective cell half-wavelength.
    pub parcel_radius_m: Fixed,
    /// `ln(eta)` in `ln Pa*s`. Never exponentiated.
    pub ln_viscosity: Fixed,
    /// Thermal diffusivity (m^2/s).
    pub thermal_diffusivity_m2_s: Fixed,
    /// `ln(Ra_crit)`, the onset threshold in the same domain the Rayleigh number is computed in.
    pub ln_rayleigh_critical: Fixed,
}

/// The Rayleigh number and convection-onset verdict for an SI operating point, IN LOGS.
///
/// Returns `(ln Ra, convecting)`. The comparison happens in the log domain on both sides, so neither `Ra`
/// nor `Ra_crit` has to materialise, and there is no overflow guard to author: `ln Ra` for a real mantle is
/// a small representable number near 14, and the linear form's `ra_max` clamp (a REPRESENTABILITY guard,
/// never a physical ceiling) has nothing to guard here. `None` when an input is non-physical.
// @derives: a column's log-domain Rayleigh number and convection onset <- its SI buoyancy, gravity, depth, log viscosity and diffusivity
pub fn ln_convection_onset(
    inputs: &LogConvectionInputs,
    delta_t: Fixed,
    latched: bool,
) -> Option<(Fixed, bool)> {
    let delta_rho =
        laws::thermal_density_anomaly(inputs.density_kg_m3, inputs.thermal_expansion_ppm, delta_t);
    let ln_ra = laws::ln_rayleigh_number(
        delta_rho,
        inputs.gravity_m_s2,
        inputs.depth_m,
        inputs.ln_viscosity,
        inputs.thermal_diffusivity_m2_s,
    )?;
    // THE SIGN IS THE REGIME AND THE MAGNITUDE CANNOT SUPPLY IT. `ln_rayleigh_number` takes `|delta_rho|`,
    // which is right for the ratio it computes and says nothing about which way the buoyancy points. Read
    // without this gate, a stably stratified layer produces a large `ln Ra` and is declared convecting, and
    // the more stable it is the more vigorously it appears to convect. It also governs the alien case: a
    // negative-expansion mantle is pinned rather than overturning, and it reaches this test as a data row
    // through the sign `thermal_density_anomaly` already composed.
    let destabilizing = laws::buoyancy_drives_convection(delta_rho);
    // The latch is one-way WITHIN a regime, matching the linear kernel: onset fires once and does not
    // un-fire on a dip. A sign flip is a change of regime rather than a dip, so it does un-latch.
    Some((
        ln_ra,
        destabilizing && (latched || ln_ra >= inputs.ln_rayleigh_critical),
    ))
}

/// An SI convection column: every field in metres, kilograms, seconds and kelvin, with the two quantities
/// that cannot be held linearly carried as logarithms.
///
/// # Why this exists beside [`ColumnParams`]
///
/// [`ColumnParams`] runs on a SCALED operating point with an authored fixture cluster (density 1,
/// conductivity 2, specific heat 10, a diffusivity that disagrees with `k/(rho c_p)` twentyfold). That
/// cluster is not a physics choice: it is what a linear Q32.32 kernel can hold. This type holds the real
/// derived values instead, and the three places SI would break are handled rather than avoided.
///
/// # The three representation problems, each solved rather than scaled away
///
/// **The viscosity overflows.** A derived interior viscosity is `~5.5e23 Pa*s` against a `Fixed::MAX` of
/// `2.1e9`. It is carried as `ln_viscosity` and never exponentiated; the Rayleigh number and the Stokes
/// velocity are computed in logs by [`laws::ln_rayleigh_number`] and [`laws::ln_stokes_velocity`], which
/// were built for exactly this and are twinned against their linear forms.
///
/// **The heat rates quantize to zero.** A radiogenic budget is `~5e-12 W/kg` and the full-depth conductive
/// loss is `~2.4e-13 W/kg`, against a `Fixed` resolution of `2.33e-10`. Stored as per-second RATES both are
/// zero, and a column whose production and loss are both zero never changes temperature. So the heat terms
/// are carried as PER-TICK ENERGY (J/kg): over a 1 Myr tick the same conductive loss is `7.7 J/kg`, which
/// is `3.3e10` ulp. [`laws::internal_heat_evolution`] then takes them with `dt = 1` and is unchanged.
///
/// **The tick length does not fit either.** One megayear is `3.1557e13 s` against the same `2.1e9`
/// ceiling, so `dt` is carried as `ln_dt_s` and the per-tick conductive loss is composed in logs. This is
/// the problem BENEATH the ordering hazard rather than the same one: `k dt` overflowing at `7.8e13` can be
/// reordered around, but an unrepresentable input cannot be, and only the second is fixable by care.
#[derive(Clone, Copy, Debug)]
pub struct SiColumnParams {
    /// The cold reference the contrast is measured against (K).
    pub reference_temperature_k: Fixed,
    /// Density at the column's own state (kg/m^3).
    pub density_kg_m3: Fixed,
    /// Thermal conductivity (W/m/K).
    pub thermal_conductivity_w_m_k: Fixed,
    /// Volumetric thermal expansivity (ppm/K).
    pub thermal_expansion_ppm: Fixed,
    /// Specific heat (J/kg/K).
    pub specific_heat_j_kg_k: Fixed,
    /// Thermal diffusivity (m^2/s), `k / (rho c_p)`.
    pub thermal_diffusivity_m2_s: Fixed,
    /// `ln(eta)` in `ln Pa*s`. Never exponentiated.
    pub ln_viscosity: Fixed,
    /// Surface gravity (m/s^2).
    pub gravity_m_s2: Fixed,
    /// Convecting layer depth (m).
    pub depth_m: Fixed,
    /// The buoyant parcel scale (m), the convective cell half-wavelength.
    pub parcel_radius_m: Fixed,
    /// Radiogenic heating as PER-TICK ENERGY (J/kg), not a rate. See the type documentation.
    pub heat_production_j_per_kg: Fixed,
    /// `ln(Ra_crit)`, so the onset comparison happens in the domain the Rayleigh number is computed in.
    pub ln_rayleigh_critical: Fixed,
    /// `ln(dt)` for the tick length in SECONDS. A logarithm because the tick itself does not fit: one
    /// megayear is `3.1557e13 s` against a `Fixed::MAX` of `2.1e9`, so the field cannot hold `dt` at all.
    /// `ln(dt) = 31.08` is comfortable.
    pub ln_dt_s: Fixed,
    /// The Nusselt PREFACTOR `a` in `Nu = a (Ra/Ra_crit)^(1/3)`, from
    /// [`civsim_physics::convection_scaling`] (`1.0` for the single-lid planetary case), never authored
    /// here.
    pub nusselt_prefactor: Fixed,
}

impl SiColumnParams {
    /// The conductive loss as PER-TICK ENERGY per kelvin of contrast: `k dt / (rho d^2)` [J/kg/K].
    ///
    /// COMPUTED IN LOGS, and not as a stylistic preference: the tick length does not fit. One megayear is
    /// `3.1557e13 s` against a `Fixed::MAX` of `2.1e9`, so there is no ordering of a linear composition
    /// that helps, because the input itself is unrepresentable before any multiply happens. An earlier
    /// version of this method reordered the linear form to `dt/d -> /d -> *k -> /rho` and claimed the
    /// ordering was the correctness; the ordering IS a real hazard (`k dt` alone is `7.8e13`, twenty
    /// thousand times past the ceiling) but it is the second problem, not the first. Its own test caught
    /// the first one by failing to construct the operating point.
    ///
    /// `ln_dt - 2 ln d + ln k - ln rho`, exponentiated once, has every term near unity and no hazard at
    /// all. For the Mars-class column that is `31.083 - 28.807 + 0.901 - 8.118 = -4.941`, giving
    /// `7.15e-3 J/kg/K`.
    // @derives: a column's per-tick conductive loss energy per kelvin <- its conductivity, density, depth and log tick length
    pub fn conductive_loss_energy_per_kelvin(&self) -> Option<Fixed> {
        if self.depth_m <= Fixed::ZERO
            || self.density_kg_m3 <= Fixed::ZERO
            || self.thermal_conductivity_w_m_k <= Fixed::ZERO
        {
            return None;
        }
        let two_ln_d = self.depth_m.ln().checked_mul(Fixed::from_int(2))?;
        let ln_value = self
            .ln_dt_s
            .checked_sub(two_ln_d)?
            .checked_add(self.thermal_conductivity_w_m_k.ln())?
            .checked_sub(self.density_kg_m3.ln())?;
        Some(ln_value.exp())
    }
}

/// One SI convection step on real derived values.
///
/// # The heat loss uses the repository's OWN parameterized-convection law, and finding that mattered
///
/// [`convection_step`] composes its convective loss from [`laws::stokes_velocity`] into
/// [`laws::heat_advection`]: a buoyant PARCEL settling at Stokes velocity, carrying the full interior
/// contrast across the full depth. On the fixture cluster nobody could tell whether that was right, because
/// there was no real magnitude to check it against. On the DERIVED values it is measurably wrong.
///
/// Measured on a Mars-class column (`rho = 3354.5`, `k = 2.461`, `c_p = 1241`, `alpha = 25.9 ppm/K`,
/// `ln eta = 54.66`, `d = 1.8e6 m`, `dT = 1300 K`), the Stokes-advection form gives `12.7 K/Myr` of
/// cooling. The observational constraint is a Mars-class surface heat flux near `0.025 W/m^2`, which over
/// the same column is `0.105 K/Myr`. The form overestimates by about 121 times.
///
/// The velocity itself is NOT the problem and is worth keeping: it comes out at `5.6e-10 m/s`, about
/// `1.8 cm/yr`, which is a plausible mantle overturn rate and a real observable. What is wrong is treating
/// that velocity times the FULL contrast over the FULL depth as the heat transport. A convecting mantle
/// loses heat across the thin thermal BOUNDARY LAYER the flow maintains, which is exactly what
/// [`laws::mantle_convective_heat_flux`] computes and what this uses instead:
///
/// ```text
///   Nu = max(1, a exp((ln Ra - ln Ra_crit) / 3))      the boundary-layer enhancement
///   loss_per_tick = Nu * k * dT * dt / (rho * d^2)    [J/kg]
/// ```
///
/// `Nu >= 1` is the definition rather than a floor (convection never transports less than conduction), so
/// the SAME expression covers the conducting and convecting cases and there is no separate conductive term
/// to add. Double-counting them would be the error the clamp exists to prevent. The same column now gives
/// `0.012 K/Myr`, within an order of magnitude of the observational `0.105` rather than 121 times past it,
/// and the residual is the honest gap: this is the MOBILE-LID isoviscous instance, and a Mars-class body is
/// stagnant-lid, whose suppression through the rheological temperature scale is the flagged follow-on
/// recorded on that law.
///
/// That law was already in the tree with an owner ruling behind it and the convection step did not read it.
/// It is the third banked-but-unread finding of this arc, after `K'` and the anchor column.
///
/// Returns `None` when an input is non-physical or an intermediate leaves the window. A `None` is a REFUSAL
/// and never a silently unchanged column.
// @derives: an SI interior column's next temperature and convection state <- its derived thermal properties, log viscosity, boundary-layer Nusselt enhancement and per-tick radiogenic energy
pub fn convection_step_si(state: &ColumnState, p: &SiColumnParams) -> Option<ColumnState> {
    let delta_t = state.temperature - p.reference_temperature_k;

    // Onset, entirely in logs. No `ra_max` clamp exists here because none is needed: the linear form's is a
    // REPRESENTABILITY guard, not a physical Rayleigh ceiling, and there is nothing to overflow.
    let inputs = LogConvectionInputs {
        density_kg_m3: p.density_kg_m3,
        thermal_expansion_ppm: p.thermal_expansion_ppm,
        gravity_m_s2: p.gravity_m_s2,
        depth_m: p.depth_m,
        parcel_radius_m: p.parcel_radius_m,
        ln_viscosity: p.ln_viscosity,
        thermal_diffusivity_m2_s: p.thermal_diffusivity_m2_s,
        ln_rayleigh_critical: p.ln_rayleigh_critical,
    };
    // A column at its reference temperature has no buoyancy and so no Rayleigh number. That is not a
    // failure, it is a column that is not convecting, so the latch holds and the enhancement stays at one.
    let (ln_ra, convecting) = match ln_convection_onset(&inputs, delta_t, state.convecting) {
        Some(v) => v,
        None => (p.ln_rayleigh_critical, state.convecting),
    };

    // THE BOUNDARY-LAYER ENHANCEMENT, in logs so no linear Rayleigh number has to form.
    // `Nu = a (Ra/Ra_crit)^(1/3)` is `a exp((ln Ra - ln Ra_crit)/3)`, clamped at unity by its definition.
    let nusselt = if convecting && ln_ra > p.ln_rayleigh_critical {
        let excess = ln_ra
            .checked_sub(p.ln_rayleigh_critical)?
            .checked_div(Fixed::from_int(3))?;
        p.nusselt_prefactor
            .checked_mul(excess.exp())?
            .max(Fixed::ONE)
    } else {
        Fixed::ONE
    };

    // The loss for this tick, in J/kg. `Nu = 1` is the pure-conduction case, so this ONE expression covers
    // both regimes and there is no second term to add.
    //
    // THE CONTRAST ENTERS SIGNED, and it used to enter as a magnitude. `internal_heat_evolution` subtracts
    // this term unconditionally (`net = production - loss`), so an absolute value made a column BELOW its
    // reference temperature cool further, away from the reference rather than toward it. Conduction reverses
    // when the gradient does: a column colder than its surface gains heat, and the signed contrast is what
    // says so. Byte-neutral wherever the interior sits above its reference, which is every column in the two
    // pinned scenarios and every fixture in this file, so the correction is visible only where it matters.
    let loss = p
        .conductive_loss_energy_per_kelvin()?
        .checked_mul(delta_t)?
        .checked_mul(nusselt)?;

    // `dt = 1` because the production and the loss are ALREADY per-tick energies. The law is unchanged: it
    // divides the net energy by the heat capacity to get a temperature increment.
    let temperature = laws::internal_heat_evolution(
        state.temperature,
        p.heat_production_j_per_kg,
        loss,
        p.specific_heat_j_kg_k,
        Fixed::ONE,
    );
    Some(ColumnState {
        temperature,
        convecting,
    })
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
    /// THE CLUSTER REFUSES A MIXED-FRAME STATE, which is the honest behaviour and replaces an earlier test
    /// asserting that all seven properties derive at mantle conditions.
    ///
    /// That earlier claim was wrong and an audit caught it. The Grueneisen rows, the bulk moduli and the
    /// molar volumes are AMBIENT values near 300 K and 1 bar, and `gruneisen.rs` stores each row's frame
    /// precisely "so a caller cannot silently treat an ambient aggregate as a deep-interior value". Reading
    /// them at 1600 K and 100 kbar produced a number that agreed with measurement at one temperature by
    /// cancellation (a high-temperature Dulong-Petit capacity against 300 K gamma, modulus and volume)
    /// rather than by physics.
    ///
    /// The corrected ruling is sharper than "all seven or none": a column consumes one STATE-COHERENT
    /// property bundle or it refuses. Atomicity does not license moving seven fields together when the
    /// fields do not describe the same thermodynamic state. Carrying the rows to interior conditions is a
    /// state-resolved thermoelastic provider and a real arc, not a patch.
    #[test]
    fn the_cluster_derives_one_coherent_state_at_depth() {
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
        let anchors_tbl = civsim_physics::thermoelastic_anchors::ThermoelasticAnchors::standard()
            .expect("anchors");
        let tables = BankedTables {
            anchors: &anchors_tbl,
            registry: &registry,
            periodic: &periodic,
            conductivity: &conductivity,
            gruneisen: &gruneisen,
            moduli: &moduli,
        };
        let composition = vec![
            ("Mg".to_string(), Fixed::from_int(2)),
            ("Si".to_string(), Fixed::ONE),
            ("O".to_string(), Fixed::from_int(4)),
        ];
        let geometry = ColumnGeometry {
            layer_depth_m: Fixed::from_int(1_800_000),
            gravity_m_s2: Fixed::from_ratio(37, 10),
            temperature_contrast_k: Fixed::from_int(1300),
            ln_rayleigh_critical: crate::deeptime::RIGID_RIGID_RA_CRIT.ln(),
        };

        // A MANTLE STATE NOW DERIVES, and the whole cluster comes back state-resolved. Before the
        // thermoelastic ladder was wired in this refused by name, which was correct and was a scoping
        // limit rather than a destination. All seven fields now describe 1600 K and the pressure the
        // column's own density implies at that depth.
        //
        // THE PRESSURE IS SOLVED RATHER THAN ASKED FOR, and that sentence above is the reason. This test
        // used to hand in 100_000 bar (10 GPa) while asserting the fields described "the pressure the
        // column's own density implies at that depth". They did not: the derived density implies 11.17 GPa
        // at this geometry, an 11.7 percent disagreement past the coherence gate's own 10 percent slack.
        // It passed because that gate was DEAD, formed as `rho g d` which overflows Q32.32 for any real
        // mantle, so the comparison returned `None` and skipped rather than refusing. Selecting an
        // assemblage at one pressure and evaluating the column at a depth implying another is the exact
        // within-bundle incoherence this cluster's ruling forbids, and the test asserting coherence was
        // the thing hiding it.
        //
        // The disagreement was never a bad input, it was a fixed point left open: density depends on the
        // assemblage, the assemblage on pressure, and pressure on density. So the depth is the input now
        // and the pressure is an output.
        let (solved_bar, deep) =
            solve_column_at_depth(&composition, Fixed::from_int(1600), &geometry, &tables)
                .expect("the depth-pressure fixed point settles and the ladder answers there");

        // THE SOLVED PRESSURE IS THE ONE THE COLUMN'S OWN MASS MAKES, checked here against the same
        // `rho g d / 2` the gate forms, so the fixed point is asserted rather than assumed to have run.
        let solved_gpa = solved_bar.to_f64_lossy() / 10_000.0;
        let implied_gpa = deep.density_kg_m3.to_f64_lossy() * 3.7 * 1_800_000.0 / 2.0 / 1.0e9;
        assert!(
            (solved_gpa - implied_gpa).abs() / implied_gpa < 0.10,
            "the solved pressure {solved_gpa:.3} GPa must agree with the density-implied \
             {implied_gpa:.3} GPa, since that agreement is what makes the seven fields one state"
        );
        assert!(
            (10.5..=12.0).contains(&solved_gpa),
            "the fixed point lands near the 11.17 GPa this geometry and density imply, read \
             {solved_gpa:.3} GPa"
        );

        // THE DENSITY IS CARRIED TO THE STATE, and this assertion is the one that would have caught the
        // defect the first version of this wire shipped. That version took the expansivity from the ladder
        // at 41.94 cm^3/mol and left the density reading the registry's ambient 43.60, so the bundle
        // reported 3223 kg/m^3: two volumes for one rock, in a cluster whose entire ruling is that the
        // seven fields describe ONE thermodynamic state.
        let rho = deep.density_kg_m3.to_f64_lossy();
        assert!(
            (3300.0..=3420.0).contains(&rho),
            "forsterite at 1600 K and about 11 GPa compresses to roughly 3355 kg/m^3, well above its \
             ambient 3223; read {rho:.1}. An ambient density here would be four percent light AND would \
             disagree with the volume its own expansivity was solved at"
        );

        // The expansivity is the one solved AT that state, not the ambient row: compression suppresses it
        // from the ~29 ppm/K forsterite shows at 300 K and 1 bar, and heating alone would raise it.
        let ppm = deep.thermal_expansion_ppm_per_k.to_f64_lossy();
        assert!((18.0..=34.0).contains(&ppm), "read {ppm:.1} ppm/K at depth");

        // AND THE PRESSURE THE VISCOSITY WAS EVALUATED AT AGREES WITH THAT DENSITY, which is the
        // coherence the bundle promises: a denser column implies a higher lithostatic pressure at the same
        // depth, and the state-resolved density moved it from 10.73 to 11.17 GPa rather than leaving the
        // creep ladder evaluated at a pressure the density no longer supports.
        let p_gpa = deep.viscosity.eval_pressure_gpa.to_f64_lossy();
        let implied = rho * 3.7 * 1_800_000.0 / 2.0 / 1e9;
        assert!(
            (p_gpa - implied).abs() < 0.05,
            "the creep evaluation pressure {p_gpa:.3} GPa must be the mid-layer lithostatic pressure of \
             THIS density, {implied:.3} GPa"
        );

        // AT ITS OWN FRAME the ambient join still runs, checked against an INDEPENDENT anchor: Ye,
        // Schwering and Smyth (2009) single-crystal XRD give forsterite `alpha(300 K) ~ 29.1 +/- 2.6
        // ppm/K`, a different measurement from the Anderson and Isaak gamma row this derivation reads, so
        // recovering it is a check rather than a back-solve of the source's own number.
        let ambient =
            civsim_materials::properties::ambient_assemblage_volumetric_expansivity_per_k(
                &[("forsterite".to_string(), Fixed::ONE)],
                Fixed::from_int(300),
                Fixed::ONE,
                &registry,
                &moduli,
                &gruneisen,
            )
            .expect("forsterite carries a complete ambient row");
        let ppm = ambient.to_f64_lossy() * 1e6;
        assert!(
            (20.0..=45.0).contains(&ppm),
            "the ambient-frame expansivity should be within reach of the independent 29.1 ppm/K anchor, \
             read {ppm:.1}"
        );
    }

    /// THE COHERENCE GATE IS LIVE AT PLANETARY SCALE, which is the property that was missing rather than
    /// the comparison itself.
    ///
    /// The gate existed and read correctly; it simply never ran. Formed as `rho g d` the intermediate is
    /// `2.2e10` at Mars scale against Q32.32's `2.1e9` ceiling, so `checked_mul` returned `None` and the
    /// `if let Some(..)` SKIPPED. Measured, the band that survived was a density below 322 kg/m^3 at Mars
    /// geometry and below 76 at Earth whole-mantle scale, so no silicate column ever reached the check. A
    /// gate that fails open is worse than an absent one, because the green test reads as evidence.
    ///
    /// This asserts the representation directly, at both scales, so the reassociation cannot silently
    /// regress into an ordering that overflows again.
    #[test]
    fn the_lithostatic_pressure_is_representable_at_mars_and_earth_scale() {
        let mars = ColumnGeometry {
            layer_depth_m: Fixed::from_int(1_800_000),
            gravity_m_s2: Fixed::from_ratio(37, 10),
            temperature_contrast_k: Fixed::from_int(1300),
            ln_rayleigh_critical: crate::deeptime::RIGID_RIGID_RA_CRIT.ln(),
        };
        let earth = ColumnGeometry {
            layer_depth_m: Fixed::from_int(2_900_000),
            gravity_m_s2: Fixed::from_ratio(981, 100),
            ..mars
        };
        let rho = Fixed::from_ratio(33545, 10);

        let mars_gpa = implied_lithostatic_gpa(rho, &mars).expect(
            "a Mars-class column's own lithostatic pressure must be representable; the old ordering \
             overflowed here and the gate skipped",
        );
        let earth_gpa = implied_lithostatic_gpa(rho, &earth)
            .expect("and so must an Earth whole-mantle column's");

        let m = mars_gpa.to_f64_lossy();
        let e = earth_gpa.to_f64_lossy();
        assert!(
            (11.0..=11.4).contains(&m),
            "3354.5 * 3.7 * 1.8e6 / 2 / 1e9 = 11.17 GPa, read {m:.3}"
        );
        assert!(
            (46.0..=49.0).contains(&e),
            "3354.5 * 9.81 * 2.9e6 / 2 / 1e9 = 47.7 GPa, read {e:.3}"
        );

        // AND THE ORDER IS WHY, stated as the arithmetic rather than as a comment: the numerator the old
        // form built first does not fit, while the answer it was reaching for fits comfortably.
        assert!(
            rho.checked_mul(mars.gravity_m_s2)
                .and_then(|x| x.checked_mul(mars.layer_depth_m))
                .is_none(),
            "`rho g d` must still overflow, since that is the hazard the reassociation exists to avoid"
        );
    }

    /// A Mars-class SI column built from the DERIVED cluster, the operating point every SI test uses.
    fn mars_si_column() -> SiColumnParams {
        let d = Fixed::from_int(1_800_000);
        SiColumnParams {
            reference_temperature_k: Fixed::from_int(300),
            density_kg_m3: Fixed::from_ratio(33545, 10),
            thermal_conductivity_w_m_k: Fixed::from_ratio(2461, 1000),
            thermal_expansion_ppm: Fixed::from_ratio(259, 10),
            specific_heat_j_kg_k: Fixed::from_int(1241),
            thermal_diffusivity_m2_s: Fixed::from_ratio(59117, 100_000_000_000),
            ln_viscosity: Fixed::from_ratio(5466, 100),
            gravity_m_s2: Fixed::from_ratio(37, 10),
            depth_m: d,
            parcel_radius_m: Fixed::from_ratio(18144, 10_000)
                .checked_mul(Fixed::from_int(1_000_000))
                .unwrap(),
            heat_production_j_per_kg: Fixed::ZERO,
            ln_rayleigh_critical: crate::deeptime::RIGID_RIGID_RA_CRIT.ln(),
            // ln(1 Myr in seconds) = ln(3.1557e13) = 31.083. The tick itself does not fit.
            ln_dt_s: Fixed::from_ratio(31_083, 1000),
            nusselt_prefactor: Fixed::ONE,
        }
    }

    /// THE ORDERING IS THE CORRECTNESS, and this asserts the value the ordering exists to reach.
    ///
    /// `k dt / (rho d^2)` has a true value near `7.15e-3 J/kg/K`, and its natural reading forms
    /// `k dt = 7.8e13` on the first multiply, twenty thousand times past `Fixed::MAX`. Dividing the large
    /// `dt` down by the two depths FIRST reaches the same answer with every intermediate inside the window.
    #[test]
    fn the_conductive_loss_energy_survives_its_own_intermediates() {
        let p = mars_si_column();
        let per_k = p
            .conductive_loss_energy_per_kelvin()
            .expect("the ordered composition stays representable");
        let v = per_k.to_f64_lossy();
        assert!(
            (7.0e-3..=7.3e-3).contains(&v),
            "k dt / (rho d^2) = 2.461 * 3.1557e13 / (3354.5 * 3.24e12) = 7.15e-3 J/kg/K; read {v:.4e}"
        );
        // AND THE TICK ITSELF DOES NOT FIT, which is the problem beneath the ordering hazard: no
        // reordering of a linear composition helps when an INPUT is unrepresentable before any multiply.
        assert!(
            Fixed::from_int(31_557_000)
                .checked_mul(Fixed::from_int(1_000_000))
                .is_none(),
            "1 Myr in seconds is 3.1557e13 against a Fixed::MAX of 2.1e9, so dt cannot be held linearly"
        );
    }

    /// THE COOLING RATE AGAINST THE OBSERVATIONAL CONSTRAINT, which is the check that convicted the law
    /// the linear kernel had been using.
    ///
    /// A Mars-class surface heat flux is about `0.025 W/m^2`. Over this column that is
    /// `0.025 / (3354.5 * 1.8e6) = 4.1e-12 W/kg`, which over a 1 Myr tick is `131 J/kg` and
    /// `0.105 K/Myr`. That number comes from observation and not from anything this kernel computes, so
    /// agreeing with it is evidence.
    ///
    /// The Stokes-parcel form the linear kernel uses gives `12.7 K/Myr`, 121 times past it. The
    /// boundary-layer form gives about `0.012 K/Myr`, within an order of magnitude and on the LOW side,
    /// which is the honest direction for a mobile-lid law applied to a stagnant-lid body.
    #[test]
    fn the_cooling_rate_lands_within_an_order_of_the_observational_constraint() {
        let p = mars_si_column();
        let state = ColumnState {
            temperature: Fixed::from_int(1600),
            convecting: true,
        };
        let next = convection_step_si(&state, &p).expect("the SI column steps");
        let cooled = state.temperature.to_f64_lossy() - next.temperature.to_f64_lossy();
        assert!(
            cooled > 0.0,
            "with no radiogenic production a hot column must COOL; it moved {cooled:+.4} K"
        );
        assert!(
            (0.002..=0.20).contains(&cooled),
            "the observational constraint is 0.105 K/Myr and the boundary-layer form gives about 0.012; \
             read {cooled:.4} K/Myr. The Stokes-parcel form this replaced gives 12.7, which is what \
             convicted it"
        );
    }

    /// A COLUMN BELOW ITS REFERENCE WARMS TOWARD IT, which the magnitude form made impossible.
    ///
    /// `internal_heat_evolution` subtracts the loss term unconditionally (`net = production - loss`), and the
    /// loss used to be built on `|delta_t|`. So a column COLDER than its surface reference produced a
    /// positive loss and cooled further, away from the reference rather than toward it, and the further
    /// below it sat the faster it ran away. Conduction reverses when the gradient does; the signed contrast
    /// is what says so.
    ///
    /// This runs the same fixture from both sides of its reference, which is what makes it a twin rather
    /// than a single-sided assertion.
    #[test]
    fn a_column_below_its_reference_warms_rather_than_cooling_further() {
        let p = mars_si_column();
        let reference = p.reference_temperature_k.to_f64_lossy();

        // ABOVE the reference it cools, which is the case that already worked and must keep working.
        let hot = ColumnState {
            temperature: Fixed::from_int(1600),
            convecting: true,
        };
        let hot_next = convection_step_si(&hot, &p).expect("a hot column steps");
        assert!(
            hot_next.temperature < hot.temperature,
            "a column above its reference sheds heat, moved to {}",
            hot_next.temperature.to_f64_lossy()
        );

        // BELOW the reference it warms. With no radiogenic production the ONLY term is the conductive one,
        // so the direction here is that term's sign and nothing else.
        let cold = ColumnState {
            temperature: Fixed::from_int(100),
            convecting: false,
        };
        assert!(
            (cold.temperature.to_f64_lossy()) < reference,
            "the cold fixture must actually sit below the reference for this to test anything"
        );
        let cold_next = convection_step_si(&cold, &p).expect("a cold column steps");
        assert!(
            cold_next.temperature > cold.temperature,
            "a column below its reference gains heat by conduction, moved from {} to {}",
            cold.temperature.to_f64_lossy(),
            cold_next.temperature.to_f64_lossy()
        );
        // AND IT DOES NOT OVERSHOOT the reference in one tick, which would be a different defect.
        assert!(
            cold_next.temperature.to_f64_lossy() <= reference,
            "warming toward the reference must not cross it in a single tick, reached {}",
            cold_next.temperature.to_f64_lossy()
        );
    }

    /// A STABLY STRATIFIED LAYER DOES NOT CONVECT AT ANY MAGNITUDE, and a negative-expansion mantle is a
    /// data row through the same test rather than a special case.
    ///
    /// `ln_rayleigh_number` takes `|delta_rho|`, which is right for the dimensionless ratio it forms and
    /// silent about the regime. Read without a sign gate, a stably stratified layer produces a large
    /// `ln Ra` and is declared convecting, and the more stable it is the more vigorously it appears to
    /// convect. The sign arrives already composed by `thermal_density_anomaly`, so all four rows below run
    /// the same predicate and nothing keys on the material being alien.
    #[test]
    fn the_convection_verdict_reads_the_sign_of_the_buoyancy_and_not_its_size() {
        let p = mars_si_column();
        let inputs = LogConvectionInputs {
            density_kg_m3: p.density_kg_m3,
            thermal_expansion_ppm: p.thermal_expansion_ppm,
            gravity_m_s2: p.gravity_m_s2,
            depth_m: p.depth_m,
            parcel_radius_m: p.parcel_radius_m,
            ln_viscosity: p.ln_viscosity,
            thermal_diffusivity_m2_s: p.thermal_diffusivity_m2_s,
            ln_rayleigh_critical: p.ln_rayleigh_critical,
        };
        let hot = Fixed::from_int(1300);
        let cold = Fixed::ZERO - hot;

        // ORDINARY EXPANSION, INTERIOR HOTTER: heated from below, it overturns.
        let (ln_ra_up, up) = ln_convection_onset(&inputs, hot, false).expect("a contrast resolves");
        assert!(up, "an ordinary mantle hotter than its surface convects");
        assert!(
            ln_ra_up > inputs.ln_rayleigh_critical,
            "and it is supercritical, so the verdict is the sign rather than a small Ra"
        );

        // ORDINARY EXPANSION, INTERIOR COLDER: heated from above, stable at the SAME magnitude of Ra.
        let (ln_ra_down, down) =
            ln_convection_onset(&inputs, cold, false).expect("a contrast resolves");
        assert!(
            !down,
            "a layer heated from above is stably stratified and must not convect"
        );
        assert_eq!(
            ln_ra_up, ln_ra_down,
            "the two differ ONLY in sign, so an equal ln Ra is exactly what makes the magnitude \
             insufficient to decide"
        );

        // NEGATIVE THERMAL EXPANSION, INTERIOR HOTTER: heating makes the parcel denser, so the buoyancy
        // pins the layer rather than driving it. This is the alien row, and it takes no new code path.
        let mut nte = inputs;
        nte.thermal_expansion_ppm = Fixed::ZERO - p.thermal_expansion_ppm;
        let (_, nte_hot) = ln_convection_onset(&nte, hot, false).expect("a contrast resolves");
        assert!(
            !nte_hot,
            "a negative-expansion mantle hotter than its surface is STABLE, since heating densifies it"
        );

        // NEGATIVE EXPANSION, INTERIOR COLDER: the sign flips back and it overturns.
        let (_, nte_cold) = ln_convection_onset(&nte, cold, false).expect("a contrast resolves");
        assert!(
            nte_cold,
            "a negative-expansion mantle colder than its surface is the unstable one"
        );

        // THE LATCH DOES NOT SURVIVE A SIGN FLIP. It is one-way within a regime, and a change of regime is
        // not a dip: a column that was convecting and becomes stably stratified stops.
        let (_, latched_but_stable) =
            ln_convection_onset(&inputs, cold, true).expect("a contrast resolves");
        assert!(
            !latched_but_stable,
            "a one-way onset latch must not keep a stably stratified layer convecting"
        );
    }

    /// RADIOGENIC PRODUCTION OPPOSES THE LOSS, and both are per-tick energies so neither quantizes away.
    ///
    /// This is the second half of the representation finding: as per-second RATES the production is
    /// `~5e-12 W/kg` and the loss `~2.4e-13 W/kg`, against a `Fixed` resolution of `2.33e-10`. Both are
    /// ZERO, and a column whose production and loss are both zero never changes temperature at all.
    #[test]
    fn the_per_tick_energy_form_keeps_heat_production_from_quantizing_to_nothing() {
        let mut p = mars_si_column();
        let state = ColumnState {
            temperature: Fixed::from_int(1600),
            convecting: true,
        };
        let without = convection_step_si(&state, &p).expect("steps");

        // The per-tick radiogenic energy that balances the loss exactly at this contrast.
        let balance = p
            .conductive_loss_energy_per_kelvin()
            .and_then(|c| c.checked_mul(Fixed::from_int(1300)))
            .expect("representable");
        p.heat_production_j_per_kg = balance;
        let with = convection_step_si(&state, &p).expect("steps");
        assert!(
            with.temperature > without.temperature,
            "adding radiogenic production must slow the cooling: without {} K, with {} K",
            without.temperature.to_f64_lossy(),
            with.temperature.to_f64_lossy()
        );

        // AS A RATE it would have vanished: the same energy over the tick is 2.4e-13 W/kg, which is
        // 0.001 ulp and quantizes to exactly zero.
        let as_rate = balance
            .ln()
            .checked_sub(p.ln_dt_s)
            .expect("representable")
            .exp();
        assert_eq!(
            as_rate,
            Fixed::ZERO,
            "the SAME quantity carried as a per-second rate is exactly zero, which is the whole reason \
             the field holds energy"
        );
    }

    /// A column below onset conducts, and the SAME expression covers it because `Nu = 1` is the definition.
    #[test]
    fn a_sub_critical_column_conducts_through_the_same_expression() {
        let mut p = mars_si_column();
        // A far stiffer interior: ln eta up by 10 is eta up by 22026x, which drops Ra below critical.
        p.ln_viscosity += Fixed::from_int(10);
        let state = ColumnState {
            temperature: Fixed::from_int(1600),
            convecting: false,
        };
        let next = convection_step_si(&state, &p).expect("steps");
        assert!(
            !next.convecting,
            "a column 22000 times stiffer must sit below the onset"
        );
        let cooled = state.temperature.to_f64_lossy() - next.temperature.to_f64_lossy();
        let pure_conduction = p
            .conductive_loss_energy_per_kelvin()
            .unwrap()
            .to_f64_lossy()
            * 1300.0
            / 1241.0;
        assert!(
            (cooled - pure_conduction).abs() < 1e-6,
            "below onset the loss must be exactly the conductive one (Nu = 1), no convective term added: \
             stepped {cooled:.6}, conductive {pure_conduction:.6}"
        );
    }

    /// Determinism: the same SI query returns bit-identical results.
    #[test]
    fn the_si_step_is_bit_reproducible() {
        let p = mars_si_column();
        let state = ColumnState {
            temperature: Fixed::from_int(1600),
            convecting: true,
        };
        assert_eq!(
            convection_step_si(&state, &p),
            convection_step_si(&state, &p),
            "same inputs, same bits"
        );
    }

    /// THE LOG-DOMAIN TWIN: the two Rayleigh forms must agree where BOTH can run.
    ///
    /// This is the test that makes the lift a migration rather than a rewrite. The linear kernel cannot run
    /// on SI values (a real viscosity is `~5e23 Pa*s` against a `Fixed::MAX` of `2.1e9`), which is the whole
    /// reason its callers pass a scaled operating point with an authored fixture cluster. The log form can.
    /// But "the log form can run" is not "the log form computes the same thing", and asserting the second
    /// from the first is exactly the class of error this arc has already paid for twice today.
    ///
    /// So both are run on an operating point the LINEAR one can hold, fed identical inputs, and the linear
    /// result is compared against `exp(ln Ra)`. Where they disagree the migration is wrong, whatever the
    /// SI-only path reports, because there is nothing to check it against there.
    #[test]
    fn the_log_rayleigh_form_reproduces_the_linear_one_where_both_can_run() {
        // A scaled operating point, chosen so the LINEAR form stays inside its window.
        let density = Fixed::from_int(3);
        let expansion_ppm = Fixed::from_int(30);
        let delta_t = Fixed::from_int(400);
        let gravity = Fixed::from_ratio(37, 10);
        let depth = Fixed::from_int(2);
        let viscosity = Fixed::from_int(5);
        let kappa = Fixed::from_ratio(1, 100);

        let delta_rho = laws::thermal_density_anomaly(density, expansion_ppm, delta_t);
        let linear = laws::rayleigh_number(
            delta_rho,
            gravity,
            depth,
            viscosity,
            kappa,
            Fixed::from_int(1_000_000),
        );
        let ln_form = laws::ln_rayleigh_number(delta_rho, gravity, depth, viscosity.ln(), kappa)
            .expect("the log form answers on a physical operating point");
        let from_log = ln_form.exp().to_f64_lossy();
        let direct = linear.to_f64_lossy();
        assert!(
            direct > 0.0 && (from_log - direct).abs() / direct < 0.01,
            "the two Rayleigh forms must agree within a percent where both run: linear {direct:.4}, \
             exp(log) {from_log:.4}. A disagreement here means the log-domain lift changes the physics \
             rather than the representation"
        );

        // AND THE STOKES PAIR, same discipline: the advective loss reads this one, so a drift here moves a
        // convecting column's cooling rate while the onset verdict stays right.
        let radius = Fixed::from_int(1);
        let v_linear = laws::stokes_velocity(
            delta_rho,
            gravity,
            radius,
            viscosity,
            Fixed::from_int(1_000_000),
        )
        .to_f64_lossy();
        let v_log = laws::ln_stokes_velocity(delta_rho, gravity, radius, viscosity.ln())
            .expect("the log Stokes form answers")
            .exp()
            .to_f64_lossy();
        assert!(
            v_linear > 0.0 && (v_log - v_linear).abs() / v_linear < 0.01,
            "the two Stokes forms must agree within a percent: linear {v_linear:.6}, exp(log) {v_log:.6}"
        );
    }

    /// THE LOG FORM RUNS WHERE THE LINEAR ONE CANNOT, which is the reason to have it.
    ///
    /// At the real SI operating point the derived cluster now produces (a Mars-class mantle at 1600 K with
    /// `ln eta ~ 54.7`), the linear form falls to its overflow branch and reports a Rayleigh number of zero,
    /// so a column that plainly convects reads as conducting. The log form answers with a mantle-scale
    /// `ln Ra` and the onset fires. This is the defect the fixture cluster was papering over.
    #[test]
    fn the_linear_form_fails_at_si_where_the_log_form_answers() {
        let inputs = LogConvectionInputs {
            density_kg_m3: Fixed::from_int(3354),
            thermal_expansion_ppm: Fixed::from_int(26),
            gravity_m_s2: Fixed::from_ratio(37, 10),
            depth_m: Fixed::from_int(1_800_000),
            parcel_radius_m: Fixed::from_int(1_800_000),
            ln_viscosity: Fixed::from_ratio(547, 10),
            thermal_diffusivity_m2_s: Fixed::from_ratio(6, 10_000_000),
            ln_rayleigh_critical: crate::deeptime::RIGID_RIGID_RA_CRIT.ln(),
        };
        let delta_t = Fixed::from_int(1300);
        let (ln_ra, convecting) =
            ln_convection_onset(&inputs, delta_t, false).expect("the log form answers at SI");

        // THE VALUE IS COMPUTED HERE RATHER THAN GUESSED, because guessing it is how this assertion was
        // wrong the first time. `drho = rho alpha dT = 3354 * 26e-6 * 1300 = 113.4 kg/m^3`, `d^3 = 5.83e18`,
        // `eta = e^54.7 = 5.7e23`, `kappa = 6e-7`, so `Ra = 113.4 * 3.7 * 5.83e18 / (5.7e23 * 6e-7) = 7155`
        // and `ln Ra = 8.88`. The first version of this test asserted `> 10` from intuition about what a
        // "mantle-scale" Rayleigh number looks like and convicted correct arithmetic.
        //
        // It is lower than a textbook mantle `Ra ~ 1e6` for a stated reason worth carrying: the viscosity
        // is the DERIVED dry-dislocation value at 1600 K and 11 GPa, `~5.7e23 Pa*s`, a few hundred times the
        // `~1e21` usually quoted, and the diffusivity is the derived `6e-7 m^2/s` rather than the customary
        // `1e-6`. Both push `Ra` down. The column still convects, at about four times critical.
        let v = ln_ra.to_f64_lossy();
        assert!(
            (8.5..=9.3).contains(&v),
            "the computed ln Ra for this operating point is 8.88, read {v:.2}"
        );
        let critical = inputs.ln_rayleigh_critical.to_f64_lossy();
        assert!(
            v > critical,
            "and it must exceed the rigid-rigid ln(1707.76) = {critical:.2} for the onset to fire"
        );
        assert!(convecting, "so the latch fires at ln Ra = {v:.2}");

        // THE LINEAR FORM ON THE SAME INPUTS, which is the comparison that justifies the migration. Its
        // depth-cubed term alone is 5.8e18 against a Fixed::MAX of 2.1e9, so it cannot even form the
        // numerator and returns its overflow value.
        let delta_rho = laws::thermal_density_anomaly(
            inputs.density_kg_m3,
            inputs.thermal_expansion_ppm,
            delta_t,
        );
        let linear = laws::rayleigh_number(
            delta_rho,
            inputs.gravity_m_s2,
            inputs.depth_m,
            Fixed::from_int(1),
            inputs.thermal_diffusivity_m2_s,
            Fixed::from_int(1_000_000),
        );
        assert_ne!(
            linear.to_f64_lossy(),
            ln_ra.exp().to_f64_lossy(),
            "the linear form cannot reproduce the SI answer, which is why the operating point was scaled"
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
            // The band is irrelevant to this test, which is about the three facts kappa composes from.
            viscosity: civsim_physics::convective_viscosity::ViscosityBand {
                ln_viscosity_min: Fixed::from_int(46),
                ln_viscosity_max: Fixed::from_int(50),
                ln_viscosity_primary: Fixed::from_int(48),
                eval_temperature_k: Fixed::from_int(1600),
                eval_pressure_gpa: Fixed::from_int(10),
            },
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

    /// THE TWO DEPTHS AGREE WHEN THE GEOMETRY IS DERIVED FROM THE COLUMN, and that is the whole point of
    /// the checked constructor. Built by hand the pair can disagree by `1e6` with no complaint from either
    /// type; built from the column it cannot, because the SI depth is computed rather than accepted.
    #[test]
    fn the_derived_geometry_cannot_disagree_with_the_column_it_came_from() {
        let (_, mut p) = column(Fixed::from_int(1000));
        p.depth = Fixed::from_ratio(18, 10); // 1.8 Mm, the scaled kernel's unit
        p.gravity = Fixed::from_ratio(37, 10);
        let g = ColumnGeometry::from_scaled_column(
            &p,
            Fixed::from_int(1300),
            crate::deeptime::RIGID_RIGID_RA_CRIT.ln(),
        )
        .expect("a positive scaled depth converts");
        // Compared with a tolerance rather than exactly: `1.8` has no exact binary fixed-point
        // representation, so the scaled-to-SI product lands a fraction of a metre under 1,800,000. That is
        // a representation artifact of the conversion, not a disagreement between the two depths, and
        // asserting exact equality here would be asserting something about binary fractions rather than
        // about the unit coherence this test exists for.
        let drift = (g.layer_depth_m - Fixed::from_int(1_800_000)).abs();
        assert!(
            drift < Fixed::ONE,
            "the SI depth is DERIVED from the scaled one, within representation: read {}",
            g.layer_depth_m.to_f64_lossy()
        );
        assert_eq!(
            g.gravity_m_s2, p.gravity,
            "gravity comes from the column too"
        );
        // A degenerate scaled depth has no SI counterpart and refuses rather than yielding zero metres.
        p.depth = Fixed::ZERO;
        assert!(ColumnGeometry::from_scaled_column(
            &p,
            Fixed::from_int(1300),
            crate::deeptime::RIGID_RIGID_RA_CRIT.ln()
        )
        .is_none());
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
