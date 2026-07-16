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

//! The PER-WORLD DISK COMPOSITION: the star's own elemental abundance pattern, the initial condition the
//! condensation sequence reads. This is the datum that retires the solar-bias defect where every world's crust
//! condensed from the Sun's composition. The condensation kernel (`surface_composition::derive_surface_composition`
//! and the viewer's `derive_uncompressed_bulk_density`) reads WHATEVER pattern this datum carries, so each world's
//! crust and density derive from that world's own star, not a universal solar table.
//!
//! THE PATTERN (a per-world initial-condition datum, the `OrbitalElements::dev_earth` and `DiurnalSky::mirror`
//! model). A world's disk composition is a physical property of its star, not an Earth or Sun constant. The pattern
//! lives here as a labelled per-world datum: one option among many, surfaced for the owner or for worldgen to set,
//! never a silent global default. Physics may be an authored cultural input (Principle 9): the composition a world
//! forms from is such an input, read as data per world. Nothing in this datum authors a cultural or emergent
//! outcome; it seeds the physics, and the crust that follows emerges from the condensation.
//!
//! ADMIT THE ALIEN (Prime Directive 7). The datum keys on the star's own abundance pattern, so an alien star is a
//! data row, never a rewrite. A carbon star with `C/O > 1` is a different pattern that will drive the condensation
//! toward carbides and graphite rather than silicates (once the carbide gas species land in the JANAF budget); a
//! metal-poor (low-`Z`) or metal-rich star is a pattern with the rock-formers scaled against hydrogen. The
//! [`DiskComposition::from_pattern`] constructor takes any such pattern; [`DiskComposition::mirror`] is the one
//! labelled fixture (the Sun's composition, Mirror's initial condition), the same standing as `dev_earth`.
//!
//! THE HELD-OUT REFERENCE, NOT THE INPUT. The condensation reads THIS per-world datum. The AGSS09 solar table
//! (`civsim_physics::solar_abundances`) and the Lodders 2003 condensation fronts
//! (`civsim_physics::condensation`) are HELD-OUT CROSS-CHECKERS: they validate the DERIVED output (the condensation
//! front, the mineralogy, the density), on a DIFFERENT quantity than the abundance input, never fed into the
//! reasoning. The cross-check in this module's tests mirrors the Lodders gate in
//! `equilibrium_condensation`: it runs the Mirror datum through the condensation and proves the derived iron front
//! reproduces the INDEPENDENTLY-computed Lodders front within an inter-dataset band. That the Mirror datum's
//! numbers coincide with the Sun's measured composition is expected (Mirror mirrors the Sun, so the Sun's real
//! pattern is Mirror's initial condition, Directive-blessed); the non-circularity is that the validation compares a
//! derived condensation temperature against an independent thermochemistry, not "abundances in, abundances out".
//!
//! NORTH STAR (flagged, not built). The deep version derives the composition ITSELF from stellar nucleosynthesis
//! and galactic chemical evolution, so even the pattern is an output of an upstream substrate and the AGSS09 table
//! becomes a pure check with no input role anywhere. That is a large arc (a nucleosynthetic-yield substrate keyed
//! on stellar mass, generation, and metallicity history); this datum is the initial-condition rung beneath it, the
//! seam where such a derivation would plug in. Until it lands, the pattern is read as per-world data.

use civsim_core::Fixed;
use civsim_physics::solar_abundances::{SolarAbundanceError, SolarAbundances};

/// A world's DISK COMPOSITION: the elemental abundance pattern its star and disk formed from, the initial condition
/// the condensation sequence reads. The `label` is provenance (which world's pattern this is), not canonical state;
/// the `abundances` carry the per-element pattern in the [`SolarAbundances`] container (the abundance-pattern type,
/// reused for its shape, not because the pattern must be solar). The condensation reads [`DiskComposition::pattern`]
/// exactly where it used to read `SolarAbundances::standard()`, so the pipeline changes not at all; only the SOURCE
/// of the pattern moves from a universal solar table to this per-world datum.
#[derive(Debug, Clone)]
pub struct DiskComposition {
    label: String,
    abundances: SolarAbundances,
}

