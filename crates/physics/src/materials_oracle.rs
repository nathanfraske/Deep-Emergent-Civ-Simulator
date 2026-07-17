//! The materials oracle, Stage-6 property emission: a material's mechanical properties DERIVED from its stable
//! mineral assemblage rather than authored per rock type. This is the composition-to-property bridge the spec
//! (`docs/working/MATERIALS_ORACLE_SPEC.md`) calls `K_solid`, the mechanical sibling of the density
//! `crustal_density` already emits from the same assemblage: the assemblage's phases and their volume fractions
//! are the input, the aggregate stiffness is the derived output, so a rock's stiffness falls out of its
//! mineralogy and never an authored per-rock-type table (Principle 8).
//!
//! TIER (owner research, #182). The stiffness of a solid splits by bonding class, and the routes are not one.
//! The cohesive-energy-density estimator here, `M ~ E_coh / V`, is the METALLIC / invented-element /
//! QUICK-SCREEN tier `[E]`: a bonding-agnostic order-of-magnitude stiffness SCALE, legitimate where nothing
//! more principled is reachable (a metal, an invented phase with no structure, a first pass). It is NOT the
//! principled route for the seed registry's ionic-to-covalent OXIDE phases (forsterite, quartz, periclase,
//! corundum, hematite, fayalite): for those the bulk modulus is LATTICE CURVATURE on the Shannon radius,
//! `B = (n-1) A / (18 r0^4)` (Madelung `A`, Born exponent `n`, cation-anion distance `r0`), column-pure with no
//! `E_coh` at all and radius-dominated (`B ~ 1/r0^4`). That principled bulk-modulus route, and the shear-modulus
//! debt it exposes (a central-force model obeys the Cauchy relation `C12 = C44` as a theorem, so `G` needs one
//! class-dispatched ingredient beyond `B`), are the corrected carve in `MATERIALS_ORACLE_MODULUS_CARVE2.md`,
//! surfaced for the gate's ruling, not built here.
//!
//! What this module carries today, plainly labeled:
//!
//! 1. [`phase_cohesive_energy`] by Hess's law, exact from the floor: `E_coh = sum(atomization_i * count_i) -
//!    dH_f`, the atomization enthalpies from the periodic table (`[M, floor-and-validation]`) and the enthalpy
//!    of formation from the phase registry (`[M]`). Derived `[D]`, no authored step. This is the estimator input
//!    and the validation quantity, not the disposer's substrate (which cancels it by Hess).
//! 2. [`phase_elastic_modulus`], the `E_coh / V` quick-screen stiffness scale `[E]` (`1 kJ/cm^3 = 1 GPa`). A
//!    bonding-agnostic order-of-magnitude estimate, not a cleanly separated bulk or shear modulus; the principled
//!    ionic route above supersedes it for the oxide phases.
//! 3. [`assemblage_elastic_modulus`], the Voigt-Reuss-Hill aggregate of the per-phase quick-screen scales over
//!    their volume fractions, `[E]` (the aggregation is an exact `[D]` step, but it inherits the estimator tier
//!    of its inputs). Its half-gap is the derived aggregation band; the Hashin-Shtrikman tightening is a Stage-7
//!    follow-on.
//!
//! Everything here is fixed-point (the assemblage solve already is) and deterministic. Nothing reads the output
//! yet, so the pins hold. `mat.elastic_modulus` (Young's) is retired only by the principled `B` derived plus `G`
//! class-dispatched route, not by this quick-screen scale alone.

use crate::mineral_moduli::MineralModuli;
use crate::periodic::PeriodicTable;
use crate::petrology::Assemblage;
use crate::petrology_data::{Phase, PhaseRegistry};
use civsim_core::fixed::Fixed;

/// The provenance tag of an emitted value, the seven-tag register the spec shares with the derive-live gate:
/// `[D]` derived by an exact relation, `[M]` measured, `[E]` estimator (an approximate physical model), `[C]`
/// closure, `[A]` authored, `[W]` written state, `[X]` contingency. This is a LOCAL PLACEHOLDER for the
/// enforced enum Agent A is standing up in the provenance register (Phase 1); the oracle designs against these
/// semantics now and binds to the real enum when it lands on main, so the tag a value carries never drifts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// `[D]`: derived from other floor values by an exact relation (a Hess-law sum, a VRH aggregation).
    Derived,
    /// `[M]`: a measured component-level constant read from the floor.
    Measured,
    /// `[E]`: an approximate physical estimator, trustworthy to a stated band, never exact.
    Estimator,
    /// `[C]`: a closure constant.
    Closure,
    /// `[A]`: an authored value (the retirement target; a derivation never emits this).
    Authored,
    /// `[W]`: written simulation state.
    Written,
    /// `[X]`: a contingency fallback.
    Contingency,
}

