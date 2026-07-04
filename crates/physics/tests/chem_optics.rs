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

//! The wave-2 chemistry and optics floors: they load onto the mechanical and fluids floors (they read
//! the shared mech.* axes and the fluids therm.temperature state axis), and their closed-form kernels
//! compute physically sensible, deterministic values (R-PHYS-W2, design Part 41, record 62.21). The
//! radiative-equilibrium temperature is the two-nested-square-root fourth root that sets a surface's
//! temperature from absorbed light without materialising T^4.

use civsim_core::Fixed;
use civsim_physics::{laws, PhysicsRegistry};

fn data_path(file: &str) -> String {
    format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
}
fn f(n: i64, d: i64) -> Fixed {
    Fixed::from_ratio(n, d)
}

#[test]
fn the_chem_optics_floor_loads_onto_the_mechanical_and_fluids_floors() {
    let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    reg.extend(data_path("fluids_floor.toml")).unwrap();
    reg.extend(data_path("chem_optics_floor.toml")).unwrap();
    // 38 mech + 20 fluids + 14 chem/optics axes; 20 + 18 + 11 laws; 2 + 2 + 0 substances.
    assert_eq!(
        reg.axis_count(),
        72,
        "the mechanical, fluids, and chem/optics axes"
    );
    assert_eq!(reg.law_count(), 50, "the wave-1 and wave-2 laws");
    assert_eq!(reg.substance_count(), 4, "iron, oak, air, water");
    assert!(reg.law("law.radiative_equilibrium").is_some());
    assert!(reg.axis("chem.formation_enthalpy").is_some());
    assert!(reg.axis("opt.spectral_band").is_some());
}

#[test]
fn the_chem_optics_ranges_are_owner_set_and_read_back_exactly() {
    // The owner ratified the wave-2 chemistry-and-optics ranges from their cited bounds (2026-07-02).
    // A set range reads back exactly, and no axis over the stack stays range-reserved now that the
    // geometry second moment of area is set (2026-07-03, R-UNITS-PIN).
    let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    reg.extend(data_path("fluids_floor.toml")).unwrap();
    reg.extend(data_path("chem_optics_floor.toml")).unwrap();
    let (lo, hi) = reg
        .axis("opt.refractive_index")
        .unwrap()
        .range
        .require("opt.refractive_index")
        .unwrap();
    assert_eq!(lo, Fixed::from_int(1), "refractive index of vacuum");
    assert_eq!(
        hi,
        Fixed::from_int(5),
        "the high-index headroom above diamond"
    );
    // The only range-reserved axes over the stack are the three acoustic channel-physics axes added
    // 2026-07-03, surfaced reserved-with-basis (the owner's to set, never fabricated).
    assert_eq!(
        reg.reserved_axis_ids(),
        vec![
            "acoustic.absorption_reference",
            "acoustic.formant_frequency",
            "acoustic.resonator_length",
        ],
        "exactly the three new acoustic axes are reserved-with-basis"
    );
}

#[test]
fn carnot_efficiency_is_the_temperature_ratio_and_bounded() {
    // A steam engine 500 K / 300 K: eta = 1 - 300/500 = 0.4.
    let eta = laws::carnot_limit(Fixed::from_int(500), Fixed::from_int(300)).to_f64_lossy();
    assert!(
        (0.35..0.45).contains(&eta),
        "Carnot 500/300 ~0.4, got {eta}"
    );
    // No gradient, no work; and a wider gradient is more efficient.
    assert_eq!(
        laws::carnot_limit(Fixed::from_int(300), Fixed::from_int(300)),
        Fixed::ZERO
    );
    let hot = laws::carnot_limit(Fixed::from_int(1200), Fixed::from_int(300)).to_f64_lossy();
    assert!(hot > eta, "a hotter source is more efficient");
}

#[test]
fn reaction_enthalpy_signs_exothermic_and_gates_on_the_barrier() {
    // Products lower in enthalpy than reactants is exothermic (negative delta_h).
    let (dh, crossed) = laws::reaction(
        Fixed::from_int(-400),
        Fixed::from_int(-100),
        Fixed::from_int(600),
        Fixed::from_int(500),
    );
    assert!(dh < Fixed::ZERO, "downhill reaction is exothermic");
    assert!(
        crossed,
        "above the barrier temperature the reaction proceeds"
    );
    let (_, cold) = laws::reaction(
        Fixed::from_int(-400),
        Fixed::from_int(-100),
        Fixed::from_int(400),
        Fixed::from_int(500),
    );
    assert!(!cold, "below the barrier it does not");
}

#[test]
fn corrosion_needs_a_favourable_potential_and_rises_as_ph_falls() {
    let cap = Fixed::from_int(1000);
    // Oxidiser above the material potential attacks; acid attack rises as pH falls, so a strongly
    // acidic pH 1 fluid corrodes faster than a near-neutral pH 6 one at the same favourable potential.
    let acidic = laws::corrosion(f(12, 10), f(-4, 10), f(5, 10), Fixed::ONE, cap).to_f64_lossy(); // pH 1
    let mild =
        laws::corrosion(f(12, 10), f(-4, 10), f(5, 10), Fixed::from_int(6), cap).to_f64_lossy(); // pH 6
    assert!(
        acidic > mild,
        "a more acidic (lower pH) fluid corrodes faster"
    );
    // A basic pH 14 fluid is the least aggressive: it reaches the zero-aggressiveness floor.
    assert_eq!(
        laws::corrosion(f(12, 10), f(-4, 10), f(5, 10), Fixed::from_int(14), cap),
        Fixed::ZERO,
        "at the pH ceiling the acid-attack aggressiveness is zero"
    );
    // A noble material (higher potential than the oxidiser) does not corrode.
    assert_eq!(
        laws::corrosion(f(-4, 10), f(12, 10), f(5, 10), Fixed::ONE, cap),
        Fixed::ZERO,
        "an uphill pairing does not attack"
    );
}