impl DiskComposition {
    /// Build a disk composition from an explicit per-world abundance pattern. The general constructor: a worldgen
    /// stage, an owner-set scenario, or (the north star) a nucleosynthesis substrate hands the pattern in, and the
    /// condensation reads it. `label` names the world's pattern for provenance. This is the seam the alien enters
    /// as data: a carbon-star or metal-poor pattern is a different `abundances`, never a code change.
    pub fn from_pattern(label: impl Into<String>, abundances: SolarAbundances) -> Self {
        DiskComposition {
            label: label.into(),
            abundances,
        }
    }

    /// The MIRROR world's disk composition: the Sun's own measured elemental pattern as Mirror's initial condition,
    /// the labelled fixture in the standing of `OrbitalElements::dev_earth` and `DiurnalSky::mirror`. Mirror mirrors
    /// the real Sun, so its formation pattern IS the Sun's real composition (the AGSS09 measurement), read here as
    /// Mirror's per-world datum. An alien world sets a different pattern through [`DiskComposition::from_pattern`];
    /// this is one option among many, not a global default. Fails loud if the solar pattern does not load (the
    /// fail-loud-while-reserved discipline: no silent fallback).
    pub fn mirror() -> Result<Self, SolarAbundanceError> {
        Ok(DiskComposition {
            label: "mirror-solar".to_string(),
            abundances: SolarAbundances::standard()?,
        })
    }

    /// The per-world abundance pattern the condensation reads, handed to
    /// `surface_composition::derive_surface_composition` in place of `SolarAbundances::standard()`.
    pub fn pattern(&self) -> &SolarAbundances {
        &self.abundances
    }