/// A property estimate with its uncertainty band and provenance, the Stage-6 emission shape `{value, band,
/// provenance}`. The `value` and `band` are in the property's own unit (GPa for a modulus); the `band` is the
/// symmetric half-width of the derived uncertainty interval, so the estimate reads as `value +/- band`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyEstimate {
    /// The estimated property value, in the property's unit (GPa for the elastic modulus).
    pub value: Fixed,
    /// The symmetric half-width of the uncertainty band, in the property's unit. For the assemblage modulus
    /// this is the derived Voigt-Reuss half-gap; the estimator's own scatter is the reserved band documented on
    /// [`assemblage_elastic_modulus`], surfaced not baked.
    pub band: Fixed,
    /// The provenance tag of the emitted value.
    pub provenance: Provenance,
}

/// The COHESIVE ENERGY of a registry [`Phase`] in kJ/mol, the energy to disperse one mole of the phase into
/// free gaseous atoms, DERIVED exactly by Hess's law: `E_coh = sum(atomization_enthalpy_i * count_i) - dH_f`,
/// the atomization enthalpies read from the periodic table (measured `[M]`) and the enthalpy of formation from
/// the phase registry (measured `[M]`). Positive for a bound phase (energy is required to disperse it). Returns
/// `None` if the formula names an element absent from the table or an element carries no atomization enthalpy
/// yet (the extensible registry: an unpopulated element is a data gap the floor grows to close, not a zero).
pub fn phase_cohesive_energy(phase: &Phase, table: &PeriodicTable) -> Option<Fixed> {
    let mut atomization_sum = Fixed::ZERO;
    for (sym, count) in &phase.composition {
        let el = table.element(sym)?;
        let atom = el.atomization_enthalpy?;
        atomization_sum += atom.checked_mul(Fixed::from_int(*count as i32))?;
    }
    // E_coh = (energy to atomize the constituent elements) - (energy the phase released forming from them).
    // dH_f is negative for a stable phase, so subtracting it ADDS its magnitude: a more stable phase (more
    // negative formation enthalpy) is harder to pull apart, a larger cohesive energy.
    Some(atomization_sum - phase.enthalpy_formation)
}

/// The per-phase QUICK-SCREEN STIFFNESS SCALE in GPa, the estimator `M ~ E_coh / V`: the cohesive-energy
/// density, in kJ/cm^3 which is GPa directly (`1 kJ/cm^3 = 1 GPa`). A bonding-agnostic order-of-magnitude
/// stiffness, the METALLIC / invented-element tier `[E]`, not a cleanly separated bulk or shear modulus and not
/// the principled route for the ionic-covalent oxide phases (whose bulk modulus rides the Shannon radius, see
/// the module doc). Returns `None` if the cohesive energy is unavailable (a data gap) or the molar volume is
/// non-positive. Its scatter is the reserved band on [`assemblage_elastic_modulus`].
pub fn phase_elastic_modulus(phase: &Phase, table: &PeriodicTable) -> Option<Fixed> {
    let e_coh = phase_cohesive_energy(phase, table)?;
    if phase.molar_volume <= Fixed::ZERO {
        return None;
    }
    e_coh.checked_div(phase.molar_volume)
}

/// The AGGREGATE quick-screen stiffness scale of a stable [`Assemblage`] in GPa, the Voigt-Reuss-Hill estimate
/// over its phases' per-phase scales weighted by their VOLUME FRACTIONS: the Voigt mean (arithmetic) is the
/// rigorous upper bound, the Reuss mean (harmonic) the rigorous lower bound, the Hill average of the two the
/// first-pass estimate, and their half-gap the derived aggregation band. Emitted as an ESTIMATOR `[E]`: the
/// aggregation step is an exact `[D]` relation, but the per-phase scales it aggregates are estimator values, so
/// the composite is only as trustworthy as the estimator it is built on, and the honest tag is the weaker one.
/// This is the metallic / quick-screen tier, superseded for the ionic-covalent oxide phases by the principled
/// radius-curvature bulk modulus (the corrected carve); it does not by itself retire `mat.elastic_modulus`.
///
/// Returns `None` if a phase is missing from the registry, an element from the table, or an atomization
/// enthalpy is unpopulated (a data gap), or the assemblage has no volume. Byte-neutral: no consumer reads this
/// yet, so the pins hold.
///
/// The emitted `band` is the Voigt-Reuss half-gap, a `[D]` bound that is zero for a single-phase assemblage.
/// RESERVED, surfaced not baked: the per-phase ESTIMATOR SCATTER band, the factor by which real moduli deviate
/// from the cohesive-energy-density scale across bonding classes (covalent, ionic, metallic). Its basis is the
/// empirical spread of `M / (E_coh / V)` in the estimator-calibration literature, a fraction the owner sets and
/// validates against measured single-crystal moduli, at which point the emitted band widens to the larger of
/// the derived VRH gap and the estimator scatter. Until set it is not fabricated: the derivation emits the
/// derived VRH gap alone and the estimator-scatter reserve is documented here, not invented into the value.
pub fn assemblage_elastic_modulus(
    assemblage: &Assemblage,
    registry: &PhaseRegistry,
    table: &PeriodicTable,
) -> Option<PropertyEstimate> {
    // First pass: the total volume, so the per-phase volume fractions are exact.
    let mut total_volume = Fixed::ZERO;
    for (name, amt) in &assemblage.phases {
        let phase = registry.phase(name)?;
        total_volume += amt.checked_mul(phase.molar_volume)?;
    }
    if total_volume <= Fixed::ZERO {
        return None;
    }
    // Second pass: the Voigt (arithmetic) and Reuss (harmonic) means, both volume-fraction weighted.
    let mut voigt = Fixed::ZERO;
    let mut reuss_reciprocal = Fixed::ZERO;
    for (name, amt) in &assemblage.phases {
        let phase = registry.phase(name)?;
        let modulus = phase_elastic_modulus(phase, table)?;
        if modulus <= Fixed::ZERO {
            return None;
        }
        let phase_volume = amt.checked_mul(phase.molar_volume)?;
        let fraction = phase_volume.checked_div(total_volume)?;
        voigt += fraction.checked_mul(modulus)?;
        reuss_reciprocal += fraction.checked_div(modulus)?;
    }
    if reuss_reciprocal <= Fixed::ZERO {
        return None;
    }
    let reuss = Fixed::ONE.checked_div(reuss_reciprocal)?;
    // Hill: the mean of the two bounds. Band: their half-gap (Voigt >= Reuss always, so non-negative).
    let two = Fixed::from_int(2);
    let hill = (voigt + reuss).checked_div(two)?;
    let band = (voigt - reuss).checked_div(two)?;
    Some(PropertyEstimate {
        value: hill,
        band,
        provenance: Provenance::Estimator,
    })
}

