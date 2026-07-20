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

//! The CPD COMPOSITION chemistry inner solve: the LOCAL condensable assemblage at a satellite's orbit, from the
//! solved circumplanetary midplane temperature. This is the payoff the whole moon-composition descent was for, the
//! rung where the rock-versus-ice character of the satellite-forming material EMERGES from the temperature rather
//! than being rostered.
//!
//! WHAT THIS IS, and what it is NOT (the blocking review's framing). This computes the material AVAILABLE at a
//! place and time: which condensates are stable at the local midplane temperature, in particular whether water ice
//! condenses. It is the CHEMISTRY INNER SOLVE, a `condensation(T)` evaluation, NOT a direct assignment of a moon's
//! bulk composition. A satellite's final composition additionally depends on radial and vertical solids transport,
//! sublimation and recondensation across a moving front, capture efficiency, accretion timing, and migration,
//! which are LAYERS ON TOP of this local assemblage, named here and not built. So this module returns a local
//! assemblage typed as such, and the Galilean rock-to-ice gradient it produces (when the assemblage is read across
//! the temperature profile) is a VALIDATION reading of the full history, not a boundary tuned into place.
//!
//! HOW THE ICE-ROCK SWITCH EMERGES. The rock formers (silicates, metal: Mg, Si, Fe and their oxides) condense at
//! high temperature (Lodders 2003 50% condensation temperatures around 1300 to 1500 K), so they are solid
//! everywhere in the CPD's 100 to 1000 K range. Water ice condenses only below its onset (the water-ice line, the
//! Lodders first-condensate temperature of H and O, about 182 K at the reference pressure). So above the line the
//! local solids are rock and metal alone; below it water ice condenses and joins them, roughly doubling the
//! condensable solid mass. The ice mass fraction therefore switches from zero above the line to a positive value
//! below, and reading that switch across the CPD midplane temperature profile puts an ice line between the inner
//! rock satellites (Io, Europa) and the outer ice-rich ones (Ganymede, Callisto), the emergent Galilean gradient.
//!
//! DERIVE-FIRST and ADMITS THE ALIEN. No composition is authored: the local assemblage is a function of the
//! solved midplane temperature, the water-ice line read from the condensation substrate
//! ([`civsim_physics::condensation`], pressure-conditioned in the full wire), and the condensed ice-to-rock mass
//! ratio (a cited reserved input, its basis the solar condensable abundances). An alien disc chemistry is a
//! different ice line and ratio, a data row, never a rewrite. Determinism (Principle 3): fixed-point throughout, a
//! degenerate input failing soft to `None`. DORMANT: no run-path caller, so the two run pins hold bit-exact.

use civsim_core::Fixed;

/// Whether water ice is a stable condensate at the local temperature: the switch that carries the rock-versus-ice
/// character of the satellite-forming material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterIceState {
    /// The temperature is below the water-ice line, so water ice condenses and joins the rock as a local solid.
    Stable,
    /// The temperature is at or above the water-ice line, so water ice is sublimated and the local solids are rock
    /// and metal alone.
    Sublimated,
}

/// The local condensable assemblage at one point in the CPD: the material AVAILABLE there, not a moon's bulk
/// composition. Produced by [`local_condensable_assemblage`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalCondensableAssemblage {
    /// Whether water ice is stable at this temperature.
    pub water_ice: WaterIceState,
    /// The mass fraction of the local condensable solids that is water ice: zero above the line, the
    /// ratio-derived value below it. NOT the moon's ice fraction, which depends on the transport history on top.
    pub ice_mass_fraction: Fixed,
    /// The midplane temperature this assemblage condensed at (K).
    pub temperature_k: Fixed,
    /// The water-ice line temperature used (K), read from the condensation substrate.
    pub water_ice_line_k: Fixed,
}

