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

//! The petrology kernel (genesis-forward Stage 3, the surface lane): the stable mineral assemblage and the
//! pressure-temperature-dependent density a bulk composition reaches at a given pressure and temperature,
//! DERIVED by minimizing the total Gibbs free energy over the candidate-phase registry
//! ([`crate::petrology_data`]) rather than authored as a fixed mineral sequence. The assemblage EMERGES from
//! the thermodynamics and the world's own composition (Principle 8): a Terran bulk-silicate composition lands
//! olivine and pyroxene, an alien chemistry lands its own phases, because the mechanism reads the data-defined
//! registry and the per-world element budget, never a hardcoded CIPW-style allocation order (which would
//! author a Terran igneous sequence, the value-line violation the design forbids). The mechanism is fixed
//! Rust; the phase membership, their thermodynamics, and the bulk composition are data (Principle 11).
//!
//! This module carries the ATOMIC BUILDING BLOCK, the per-phase Gibbs free energy at a pressure and
//! temperature; the assemblage-minimization over the element budget is the sibling follow-on that composes
//! these energies. Every value is fixed-point ([`Fixed`]) and deterministic (no float on the canonical path,
//! Principle 3). No consumer is wired to this kernel yet; it is a pure addition.

use crate::petrology_data::Phase;
use civsim_core::Fixed;

/// The standard-state reference PRESSURE of the thermodynamic dataset, one bar. The enthalpy of formation and
/// the standard entropy in the registry are tabulated at this reference, so the pressure work term is measured
/// from it. A definitional anchor of the cited data (like the boiling-point reference pressure in the
/// Rankine-Kirchhoff law), not a per-world value.
pub const REFERENCE_PRESSURE_BAR: i32 = 1;

/// The standard-state reference TEMPERATURE of the thermodynamic dataset, 298.15 K, the temperature the
/// enthalpy of formation and standard entropy are tabulated at. Carried for the record and the Cp-integral
/// follow-on; the first-pass Gibbs energy below treats the enthalpy and entropy as constant from this
/// reference (the heat-capacity refinement is the flagged follow-on the registry grows to).
pub fn reference_temperature_k() -> Fixed {
    Fixed::from_ratio(29815, 100)
}

/// The apparent Gibbs free energy of formation of a stoichiometric phase at a temperature and pressure, in
/// joules per mole, the quantity a free-energy minimization compares across candidate assemblages. It is
///
/// `G(T, P) = dH_f - T * S + V * (P - P_ref)`,
///
/// the Benson-Helgeson apparent-Gibbs convention: the enthalpy of formation from the elements, less the
/// entropy term (so temperature favours the higher-entropy phase), plus the pressure work from the reference
/// (so pressure favours the lower-volume phase, the seed of the olivine-to-spinel-to-perovskite depth
/// sequence). Because every competing assemblage forms from the same element budget, the element reference
/// cancels in the comparison, so the apparent energy is the right quantity to minimize (the relative energies
/// decide the assemblage, which is why one internally consistent dataset is required).
///
/// UNITS and the representability choice: the enthalpy is kilojoules per mole (as the registry stores it) and
/// converts to joules; the entropy is joules per mole per kelvin; the molar volume is cubic centimetres per
/// mole. The PRESSURE is in BARS, not pascals, deliberately: geological pressures reach the order of a million
/// bars (about a hundred gigapascals) in the deep mantle, which overflows the canonical Q32.32 grid in
/// pascals, whereas a million bars is well inside it. The pressure work `V * (P - P_ref)` is then formed with
/// the exact unit bridge one cubic-centimetre-bar equals one tenth of a joule. Temperature is in kelvin.
///
/// FIRST PASS (the honest limit): the enthalpy, entropy, and molar volume are taken at their standard-state
/// values (constant in temperature and pressure), so the heat-capacity integral that bends `H(T)` and `S(T)`
/// and the compressibility and thermal expansion that bend `V(P, T)` are the flagged follow-on the registry
/// grows the optional coefficient fields for. The leading-order energy is exact for the standard state and
/// carries the correct sign of the temperature and pressure dependence.
pub fn apparent_gibbs_energy(
    enthalpy_formation_kj_per_mol: Fixed,
    standard_entropy_j_per_mol_k: Fixed,
    molar_volume_cm3_per_mol: Fixed,
    temperature_k: Fixed,
    pressure_bar: Fixed,
) -> Fixed {
    // dH_f from kilojoules to joules per mole.
    let enthalpy_j = enthalpy_formation_kj_per_mol * Fixed::from_int(1000);
    // The entropy term T * S, in joules per mole.
    let entropy_term = temperature_k * standard_entropy_j_per_mol_k;
    // The pressure work V * (P - P_ref): cubic-centimetres per mole times bars is joules per mole through the
    // exact bridge 1 cm^3 bar = 0.1 J. Both factors and their product stay inside the Q32.32 range for
    // geological pressures because the pressure is carried in bars.
    let delta_p = pressure_bar - Fixed::from_int(REFERENCE_PRESSURE_BAR);
    let pressure_work = molar_volume_cm3_per_mol * delta_p * Fixed::from_ratio(1, 10);
    enthalpy_j - entropy_term + pressure_work
}