    /// The provenance label naming which world's pattern this is (not canonical state).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The star model's metallicity ratio `Z / Z_sun` for this datum, the composition axis `derive_planet`
    /// reads, sourced from THIS per-world datum rather than a hardcoded `Fixed::ONE` at the call site. The
    /// SOLAR INSTANCE ([`DiskComposition::mirror`]) returns exactly [`Fixed::ONE`]: its heavy-element mass
    /// fraction IS the pinned solar anchor's (`Z == Z_sun`), so the ratio is UNITY BY CONSTRUCTION, no value
    /// chosen and none fabricated. A datum carrying a DIFFERENT, drawn `Z` has no numeric ratio here yet
    /// (`None`): supplying `Z / Z_sun` for a drawn composition is the abundance-DRAW generator's work (a
    /// separate commit gated on the dispersion fetch), the honest letter-versus-substance gap held open at this
    /// seam rather than papered over. `None` also if the solar anchor fails to load.
    pub fn metallicity_ratio_to_solar(&self) -> Option<Fixed> {
        let solar = SolarAbundances::standard().ok()?;
        if self.abundances.z_mass_fraction() == solar.z_mass_fraction() {
            // The solar instance: Z / Z_sun = 1 exactly, unity forced by the datum resolving to the anchor.
            return Some(Fixed::ONE);
        }
        // A drawn, non-solar Z: the numeric ratio arrives with the generator commit (path 1). Held open, never faked.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_core::Fixed;

    #[test]
    fn the_mirror_datum_carries_the_suns_pattern_as_its_initial_condition() {
        // The per-world Mirror datum resolves to the Sun's real composition (Mirror mirrors the Sun). The
        // condensation reads THIS datum's pattern, not a universal solar table hardcoded in the pipeline.
        let disk = DiskComposition::mirror().expect("the Mirror disk composition loads");
        assert_eq!(disk.label(), "mirror-solar");
        // The load-bearing rock-formers are present in the pattern (the condensation reads them from here).
        let fe = disk
            .pattern()
            .preferred("Fe")
            .expect("iron is in the Mirror pattern");
        assert!(
            (fe.to_f64_lossy() - 7.50).abs() < 0.05,
            "the Mirror pattern's iron is the solar log-eps 7.50, got {}",
            fe.to_f64_lossy()
        );
        let o = disk
            .pattern()
            .preferred("O")
            .expect("oxygen in the pattern");
        // Oxygen exceeds carbon in the solar pattern (C/O < 1), the silicate-condensing regime.
        let c = disk
            .pattern()
            .preferred("C")
            .expect("carbon in the pattern");
        assert!(
            o.to_f64_lossy() > c.to_f64_lossy(),
            "the solar pattern is oxygen-rich (C/O < 1): O {} > C {}",
            o.to_f64_lossy(),
            c.to_f64_lossy()
        );
    }

    #[test]
    fn the_solar_instance_metallicity_ratio_is_unity() {
        // The per-world metallicity ratio the star model reads, sourced from the datum rather than a bare
        // `Fixed::ONE` literal. The Mirror (solar) datum resolves to Z == Z_sun, so the ratio is exactly ONE
        // (unity by construction, no value chosen). This is the seam that will carry a DRAWN Z / Z_sun once the
        // abundance-draw generator lands.
        let disk = DiskComposition::mirror().expect("the Mirror disk composition loads");
        assert_eq!(
            disk.metallicity_ratio_to_solar(),
            Some(Fixed::ONE),
            "the solar-instance datum's metallicity ratio Z/Z_sun is unity"
        );
    }

    #[test]
    fn a_drawn_non_solar_metallicity_has_no_ratio_until_the_generator() {
        // A datum whose Z differs from the solar anchor (here a pattern that carries no Z, standing in for a
        // drawn composition) has no numeric ratio at this seam yet: the generator commit supplies it. The honest
        // gap is a None, never a fabricated ratio.
        let no_z_pattern = r#"
[[abundance]]
symbol = "H"
z = 1
log_eps_photosphere = "12.00"
source = "photospheric"

[[abundance]]
symbol = "Fe"
z = 26
log_eps_photosphere = "7.50"
source = "photospheric"
"#;
        let pattern =
            SolarAbundances::from_toml_str(no_z_pattern).expect("the stand-in pattern parses");
        let disk = DiskComposition::from_pattern("drawn-stand-in", pattern);
        assert_eq!(
            disk.metallicity_ratio_to_solar(),
            None,
            "a non-solar Z has no ratio until the generator commit fills it"
        );
    }

    #[test]
    fn an_alien_carbon_star_is_a_data_row_not_a_rewrite() {
        // ADMIT THE ALIEN. A carbon star with C/O > 1 is a different abundance pattern handed to the SAME datum and
        // the SAME pipeline. The datum carries it; the condensation would read C > O and drive toward carbides and
        // graphite (once the carbide gas species land in the JANAF budget). No pathway assumes an oxygen-rich,
        // silicate world: the star is data.
        let carbon_star = r#"
[[abundance]]
symbol = "H"
z = 1
log_eps_photosphere = "12.00"
source = "photospheric"

[[abundance]]
symbol = "C"
z = 6
log_eps_photosphere = "9.20"
source = "photospheric"

[[abundance]]
symbol = "O"
z = 8
log_eps_photosphere = "8.70"
source = "photospheric"

[[abundance]]
symbol = "Fe"
z = 26
log_eps_photosphere = "7.50"
source = "photospheric"
"#;
        let pattern =
            SolarAbundances::from_toml_str(carbon_star).expect("the alien pattern parses as data");
        let disk = DiskComposition::from_pattern("alien-carbon-star", pattern);
        assert_eq!(disk.label(), "alien-carbon-star");
        let c = disk
            .pattern()
            .preferred("C")
            .expect("carbon in the pattern");
        let o = disk
            .pattern()
            .preferred("O")
            .expect("oxygen in the pattern");
        assert!(
            c.to_f64_lossy() > o.to_f64_lossy(),
            "the alien pattern is carbon-rich (C/O > 1): C {} > O {}, the carbide-condensing regime the datum admits",
            c.to_f64_lossy(),
            o.to_f64_lossy()
        );
    }

    #[test]
    fn the_mirror_datum_reproduces_the_lodders_iron_front_held_out() {
        // THE HELD-OUT CROSS-CHECK, mirroring `equilibrium_condensation::the_iron_condensation_temperature_
        // reproduces_the_lodders_front`, but SOURCING the iron abundance from the PER-WORLD MIRROR DATUM rather
        // than an inline solar fixture. This proves the per-world SOURCE flows into the physics and the DERIVED
        // condensation front reproduces the INDEPENDENTLY-computed Lodders front.
        //
        // WHAT IS THE INPUT: the per-world Mirror abundance pattern (its iron and helium log-eps), read from the
        // datum. WHAT IS HELD OUT: the Lodders 2003 iron T50 (computed by Lodders with her OWN thermochemistry, an
        // independent dataset) and the JANAF floor thermochemistry. WHAT THE CROSS-CHECK COMPARES: the derived
        // 50%-condensation TEMPERATURE against the Lodders temperature, a DIFFERENT quantity than the abundance
        // input, so this is never "abundances in, abundances out". The condensation front is a physics observable,
        // Lodders never touches the derivation, and the derivation never touches the Lodders table.
        let disk = DiskComposition::mirror().expect("the Mirror disk composition loads");

        // The iron partial pressure DERIVES from the Mirror datum's pattern, nothing fabricated. n_Fe/n_H =
        // 10^(log_eps(Fe) - 12); the gas is hydrogen-dominated, so the total particle count per H nucleus is
        // n_H2 + n_He = 0.5 (hydrogen mostly molecular H2) + 10^(log_eps(He) - 12) (helium, from the datum), and
        // x_Fe = (n_Fe/n_H) / that count. P_Fe = x_Fe * P_total at the disk total pressure 1e-4 bar. Every input
        // is read from the per-world datum (Fe and He abundances) or the disk setting.
        let log_eps_fe = disk.pattern().preferred("Fe").expect("iron in the pattern");
        let log_eps_he = disk
            .pattern()
            .preferred("He")
            .expect("helium in the pattern");
        let n_fe_over_n_h = (10.0_f64).powf(log_eps_fe.to_f64_lossy() - 12.0);
        let n_he_over_n_h = (10.0_f64).powf(log_eps_he.to_f64_lossy() - 12.0);
        // 0.5: hydrogen is mostly molecular H2 in the disk gas, so half as many H2 particles as H nuclei. This is a
        // gas-state fact, not a fabricated tuneable; the helium term is read from the datum.
        let particles_per_h = 0.5 + n_he_over_n_h;
        let x_fe = n_fe_over_n_h / particles_per_h;
        let p_total_bar = 1.0e-4_f64;
        let p_fe_bar = x_fe * p_total_bar;

        // f(T) = g(gas) - g(solid) through the JANAF mu-standard floor (the same wire the sibling gate uses).
        let janaf = civsim_physics::janaf::JanafTables::standard().expect("JANAF loads");
        let fe_g = janaf.species("Fe(g)").expect("Fe(g) in JANAF");
        let fe_cr = janaf.species("Fe(cr)").expect("Fe(cr) in JANAF");
        let f = |t: f64| -> f64 {
            let tf = Fixed::from_int(t as i32);
            let g_gas = crate::equilibrium_condensation::janaf_g_over_rt(
                fe_g.delta_f_g_at(tf).unwrap(),
                tf,
            )
            .unwrap();
            let g_sol = crate::equilibrium_condensation::janaf_g_over_rt(
                fe_cr.delta_f_g_at(tf).unwrap(),
                tf,
            )
            .unwrap();
            g_gas.checked_sub(g_sol).unwrap().to_f64_lossy()
        };
        // target = ln(P0/P_Fe) + ln 2 (at T50 half the iron is condensed, so the residual gas pressure is halved).
        let target = (1.0_f64 / p_fe_bar).ln() + 2.0_f64.ln();
        let f_lo = f(1300.0);
        let f_hi = f(1400.0);
        assert!(
            f_lo > target && target > f_hi,
            "the Fe T50 is bracketed by [1300, 1400] K: f(1300)={f_lo:.2} > target={target:.2} > f(1400)={f_hi:.2}"
        );
        let t50 = 1300.0 + 100.0 * (f_lo - target) / (f_lo - f_hi);

        // THE HELD-OUT LODDERS FRONT, consumed only here in validation, never in the derivation above.
        let lodders =
            civsim_physics::condensation::CondensationTable::standard().expect("Lodders loads");
        let lodders_fe = lodders.t50_k("Fe").expect("Fe in Lodders").to_f64_lossy();
        assert!(
            (t50 - lodders_fe).abs() < 30.0,
            "the Mirror-datum-sourced Fe T50 ({t50:.0} K) reproduces the held-out Lodders front ({lodders_fe:.0} K) within the inter-dataset +-30 K band"
        );
    }
}
