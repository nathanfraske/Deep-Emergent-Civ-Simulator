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

//! The eruption SOURCE energetics: the speed a fragmenting magma leaves the vent at, DERIVED from the
//! expansion of its exsolved volatiles rather than authored. As magma rises and decompresses, its dissolved
//! gas comes out of solution and expands, and that expansion work is what accelerates the mixture out of the
//! vent. Taking the expansion as isothermal (the standard first approximation, Wilson 1980; Woods 1995), the
//! kinetic energy per unit mass equals the expansion work per unit mass, so the exit speed is
//! `v = sqrt(2 * n * (R/M) * T * ln(P0 / P_atm))`: the gas mass fraction `n`, the volatile's specific gas
//! constant `R/M`, the magma temperature `T`, and the pressure drop from the fragmentation level `P0` to the
//! surface `P_atm`.
//!
//! This is the seam that connects the eruption to the rest of the derived world. `T` is the magma
//! temperature, which the interior geodynamics sets; `P_atm` is the surface pressure, which the derived
//! atmosphere sets; `n` and `R/M` are the magma's volatile load and its species, DATA that a fuller chain
//! derives from the melt composition and the solubility. The alien seam is `R/M`: a water-driven eruption,
//! a carbon-dioxide-driven one, or a world whose magmas carry an exotic volatile is a different `R/M` row,
//! never a rewrite (a lighter volatile of larger `R/M` drives a faster jet). A thinner atmosphere lets the
//! gas expand further and erupts faster, with no code change. Determinism: fixed-point throughout (the log
//! through the pinned [`Fixed::ln`], the speed through the exact [`Fixed::sqrt`]).

use civsim_core::Fixed;

/// The gas-thrust exit velocity of a fragmenting magma, `v = sqrt(2 n (R/M) T ln(P0/P_atm))`. `None` on a
/// non-physical input: a negative gas fraction, a non-positive specific gas constant or temperature, or a
/// pressure ratio that is not above one (no net expansion, so no drive), or on any overflow.
pub fn gas_thrust_exit_velocity(
    gas_mass_fraction: Fixed,
    specific_gas_constant: Fixed,
    magma_temperature_k: Fixed,
    chamber_pressure: Fixed,
    surface_pressure: Fixed,
) -> Option<Fixed> {
    if gas_mass_fraction < Fixed::ZERO
        || specific_gas_constant <= Fixed::ZERO
        || magma_temperature_k <= Fixed::ZERO
        || surface_pressure <= Fixed::ZERO
        || chamber_pressure <= surface_pressure
    {
        return None;
    }
    let ratio = chamber_pressure.checked_div(surface_pressure)?;
    let ln_ratio = ratio.ln(); // > 0 since ratio > 1
    if ln_ratio <= Fixed::ZERO {
        return None;
    }
    // 2 * n * (R/M) * T * ln(ratio), built so each intermediate stays in range for physical inputs.
    let energy = Fixed::from_int(2)
        .checked_mul(gas_mass_fraction)?
        .checked_mul(specific_gas_constant)?
        .checked_mul(magma_temperature_k)?
        .checked_mul(ln_ratio)?;
    Some(energy.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Water vapour and carbon dioxide specific gas constants R/M (J/kg/K), the volatile-species data.
    fn r_water() -> Fixed {
        Fixed::from_ratio(462, 1) // 8.314 / 0.018
    }
    fn r_co2() -> Fixed {
        Fixed::from_ratio(189, 1) // 8.314 / 0.044
    }
    fn base() -> (Fixed, Fixed, Fixed) {
        // (T, P0, P_atm): 1200 K, 100 MPa chamber, 0.1 MPa surface, ratio 1000.
        (
            Fixed::from_int(1200),
            Fixed::from_int(100_000_000),
            Fixed::from_int(100_000),
        )
    }

    #[test]
    fn a_water_rich_magma_erupts_at_hundreds_of_metres_per_second() {
        let (t, p0, pa) = base();
        let v = gas_thrust_exit_velocity(Fixed::from_ratio(3, 100), r_water(), t, p0, pa)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (v - 479.0).abs() < 4.0,
            "a 3% water magma erupts at ~479 m/s, got {v}"
        );
    }

    #[test]
    fn the_exit_speed_scales_as_the_square_root_of_the_gas_fraction() {
        let (t, p0, pa) = base();
        let v1 = gas_thrust_exit_velocity(Fixed::from_ratio(3, 100), r_water(), t, p0, pa)
            .unwrap()
            .to_f64_lossy();
        let v2 = gas_thrust_exit_velocity(Fixed::from_ratio(6, 100), r_water(), t, p0, pa)
            .unwrap()
            .to_f64_lossy();
        assert!(
            (v2 / v1 - std::f64::consts::SQRT_2).abs() < 0.02,
            "doubling the gas fraction raises the speed by sqrt(2), ratio {}",
            v2 / v1
        );
    }

    #[test]
    fn a_lighter_volatile_drives_a_faster_jet_admit_the_alien() {
        // Same gas fraction, different volatile: water (larger R/M) drives a faster jet than carbon dioxide,
        // by the ratio sqrt(R_water/R_co2). The species is a data row, never a rewrite.
        let (t, p0, pa) = base();
        let n = Fixed::from_ratio(3, 100);
        let vw = gas_thrust_exit_velocity(n, r_water(), t, p0, pa)
            .unwrap()
            .to_f64_lossy();
        let vc = gas_thrust_exit_velocity(n, r_co2(), t, p0, pa)
            .unwrap()
            .to_f64_lossy();
        let expected = (189.0f64 / 462.0).sqrt();
        assert!(
            (vc / vw - expected).abs() < 0.01,
            "the CO2/H2O speed ratio is sqrt(R_co2/R_water) = {expected}, got {}",
            vc / vw
        );
    }

    #[test]
    fn it_is_deterministic_and_guards_the_inputs() {
        let (t, p0, pa) = base();
        let n = Fixed::from_ratio(3, 100);
        assert_eq!(
            gas_thrust_exit_velocity(n, r_water(), t, p0, pa),
            gas_thrust_exit_velocity(n, r_water(), t, p0, pa),
            "the exit velocity replays byte for byte"
        );
        // No net expansion (chamber not above surface) has no drive.
        assert_eq!(gas_thrust_exit_velocity(n, r_water(), t, pa, p0), None);
        // A non-physical temperature is rejected.
        assert_eq!(
            gas_thrust_exit_velocity(n, r_water(), Fixed::ZERO, p0, pa),
            None
        );
    }
}