// ----- The principled aggregate: a rock's moduli DERIVED from its census of cited MINERAL moduli -----
//
// This is the doctrine route (RUNBOOK section 14), distinct from the quick-screen scale above. The floor is a
// cited MINERAL modulus (measured in ignorance of any rock, so it cannot fit an outcome, `crate::mineral_moduli`);
// the DERIVATION is the Voigt-Reuss aggregation over the world's own mineral census; the OUTPUT is the rock's
// moduli, which a handbook basalt or granite then referees from OUTSIDE. Nothing cited ever substitutes for the
// aggregation step: a rock modulus is derived here or it is refused, never read from a table.

/// A rock's DERIVED bulk and shear moduli, each a [`PropertyEstimate`] carrying the derived value, its combined
/// texture-and-measurement band, and the `Derived` provenance (the Voigt-Reuss aggregation is an exact `[D]`
/// step over MEASURED `[M]` mineral inputs, so the honest composite tag is `[D]`, unlike the quick-screen
/// [`assemblage_elastic_modulus`] whose estimator inputs make it `[E]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateModuli {
    /// The aggregate bulk modulus `K` (GPa), derived.
    pub bulk: PropertyEstimate,
    /// The aggregate shear modulus `G` (GPa), derived.
    pub shear: PropertyEstimate,
}

/// The Voigt-Reuss-Hill aggregate of one property over a census, given the four accumulated sums: the
/// volume-fraction-weighted Voigt (arithmetic) and Reuss (harmonic) means at the phases' CENTRAL moduli, and the
/// same two at the phases' outer measurement edges (Voigt over the stiff edges `M_i + band_i`, Reuss over the
/// soft edges `M_i - band_i`). The value is the conventional Hill best estimate (the midpoint of the central
/// Voigt-Reuss gap); the band is the symmetric half-width that covers the OUTER interval `[reuss_lo, voigt_hi]`,
/// so it folds the texture uncertainty (Voigt versus Reuss, the rock's unknown grain geometry) and the input
/// measurement uncertainty (each mineral's own band) into one rigorous interval. When every measurement band is
/// zero this reduces exactly to the classic Hill value with the half-gap band.
fn vrh_banded(
    voigt_center: Fixed,
    reuss_recip_center: Fixed,
    voigt_hi: Fixed,
    reuss_recip_lo: Fixed,
) -> Option<PropertyEstimate> {
    if reuss_recip_center <= Fixed::ZERO || reuss_recip_lo <= Fixed::ZERO {
        return None;
    }
    let two = Fixed::from_int(2);
    let reuss_center = Fixed::ONE.checked_div(reuss_recip_center)?;
    let reuss_lo = Fixed::ONE.checked_div(reuss_recip_lo)?;
    // The conventional Hill best estimate: the midpoint of the central Voigt-Reuss bounds.
    let hill = (voigt_center + reuss_center).checked_div(two)?;
    // The band covers the outer texture-times-measurement interval [reuss_lo, voigt_hi]. Voigt_hi >= voigt_center
    // >= hill and reuss_lo <= reuss_center <= hill, so both gaps are non-negative and the larger is the covering
    // half-width.
    let up = voigt_hi - hill;
    let down = hill - reuss_lo;
    let band = if up >= down { up } else { down };
    Some(PropertyEstimate {
        value: hill,
        band,
        provenance: Provenance::Derived,
    })
}