/// The apparent Gibbs free energy of a registry [`Phase`] at a temperature and pressure, in joules per mole,
/// reading the phase's standard-state thermodynamics ([`apparent_gibbs_energy`]). A thin accessor so a caller
/// works from a phase rather than its unpacked fields; the pressure is in bars and the temperature in kelvin.
pub fn phase_gibbs_energy(phase: &Phase, temperature_k: Fixed, pressure_bar: Fixed) -> Fixed {
    apparent_gibbs_energy(
        phase.enthalpy_formation,
        phase.standard_entropy,
        phase.molar_volume,
        temperature_k,
        pressure_bar,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::petrology_data::PhaseRegistry;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_standard_state_gibbs_energy_is_enthalpy_minus_the_entropy_term() {
        // At the reference pressure (1 bar) the pressure work vanishes, so G reduces to dH_f - T * S. For
        // quartz (dH_f = -910.70 kJ/mol, S = 41.46 J/mol/K) at the standard temperature 298.15 K this is
        // -910700 - 298.15 * 41.46 = -923060.9 J/mol, the hand computation the kernel must reproduce.
        let t = reference_temperature_k();
        let p_ref = Fixed::from_int(REFERENCE_PRESSURE_BAR);
        let g = apparent_gibbs_energy(
            Fixed::from_ratio(-91070, 100), // -910.70 kJ/mol
            Fixed::from_ratio(4146, 100),   // 41.46 J/mol/K
            Fixed::from_ratio(22688, 1000), // 22.688 cm^3/mol
            t,
            p_ref,
        );
        assert!(
            close(g, -910700.0 - 298.15 * 41.46, 1.0),
            "standard-state G reduces to dH_f - T*S with no pressure work"
        );
    }

    #[test]
    fn temperature_lowers_the_gibbs_energy_through_the_entropy_term() {
        // Raising temperature at fixed pressure lowers G (the -T*S term), and by more for the higher-entropy
        // phase: the thermodynamic reason a hotter world favours the higher-entropy assemblage.
        let p = Fixed::from_int(1);
        let low_s = Fixed::from_ratio(4146, 100); // quartz-like, S = 41.46
        let high_s = Fixed::from_int(100); // a higher-entropy phase
        let h = Fixed::from_ratio(-91070, 100);
        let v = Fixed::from_ratio(22688, 1000);
        let g_cold_low = apparent_gibbs_energy(h, low_s, v, Fixed::from_int(300), p);
        let g_hot_low = apparent_gibbs_energy(h, low_s, v, Fixed::from_int(1300), p);
        let g_cold_high = apparent_gibbs_energy(h, high_s, v, Fixed::from_int(300), p);
        let g_hot_high = apparent_gibbs_energy(h, high_s, v, Fixed::from_int(1300), p);
        assert!(g_hot_low < g_cold_low, "heating lowers G through -T*S");
        assert!(
            (g_cold_high - g_hot_high) > (g_cold_low - g_hot_low),
            "the higher-entropy phase drops faster with temperature"
        );
    }

    #[test]
    fn pressure_raises_the_gibbs_energy_through_the_volume_term() {
        // At fixed temperature, raising pressure raises G (the +V*(P-P_ref) work), and by more for the
        // larger-volume phase: the thermodynamic reason depth favours the denser, lower-volume assemblage.
        let t = Fixed::from_int(1000);
        let h = Fixed::from_ratio(-91070, 100);
        let s = Fixed::from_ratio(4146, 100);
        let small_v = Fixed::from_int(11); // a dense phase
        let large_v = Fixed::from_int(44); // an open phase
                                           // One kilobar (1000 bar) of pressure work.
        let g_low_small = apparent_gibbs_energy(h, s, small_v, t, Fixed::from_int(1));
        let g_high_small = apparent_gibbs_energy(h, s, small_v, t, Fixed::from_int(1000));
        let g_low_large = apparent_gibbs_energy(h, s, large_v, t, Fixed::from_int(1));
        let g_high_large = apparent_gibbs_energy(h, s, large_v, t, Fixed::from_int(1000));
        assert!(
            g_high_small > g_low_small,
            "compression raises G through +V*dP"
        );
        assert!(
            (g_high_large - g_low_large) > (g_high_small - g_low_small),
            "the larger-volume phase rises faster with pressure"
        );
        // The pressure work is exact: 44 cm^3 over 999 bar is 44 * 999 * 0.1 = 4395.6 J/mol.
        assert!(
            close(g_high_large - g_low_large, 44.0 * 999.0 * 0.1, 1.0),
            "the pressure work is V * dP * 0.1 J per cm^3 bar"
        );
    }

    #[test]
    fn the_registry_phases_read_a_negative_gibbs_energy_at_the_surface() {
        // Every seeded phase is a stable oxide or silicate (a negative enthalpy of formation), so its apparent
        // Gibbs energy at a warm surface is negative, and the kernel reads each phase from the registry.
        let r = PhaseRegistry::standard().expect("the embedded phase registry loads");
        let t = Fixed::from_int(300);
        let p = Fixed::from_int(1);
        for phase in r.phases() {
            let g = phase_gibbs_energy(phase, t, p);
            assert!(
                g.to_f64_lossy() < 0.0,
                "phase {} reads a negative apparent Gibbs energy at the surface",
                phase.name
            );
        }
    }
}