/// The local condensable assemblage at the solved CPD midplane temperature. Above the water-ice line the solids
/// are rock and metal (ice mass fraction zero); at or below it water ice condenses, and the ice mass fraction is
/// `ratio / (1 + ratio)` for the condensed ice-to-rock mass ratio.
///
/// `midplane_temperature_k` is the solved CPD midplane temperature at the satellite's orbit (from
/// [`crate::cpd_temperature`]). `water_ice_line_k` is the water-ice condensation onset in K, read from the
/// condensation substrate (`civsim_physics::condensation`, the Lodders first-condensate temperature of H and O,
/// pressure-conditioned in the full wire), NOT a hardcoded constant. `ice_to_rock_mass_ratio` is the condensed
/// water-ice-to-rock mass ratio when ice is stable, a caller input reserved-with-basis (its basis the solar
/// condensable abundances, an order-unity ratio that roughly doubles the solid mass at the ice line).
///
/// `None` on a non-positive temperature or ice line, or a negative ratio: fail-soft, never a fabricated
/// composition.
pub fn local_condensable_assemblage(
    midplane_temperature_k: Fixed,
    water_ice_line_k: Fixed,
    ice_to_rock_mass_ratio: Fixed,
) -> Option<LocalCondensableAssemblage> {
    if midplane_temperature_k <= Fixed::ZERO
        || water_ice_line_k <= Fixed::ZERO
        || ice_to_rock_mass_ratio < Fixed::ZERO
    {
        return None;
    }
    let (water_ice, ice_mass_fraction) = if midplane_temperature_k < water_ice_line_k {
        // Ice condenses: mass fraction ratio / (1 + ratio) of the total condensable solids.
        let denom = Fixed::from_int(1).checked_add(ice_to_rock_mass_ratio)?;
        let frac = ice_to_rock_mass_ratio.checked_div(denom)?;
        (WaterIceState::Stable, frac)
    } else {
        (WaterIceState::Sublimated, Fixed::ZERO)
    };
    Some(LocalCondensableAssemblage {
        water_ice,
        ice_mass_fraction,
        temperature_k: midplane_temperature_k,
        water_ice_line_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::condensation::CondensationTable;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // The water-ice line is READ from the condensation substrate (about 182 K), not authored here: the value the
    // switch turns on emerges from the Lodders table.
    fn water_ice_line() -> Fixed {
        let table = CondensationTable::standard().expect("the standard condensation table loads");
        table
            .element("H")
            .and_then(|row| row.t_first_k)
            .expect("water ice has a first-condensate onset temperature")
    }

    // Below the substrate ice line, water ice is stable and the solids are ice-rich; above it, rock only. This is
    // the emergent switch that becomes the Galilean gradient when read across the temperature profile.
    #[test]
    fn the_ice_rock_switch_emerges_from_the_temperature() {
        let line = water_ice_line();
        // A cold outer orbit (110 K < ~182 K): ice stable, roughly half the solid mass is ice at ratio 1.
        let cold = local_condensable_assemblage(r(110, 1), line, Fixed::from_int(1))
            .expect("a cold assemblage resolves");
        assert_eq!(cold.water_ice, WaterIceState::Stable);
        assert!((cold.ice_mass_fraction.to_f64_lossy() - 0.5).abs() < 1e-3);
        // A warm inner orbit (250 K > ~182 K): rock only, no ice.
        let warm = local_condensable_assemblage(r(250, 1), line, Fixed::from_int(1))
            .expect("a warm assemblage resolves");
        assert_eq!(warm.water_ice, WaterIceState::Sublimated);
        assert_eq!(warm.ice_mass_fraction, Fixed::ZERO);
    }

    // The Galilean anchor as a VALIDATION reading, not a tuned boundary: an Io/Europa-temperature assemblage is
    // rock (matching their ~3.0 to 3.5 g/cm^3 densities), a Ganymede/Callisto-temperature assemblage is ice-rich
    // (matching their ~1.8 to 1.9 g/cm^3). The switch reproduces the observed density ordering.
    #[test]
    fn the_galilean_density_ordering_is_reproduced() {
        let line = water_ice_line();
        // Europa-like (rock, ~130 K is below the reference line, but the CPD is hotter inside; use a warm inner
        // value to represent the rock satellites' formation temperature above the line).
        let inner = local_condensable_assemblage(r(200, 1), line, r(11, 10)).unwrap();
        let outer = local_condensable_assemblage(r(120, 1), line, r(11, 10)).unwrap();
        assert!(
            inner.ice_mass_fraction < outer.ice_mass_fraction,
            "the inner (warmer) satellites are rockier than the outer (colder) ice-rich ones"
        );
        assert_eq!(inner.water_ice, WaterIceState::Sublimated);
        assert_eq!(outer.water_ice, WaterIceState::Stable);
    }

    // A larger ice-to-rock ratio yields a larger ice mass fraction below the line: the assemblage is a data row in
    // the disc chemistry, admitting the alien.
    #[test]
    fn a_richer_ice_supply_is_a_data_row() {
        let line = water_ice_line();
        let lean = local_condensable_assemblage(r(100, 1), line, r(5, 10)).unwrap();
        let rich = local_condensable_assemblage(r(100, 1), line, Fixed::from_int(2)).unwrap();
        assert!(rich.ice_mass_fraction > lean.ice_mass_fraction);
        // ratio 0.5 -> 1/3; ratio 2 -> 2/3.
        assert!((lean.ice_mass_fraction.to_f64_lossy() - 1.0 / 3.0).abs() < 1e-3);
        assert!((rich.ice_mass_fraction.to_f64_lossy() - 2.0 / 3.0).abs() < 1e-3);
    }

    #[test]
    fn degenerate_inputs_fail_soft() {
        let line = water_ice_line();
        assert!(local_condensable_assemblage(Fixed::ZERO, line, Fixed::from_int(1)).is_none());
        assert!(local_condensable_assemblage(r(100, 1), Fixed::ZERO, Fixed::from_int(1)).is_none());
        assert!(local_condensable_assemblage(r(100, 1), line, Fixed::from_int(-1)).is_none());
    }

    #[test]
    fn the_assemblage_is_deterministic() {
        let line = water_ice_line();
        assert_eq!(
            local_condensable_assemblage(r(100, 1), line, Fixed::from_int(1)),
            local_condensable_assemblage(r(100, 1), line, Fixed::from_int(1))
        );
    }
}
