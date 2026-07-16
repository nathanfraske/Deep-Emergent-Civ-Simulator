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
}