/// A rock's bulk and shear moduli DERIVED by Voigt-Reuss-Hill aggregation over its stable mineral
/// [`Assemblage`], reading each phase's CITED measured moduli from the [`MineralModuli`] floor and each phase's
/// VOLUME FRACTION from its registry molar volume. This is the doctrine route: a world's rock stiffness falls
/// out of the moduli of the minerals the world's own petrology drew, never a per-rock-type table.
///
/// The aggregation weights each phase by its volume fraction (the elastically correct weighting), takes the
/// Voigt arithmetic mean (the rigorous upper bound, iso-strain) and the Reuss harmonic mean (the rigorous lower
/// bound, iso-stress), and reports the Hill midpoint with a band that covers the texture gap widened by the
/// input measurement bands (see [`vrh_banded`]).
///
/// TERMS DROPPED, BY NAME (each a chord this aggregate does not carry, surfaced rather than silently absorbed):
///   - THE TEXTURE TERM. The true aggregate depends on grain geometry between the Voigt and Reuss bounds; the
///     rock's actual texture is unknown, so the Hill midpoint is the maximum-ignorance estimate and the V-R gap
///     is carried AS the band. A measured texture (a foliation, a shape-preferred orientation) would tighten
///     this toward one bound; that tightening is a future term, not dropped silently but named here.
///   - THE CRACK AND POROSITY TERM. The mineral moduli are the intact crack-free frame (the row's declared
///     `frame`); a real rock's cracks and pores lower its moduli below this intact aggregate. That softening is
///     a separate poroelastic term keyed on a porosity field, dropped here and named, not folded into the band.
///   - THE PRESSURE-TEMPERATURE CHORD. The mineral moduli are measured at their row's `(P, T)` (the seed rows
///     are ambient); a rock at depth sits at a different chord, where `K` and `G` are stiffer (pressure) and
///     softer (temperature). That `(P, T)` correction rides the moduli's own derivatives (a future column on the
///     mineral floor), dropped here and named.
///
/// Returns `None` (a banded REFUSAL, never a silent phase drop) if any census phase has no cited measured row,
/// is missing from the registry, or the assemblage has no volume. An unmeasured phase is refused rather than
/// dropped because dropping it would bias the aggregate toward the measured remainder; the honest output is a
/// refusal that says the census is not yet fully grounded. (The future tightening: an unmeasured but clean ionic
/// phase could contribute a bulk modulus through the lattice-curvature carve rung, `crate::lattice_modulus`,
/// rather than refusing the whole aggregate; the shear modulus has no floor route yet, so it would still refuse.)
///
/// Byte-neutral: no consumer reads this yet, so the pins hold.
pub fn assemblage_bulk_shear_moduli(
    assemblage: &Assemblage,
    moduli: &MineralModuli,
    registry: &PhaseRegistry,
) -> Option<AggregateModuli> {
    // First pass: the total volume, so the per-phase volume fractions are exact.
    let mut total_volume = Fixed::ZERO;
    for (name, amt) in &assemblage.phases {
        let phase = registry.phase(name)?;
        total_volume += amt.checked_mul(phase.molar_volume)?;
    }
    if total_volume <= Fixed::ZERO {
        return None;
    }
    // Second pass: the Voigt and Reuss accumulators for K and G, at the central moduli and at the outer
    // measurement edges. The loader guarantees each soft edge (modulus minus band) is strictly positive, so the
    // harmonic Reuss reciprocals never divide through zero.
    let mut k_voigt_c = Fixed::ZERO;
    let mut k_reuss_recip_c = Fixed::ZERO;
    let mut k_voigt_hi = Fixed::ZERO;
    let mut k_reuss_recip_lo = Fixed::ZERO;
    let mut g_voigt_c = Fixed::ZERO;
    let mut g_reuss_recip_c = Fixed::ZERO;
    let mut g_voigt_hi = Fixed::ZERO;
    let mut g_reuss_recip_lo = Fixed::ZERO;
    for (name, amt) in &assemblage.phases {
        let phase = registry.phase(name)?;
        let row = moduli.row(name)?; // banded refusal: an unmeasured census phase refuses the whole aggregate.
        let phase_volume = amt.checked_mul(phase.molar_volume)?;
        let fraction = phase_volume.checked_div(total_volume)?;
        let k_soft = row.bulk_gpa - row.bulk_band_gpa;
        let g_soft = row.shear_gpa - row.shear_band_gpa;
        if k_soft <= Fixed::ZERO || g_soft <= Fixed::ZERO {
            return None;
        }
        k_voigt_c += fraction.checked_mul(row.bulk_gpa)?;
        k_reuss_recip_c += fraction.checked_div(row.bulk_gpa)?;
        k_voigt_hi += fraction.checked_mul(row.bulk_gpa + row.bulk_band_gpa)?;
        k_reuss_recip_lo += fraction.checked_div(k_soft)?;
        g_voigt_c += fraction.checked_mul(row.shear_gpa)?;
        g_reuss_recip_c += fraction.checked_div(row.shear_gpa)?;
        g_voigt_hi += fraction.checked_mul(row.shear_gpa + row.shear_band_gpa)?;
        g_reuss_recip_lo += fraction.checked_div(g_soft)?;
    }
    let bulk = vrh_banded(k_voigt_c, k_reuss_recip_c, k_voigt_hi, k_reuss_recip_lo)?;
    let shear = vrh_banded(g_voigt_c, g_reuss_recip_c, g_voigt_hi, g_reuss_recip_lo)?;
    Some(AggregateModuli { bulk, shear })
}