#[test]
fn radiant_emission_rises_steeply_with_temperature_and_caps_at_plasma() {
    let sigma = Fixed::from_ratio(567, 10_000_000_000); // 5.67e-8
    let flux = Fixed::from_int(2_000_000_000);
    let area = Fixed::ONE;
    let warm = laws::radiant_emission(
        Fixed::ONE,
        area,
        Fixed::from_int(400),
        Fixed::from_int(300),
        sigma,
        flux,
    )
    .to_f64_lossy();
    let hot = laws::radiant_emission(
        Fixed::ONE,
        area,
        Fixed::from_int(800),
        Fixed::from_int(300),
        sigma,
        flux,
    )
    .to_f64_lossy();
    assert!(
        hot > warm * 8.0,
        "doubling the surface temperature raises emission steeply (T^4)"
    );
    // A cooler surface than ambient emits nothing net here.
    assert_eq!(
        laws::radiant_emission(
            Fixed::ONE,
            area,
            Fixed::from_int(300),
            Fixed::from_int(400),
            sigma,
            flux
        ),
        Fixed::ZERO
    );
    // A plasma temperature routes to the cap (the honest Tier-0 limit).
    let plasma = laws::radiant_emission(
        Fixed::ONE,
        area,
        Fixed::from_int(30000),
        Fixed::from_int(300),
        sigma,
        flux,
    );
    assert_eq!(
        plasma, flux,
        "beyond ~14000 K the emissive power routes to the cap"
    );
}

#[test]
fn radiative_equilibrium_sets_a_temperature_by_the_nested_square_roots() {
    let sigma = Fixed::from_ratio(567, 10_000_000_000);
    let t_max = Fixed::from_int(100000);
    // Absorbed ~239 W/m^2 (Earth-like, albedo 0.3 of ~342), emissivity 1 -> T ~ 255 K.
    let t =
        laws::radiative_equilibrium(Fixed::from_int(239), Fixed::ONE, sigma, t_max).to_f64_lossy();
    assert!(
        (230.0..280.0).contains(&t),
        "radiative-equilibrium temperature ~255 K, got {t}"
    );
    // More absorbed flux is a hotter equilibrium.
    let hotter =
        laws::radiative_equilibrium(Fixed::from_int(1000), Fixed::ONE, sigma, t_max).to_f64_lossy();
    assert!(hotter > t, "more absorbed irradiance sets a hotter surface");
    // No light, no temperature.
    assert_eq!(
        laws::radiative_equilibrium(Fixed::ZERO, Fixed::ONE, sigma, t_max),
        Fixed::ZERO
    );
}

#[test]
fn the_kernels_are_total_on_adversarial_inputs() {
    // Regression (wave-2 audit): reaction's difference of two saturated opposite-signed sums must
    // saturate, not panic; interface_split with an out-of-domain negative reflectance must not
    // overflow the residual. Both are total.
    let (dh, _) = laws::reaction(
        Fixed::MAX,
        Fixed::MIN,
        Fixed::from_int(300),
        Fixed::from_int(300),
    );
    assert_eq!(
        dh,
        Fixed::MAX,
        "the saturated difference routes to the extreme, no panic"
    );
    let (r, a, t) = laws::interface_split(
        Fixed::from_int(100),
        Fixed::from_int(-1),
        Fixed::from_int(2),
    );
    // A negative reflectance clamps to zero and a >1 transmittance clamps to one; the split stays sane.
    assert_eq!(r, Fixed::ZERO);
    assert_eq!(t, Fixed::from_int(100));
    assert_eq!(a, Fixed::ZERO);
}

#[test]
fn optics_reach_splits_and_attenuates_and_refracts() {
    // Inverse-square: farther is dimmer.
    let near = laws::inverse_square_falloff(
        Fixed::from_int(1000),
        Fixed::from_int(1),
        f(1256, 100),
        Fixed::from_int(1_000_000),
    )
    .to_f64_lossy();
    let far = laws::inverse_square_falloff(
        Fixed::from_int(1000),
        Fixed::from_int(10),
        f(1256, 100),
        Fixed::from_int(1_000_000),
    )
    .to_f64_lossy();
    assert!(near > far, "a farther source is dimmer");
    // Interface split conserves: reflected + absorbed + transmitted = incident.
    let (r, a, t) = laws::interface_split(Fixed::from_int(100), f(3, 10), f(2, 10));
    let sum = (r + a + t).to_f64_lossy();
    assert!((99.0..101.0).contains(&sum), "R+A+T = incident, got {sum}");
    assert!(a.to_f64_lossy() > 0.0, "the residual is the absorbed half");
    // Refraction into a denser medium; TIR is possible going the other way.
    let (contrast, tir) = laws::refractive_contrast(Fixed::ONE, f(133, 100), Fixed::from_int(100));
    assert!(
        contrast.to_f64_lossy() > 1.0 && !tir,
        "air to water: n rises, no TIR"
    );
    let (_, tir_back) = laws::refractive_contrast(f(133, 100), Fixed::ONE, Fixed::from_int(100));
    assert!(
        tir_back,
        "water to air: total internal reflection is possible"
    );
    // Optical depth and dissolution replay and clamp.
    assert_eq!(
        laws::dissolution(f(8, 10), f(5, 10)),
        f(4, 10),
        "leach fraction is affinity times aggressiveness, clamped"
    );
    let d = laws::optical_depth(f(1, 2), Fixed::from_int(4), Fixed::from_int(1000));
    assert_eq!(d, Fixed::from_int(2), "optical depth is alpha times path");
}
