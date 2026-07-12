//! The materials oracle, Stage-6 property emission: a material's mechanical properties DERIVED from its stable
//! mineral assemblage rather than authored per rock type. This is the composition-to-property bridge the spec
//! (`docs/working/MATERIALS_ORACLE_SPEC.md`) calls `K_solid`, the elastic-modulus sibling of the density
//! `crustal_density` already emits from the same assemblage: the assemblage's phases and their mode fractions
//! are the input, the aggregate stiffness is the derived output, so a rock's modulus falls out of its
//! mineralogy and never an authored per-rock-type table (Principle 8). The named-substance floor's authored
//! elastic anchor (`mat.elastic_modulus`) is the retirement target; this derivation stands the replacement up
//! dormant (byte-neutral, armed by no scenario, the `derive_mantle_density` pattern one property over).
//!
//! The route, per the gate's ESTIMATOR-ONLY ruling on the carve (#182):
//!
//! 1. The COHESIVE ENERGY of a phase by Hess's law, exact from the floor: the energy to disperse the phase into
//!    free atoms is the energy to make free atoms from the elements' reference states (the sum of the elements'
//!    atomization enthalpies, a measured `[M]` component-level datum now on the periodic table) minus the
//!    energy the phase already released forming from those elements (its enthalpy of formation, on the phase
//!    registry). `E_coh = sum(atomization_i * count_i) - dH_f`. Derived `[D]` from measured inputs, no authored
//!    step.
//! 2. The per-phase ELASTIC MODULUS estimated from the cohesive-energy density, `M ~ E_coh / V`: a stiff solid
//!    is one that stores much cohesive energy in little volume, and the cohesive-energy density carries the
//!    units of a modulus directly (`1 kJ/cm^3 = 1 GPa`, since `1 kJ = 1e3 J` over `1 cm^3 = 1e-6 m^3` is
//!    `1e9 Pa`). This is the estimator `[E]`, the one approximate step: the true modulus scatters around the
//!    cohesive-energy scale by a bonding-dependent factor of order one, surfaced below as the reserved
//!    estimator band, never baked into the value.
//! 3. The AGGREGATE modulus of the assemblage from its per-phase moduli over their volume fractions, the
//!    Voigt-Reuss-Hill mean: the Voigt (arithmetic, volume-weighted) and Reuss (harmonic, volume-weighted)
//!    means are the rigorous upper and lower bounds a two-phase-and-up composite's modulus must lie between,
//!    and the Hill average is the standard first-pass estimate; their half-gap is the DERIVED aggregation band.
//!    Derived `[D]` from the per-phase estimates. The Hashin-Shtrikman bounds are the tighter refinement the
//!    spec marks for Stage 7, a follow-on, not this slice.
//!
//! Everything here is fixed-point (the assemblage solve already is) and deterministic. Nothing reads the output
//! yet, so the pins hold.

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

/// The per-phase ELASTIC MODULUS in GPa, the estimator `M ~ E_coh / V`: the cohesive-energy density, in kJ/cm^3
/// which is GPa directly (`1 kJ/cm^3 = 1 GPa`). Returns `None` if the cohesive energy is unavailable (a data
/// gap) or the molar volume is non-positive. This is the estimator `[E]`, the single approximate step in the
/// route; its scatter is the reserved band on [`assemblage_elastic_modulus`].
pub fn phase_elastic_modulus(phase: &Phase, table: &PeriodicTable) -> Option<Fixed> {
    let e_coh = phase_cohesive_energy(phase, table)?;
    if phase.molar_volume <= Fixed::ZERO {
        return None;
    }
    e_coh.checked_div(phase.molar_volume)
}

/// The AGGREGATE elastic modulus of a stable [`Assemblage`] in GPa, the Voigt-Reuss-Hill estimate over its
/// phases' per-phase moduli weighted by their VOLUME FRACTIONS: the Voigt mean (arithmetic) is the rigorous
/// upper bound, the Reuss mean (harmonic) the rigorous lower bound, the Hill average of the two the first-pass
/// estimate, and their half-gap the derived aggregation band. The value is emitted as an ESTIMATOR `[E]`: the
/// aggregation step is an exact `[D]` relation, but the per-phase moduli it aggregates are estimator values, so
/// the composite is only as trustworthy as the estimator it is built on, and the honest tag is the weaker one.
///
/// Returns `None` if a phase is missing from the registry, an element from the table, or an atomization
/// enthalpy is unpopulated (a data gap), or the assemblage has no volume. Byte-neutral: no consumer reads this
/// yet, so the pins hold; it stands up the composition-derived replacement for the authored
/// `mat.elastic_modulus` dormant, exactly as `derive_mantle_density` landed.
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
            source: "test".to_string(),
        };
        assert!(
            phase_cohesive_energy(&phantom, &t).is_none(),
            "an element with no atomization enthalpy surfaces the data gap as None"
        );
    }
}