/// The four isotropic elastic constants of a solid, the full closure from a bulk and shear modulus: an isotropic
/// material has two independent elastic constants, so `K` and `G` fix `E` (Young's) and `nu` (Poisson's) exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsotropicClosure {
    /// The bulk modulus `K` (GPa), the input.
    pub bulk: Fixed,
    /// The shear modulus `G` (GPa), the input.
    pub shear: Fixed,
    /// Young's modulus `E` (GPa), derived: `E = 9 K G / (3 K + G)`.
    pub young: Fixed,
    /// Poisson's ratio `nu` (dimensionless), derived: `nu = (3 K - 2 G) / (2 (3 K + G))`.
    pub poisson: Fixed,
}

/// The fixed-point consistency budget for the two Poisson-ratio routes. The direct route `nu = (3K - 2G) /
/// (2(3K + G))` and the through-`E` twin `nu = (3K - E) / (6K)` are one algebraic identity, so their difference
/// is pure Q32.32 rounding across two division orders (a few ULP), never a physical disagreement. A divergence
/// beyond this budget signals an arithmetic defect and refuses the closure. This is a REPRESENTATION tolerance
/// (the fixed-point grid's rounding), not a world value: `1e-6`, thousands of ULP above the real rounding and far
/// below any bug's signature.
fn isotropic_twin_tolerance() -> Fixed {
    Fixed::from_ratio(1, 1_000_000)
}

/// The isotropic elastic closure from a bulk and shear modulus: `E` and `nu` derived by the exact isotropic
/// relations, with a two-route consistency twin on `nu` and a thermodynamic guard. Returns `None` if either
/// input modulus is non-positive, if `nu` falls outside the thermodynamic range `(-1, 1/2)` (a corrupted input),
/// or if the two `nu` routes diverge beyond the fixed-point budget (an arithmetic defect). The caller passes the
/// central aggregate values (a [`PropertyEstimate`]'s `value`); propagating the `K` and `G` bands through this
/// nonlinear closure into `E` and `nu` bands is a documented follow-on, the bands being carried on
/// [`AggregateModuli`] until then.
pub fn assemblage_isotropic_closure(bulk_gpa: Fixed, shear_gpa: Fixed) -> Option<IsotropicClosure> {
    if bulk_gpa <= Fixed::ZERO || shear_gpa <= Fixed::ZERO {
        return None;
    }
    let two = Fixed::from_int(2);
    let three = Fixed::from_int(3);
    let six = Fixed::from_int(6);
    let nine = Fixed::from_int(9);
    let three_k = three.checked_mul(bulk_gpa)?;
    let three_k_plus_g = three_k + shear_gpa;
    if three_k_plus_g <= Fixed::ZERO {
        return None;
    }
    // Young's modulus E = 9 K G / (3K + G).
    let young = nine
        .checked_mul(bulk_gpa)?
        .checked_mul(shear_gpa)?
        .checked_div(three_k_plus_g)?;
    // Poisson's ratio, direct route: nu = (3K - 2G) / (2 (3K + G)).
    let two_g = two.checked_mul(shear_gpa)?;
    let poisson = (three_k - two_g).checked_div(two.checked_mul(three_k_plus_g)?)?;
    // Poisson's ratio, through-E twin: nu = (3K - E) / (6K). Algebraically identical; a fixed-point cross-check.
    let poisson_twin = (three_k - young).checked_div(six.checked_mul(bulk_gpa)?)?;
    // Thermodynamic guard: an isotropic solid has nu in (-1, 1/2). For K, G > 0 this holds by construction; the
    // guard catches a corrupted input rather than shaping a real one.
    let half = Fixed::from_ratio(1, 2);
    let neg_one = Fixed::from_int(-1);
    if poisson <= neg_one || poisson >= half {
        return None;
    }
    // Fixed-point consistency: the two routes are one identity, so a divergence is a defect, not physics.
    let diff = if poisson >= poisson_twin {
        poisson - poisson_twin
    } else {
        poisson_twin - poisson
    };
    if diff > isotropic_twin_tolerance() {
        return None;
    }
    Some(IsotropicClosure {
        bulk: bulk_gpa,
        shear: shear_gpa,
        young,
        poisson,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::petrology::stable_assemblage;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the embedded periodic table loads")
    }

    fn registry() -> PhaseRegistry {
        PhaseRegistry::standard().expect("the embedded phase registry loads")
    }

    #[test]
    fn the_gas_elements_atomization_rows_are_per_atom_from_their_diatomic_references() {
        // THE CHECK THAT GATES TRUST IN EVERY OXIDE THIS ORACLE SERVES, and it is cheap enough that running it
        // once by hand would have been the waste, not the test.
        //
        // The atomization column stores the standard enthalpy of formation of the MONATOMIC GAS, which is by
        // definition measured from each element's OWN declared reference state. That single definition carries
        // TWO conventions at once, and they are not interchangeable:
        //   - Mg, Si, Fe, Al, Ca, Na, K, Ti: the reference is the SOLID, so the row is a SUBLIMATION enthalpy.
        //   - O, N, H: the reference is the DIATOMIC GAS, so the row is HALF A DISSOCIATION enthalpy.
        //
        // A consumer that read the gas rows as sublimation enthalpies, or a fetch that asked for "sublimation
        // enthalpies" across the board, would take the WRONG QUANTITY for exactly the elements oxides are made
        // of, and every silicate E_coh this oracle derives would be wrong by a term with no symptom.
        //
        // THE PROOF IS ARITHMETIC AND INDEPENDENT: if the gas rows are per-atom, DOUBLING each must recover its
        // molecule's bond dissociation enthalpy. Those totals come from outside this column, so this is a twin
        // by a different route rather than the column checked against itself.
        let t = table();
        let cases = [
            // (element, the X2 bond dissociation enthalpy, kJ/mol)
            ("H", 435.996_f64),
            ("N", 945.360_f64),
            ("O", 498.360_f64),
        ];
        for (sym, dissociation) in cases {
            let per_atom = t
                .element(sym)
                .expect("the gas element is in the table")
                .atomization_enthalpy
                .expect("it carries an atomization enthalpy")
                .to_f64_lossy();
            let doubled = 2.0 * per_atom;
            assert!(
                (doubled - dissociation).abs() < 0.05,
                "{sym}: the row must be PER-ATOM (half the dissociation), so 2 x {per_atom} should be \
                 {dissociation}, got {doubled}. A mismatch means the gas rows are being read against the wrong \
                 reference state, and every oxide this oracle derives is wrong."
            );
        }
    }

    // A test-only float readout, exactly as the petrology and periodic tests use `to_f64_lossy`: no float
    // touches the canonical integer path; this only compares a derived Fixed against a hand-computed decimal.
    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn quartz_cohesive_energy_matches_the_hess_law_hand_calc() {
        let reg = registry();
        let t = table();
        let quartz = reg.phase("quartz").expect("quartz is in the seed registry");
        // E_coh(SiO2) = atomization(Si) + 2*atomization(O) - dH_f(quartz)
        //             = 450.0 + 2*249.18 - (-910.70) = 1859.06 kJ/mol.
        let e_coh = phase_cohesive_energy(quartz, &t).expect("quartz cohesive energy derives");
        assert!(
            close(e_coh, 1859.06, 1e-2),
            "quartz E_coh should be 1859.06 kJ/mol, got {}",
            e_coh.to_f64_lossy()
        );
    }

    #[test]
    fn periclase_cohesive_energy_matches_the_hand_calc() {
        let reg = registry();
        let t = table();
        let periclase = reg
            .phase("periclase")
            .expect("periclase is in the registry");
        // E_coh(MgO) = atomization(Mg) + atomization(O) - dH_f = 147.1 + 249.18 - (-601.60) = 997.88 kJ/mol.
        let e_coh =
            phase_cohesive_energy(periclase, &t).expect("periclase cohesive energy derives");
        assert!(
            close(e_coh, 997.88, 1e-2),
            "periclase E_coh should be 997.88 kJ/mol, got {}",
            e_coh.to_f64_lossy()
        );
    }

    #[test]
    fn quartz_modulus_is_the_cohesive_energy_density_in_gpa() {
        let reg = registry();
        let t = table();
        let quartz = reg.phase("quartz").expect("quartz is in the registry");
        // M = E_coh / V = 1859.06 / 22.688 = 81.94 GPa (kJ/cm^3 = GPa directly).
        let m = phase_elastic_modulus(quartz, &t).expect("quartz modulus estimates");
        assert!(
            close(m, 81.94, 1e-1),
            "quartz modulus should be ~81.94 GPa, got {}",
            m.to_f64_lossy()
        );
    }

    #[test]
    fn a_single_phase_assemblage_has_no_aggregation_band() {
        // A pure-quartz composition: 1 Si, 2 O, reaching quartz alone. The VRH gap collapses (Voigt = Reuss =
        // the phase modulus), so the derived band is zero and the value equals the phase modulus.
        let reg = registry();
        let t = table();
        let composition = [
            ("Si".to_string(), Fixed::from_int(1)),
            ("O".to_string(), Fixed::from_int(2)),
        ];
        let asm = stable_assemblage(&composition, Fixed::from_int(300), Fixed::from_int(1), &reg)
            .expect("the pure-silica composition reaches an assemblage");
        let est = assemblage_elastic_modulus(&asm, &reg, &t).expect("the modulus emits");
        assert!(
            close(est.band, 0.0, 1e-6),
            "a single-phase assemblage has a zero VRH band, got {}",
            est.band.to_f64_lossy()
        );
        assert!(
            close(est.value, 81.94, 1e-1),
            "the single-phase value equals the phase modulus, got {}",
            est.value.to_f64_lossy()
        );
        assert_eq!(est.provenance, Provenance::Estimator);
    }

    #[test]
    fn a_multi_phase_assemblage_has_value_between_the_voigt_reuss_bounds() {
        // A forsterite-plus-quartz-forming composition (Mg, Si, O) reaches a two-phase assemblage; the Hill
        // value must lie strictly inside the Voigt-Reuss bounds, so the derived band is positive and the value
        // is bracketed by value +/- band.
        let reg = registry();
        let t = table();
        // Forsterite Mg2SiO4 plus quartz SiO2 balances exactly to Mg2 Si2 O6, a genuine two-phase mix.
        let composition = [
            ("Mg".to_string(), Fixed::from_int(2)),
            ("Si".to_string(), Fixed::from_int(2)),
            ("O".to_string(), Fixed::from_int(6)),
        ];
        let asm = stable_assemblage(&composition, Fixed::from_int(300), Fixed::from_int(1), &reg)
            .expect("the Mg-Si-O composition reaches an assemblage");
        if asm.phases.len() < 2 {
            // If the seed registry balances this to a single phase, the two-phase invariant does not apply;
            // the single-phase band case is covered by its own test.
            return;
        }
        let est = assemblage_elastic_modulus(&asm, &reg, &t).expect("the modulus emits");
        assert!(
            est.band > Fixed::ZERO,
            "a genuine two-phase assemblage has a positive VRH band, got {}",
            est.band.to_f64_lossy()
        );
        // The Hill value sits at the midpoint of the bounds, so value - band = Reuss <= value <= Voigt.
        assert!(
            est.value > est.band,
            "the Reuss bound (value - band) is positive"
        );
    }

    #[test]
    fn an_unpopulated_element_yields_no_cohesive_energy() {
        // A phase whose formula names an element without an atomization enthalpy returns None (the data gap is
        // surfaced, never silently zeroed). Construct a phase over an element with no atomization value: the
        // seed populates the common rock-formers, so pick one deliberately absent (helium carries none).
        let t = table();
        let phantom = Phase {
            name: "phantom".to_string(),
            formula: "He".to_string(),
            composition: vec![("He".to_string(), 1)],
            enthalpy_formation: Fixed::ZERO,
            enthalpy_decimal: "0".to_string(),
            standard_entropy: Fixed::ZERO,
            entropy_decimal: "0".to_string(),
            molar_volume: Fixed::from_int(10),
            volume_decimal: "10".to_string(),
            clapeyron_slope: None,
            clapeyron_decimal: None,
            prototype: None,
            source: "test".to_string(),
        };
        assert!(
            phase_cohesive_energy(&phantom, &t).is_none(),
            "an element with no atomization enthalpy surfaces the data gap as None"
        );
    }

    // ----- The principled aggregate (cited mineral moduli -> derived rock moduli) -----

    /// Synthetic mineral moduli for the machinery tests: ROUND numbers over real registry phase names so the
    /// Voigt-Reuss math is hand-verifiable. These are NOT a citation load (the source string says so); the real
    /// cited values live in `data/mineral_moduli.toml`, and these prove only that the aggregator computes the
    /// bounds and the closure correctly given known inputs.
    fn synthetic_moduli() -> MineralModuli {
        let toml = r#"
[[mineral]]
name = "periclase"
bulk_modulus_gpa = "200"
bulk_band_gpa = "2"
shear_modulus_gpa = "120"
shear_band_gpa = "1"
source = "SYNTHETIC test fixture, round numbers, not a citation"
[[mineral]]
name = "quartz"
bulk_modulus_gpa = "100"
shear_modulus_gpa = "60"
source = "SYNTHETIC test fixture, round numbers, not a citation"
"#;
        MineralModuli::from_toml_str(toml).expect("the synthetic fixture loads")
    }

    #[test]
    fn the_isotropic_closure_is_the_textbook_relation() {
        // K = 100, G = 75 gives, by the exact isotropic relations, nu = (300 - 150) / (2 * 375) = 0.2 and
        // E = 9 * 100 * 75 / 375 = 180. Both routes for nu must agree and the guard must pass.
        let closure = assemblage_isotropic_closure(Fixed::from_int(100), Fixed::from_int(75))
            .expect("a real K and G close to E and nu");
        let tol = Fixed::from_ratio(1, 100_000);
        assert!(
            (closure.poisson - Fixed::from_ratio(2, 10)).abs() < tol,
            "Poisson's ratio is 0.2, got {}",
            closure.poisson.to_f64_lossy()
        );
        assert!(
            (closure.young - Fixed::from_int(180)).abs() < tol,
            "Young's modulus is 180 GPa, got {}",
            closure.young.to_f64_lossy()
        );
        // A non-physical input (zero shear) is refused, not closed.
        assert!(assemblage_isotropic_closure(Fixed::from_int(100), Fixed::ZERO).is_none());
    }

    #[test]
    fn a_single_phase_rock_takes_the_minerals_own_moduli_and_band() {
        // A rock of one mineral has that mineral's moduli exactly, and the aggregate band is the mineral's own
        // measurement band (there is no texture gap with one phase).
        let reg = registry();
        let moduli = synthetic_moduli();
        let asm = Assemblage {
            phases: vec![("periclase".to_string(), Fixed::from_int(1))],
            total_gibbs: Fixed::ZERO,
            truncated: false,
        };
        let agg = assemblage_bulk_shear_moduli(&asm, &moduli, &reg)
            .expect("a measured single phase aggregates");
        // The aggregation divides by the total volume and takes a harmonic reciprocal, so a single phase returns
        // its own moduli to within fixed-point rounding (a few parts per million), not bit-exactly.
        let tol = Fixed::from_ratio(1, 1000);
        assert!(
            (agg.bulk.value - Fixed::from_int(200)).abs() < tol,
            "bulk is periclase's 200 GPa, got {}",
            agg.bulk.value.to_f64_lossy()
        );
        assert!(
            (agg.bulk.band - Fixed::from_int(2)).abs() < tol,
            "bulk band is periclase's 2 GPa, got {}",
            agg.bulk.band.to_f64_lossy()
        );
        assert!(
            (agg.shear.value - Fixed::from_int(120)).abs() < tol,
            "shear is periclase's 120 GPa, got {}",
            agg.shear.value.to_f64_lossy()
        );
        assert!(
            (agg.shear.band - Fixed::from_int(1)).abs() < tol,
            "shear band is periclase's 1 GPa, got {}",
            agg.shear.band.to_f64_lossy()
        );
        assert_eq!(
            agg.bulk.provenance,
            Provenance::Derived,
            "the aggregate is derived, not estimator"
        );
    }

    #[test]
    fn a_two_phase_rock_lands_between_its_minerals_and_is_volume_weighted() {
        // Periclase (K 200) plus quartz (K 100) at one mole each: quartz's larger molar volume (22.688 vs
        // 11.248 cm^3/mol) gives it the larger VOLUME fraction, so the aggregate is pulled below the equal-weight
        // midpoint of 150 toward quartz's 100, and it stays strictly inside the [100, 200] bracket with a
        // positive texture band.
        let reg = registry();
        let moduli = synthetic_moduli();
        let asm = Assemblage {
            phases: vec![
                ("periclase".to_string(), Fixed::from_int(1)),
                ("quartz".to_string(), Fixed::from_int(1)),
            ],
            total_gibbs: Fixed::ZERO,
            truncated: false,
        };
        let agg = assemblage_bulk_shear_moduli(&asm, &moduli, &reg)
            .expect("a two-phase measured rock aggregates");
        assert!(
            agg.bulk.value > Fixed::from_int(100) && agg.bulk.value < Fixed::from_int(200),
            "the aggregate bulk is bracketed by its minerals, got {}",
            agg.bulk.value.to_f64_lossy()
        );
        assert!(
            agg.bulk.value < Fixed::from_int(150),
            "quartz has the larger volume fraction, so the aggregate is pulled below the equal-weight 150, got {}",
            agg.bulk.value.to_f64_lossy()
        );
        assert!(
            agg.bulk.band > Fixed::ZERO,
            "a two-phase rock carries a positive texture band"
        );
        // The whole closure runs end to end on the derived aggregate.
        let closure = assemblage_isotropic_closure(agg.bulk.value, agg.shear.value)
            .expect("the derived rock moduli close to E and nu");
        assert!(closure.poisson > Fixed::ZERO && closure.poisson < Fixed::from_ratio(1, 2));
    }

    #[test]
    fn an_unmeasured_census_phase_refuses_the_aggregate() {
        // Forsterite is in the registry but NOT in the synthetic moduli table, an unmeasured census member. The
        // aggregator refuses (None) rather than silently dropping it and biasing the result toward the measured
        // remainder.
        let reg = registry();
        let moduli = synthetic_moduli();
        let asm = Assemblage {
            phases: vec![
                ("periclase".to_string(), Fixed::from_int(1)),
                ("forsterite".to_string(), Fixed::from_int(1)),
            ],
            total_gibbs: Fixed::ZERO,
            truncated: false,
        };
        assert!(
            assemblage_bulk_shear_moduli(&asm, &moduli, &reg).is_none(),
            "an unmeasured census phase is a banded refusal, never a silent drop"
        );
    }
}
