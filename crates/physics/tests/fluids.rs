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

//! The wave-2 fluids floor: it loads onto the wave-1 mechanical floor (it reads the shared mechanical
//! and material axes), and its closed-form kernels compute physically sensible, deterministic values
//! (R-PHYS-W2, design Part 41, record 62.21).

use civsim_core::Fixed;
use civsim_physics::{laws, PhysicsRegistry};

fn data_path(file: &str) -> String {
    format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
}

fn f(n: i64, d: i64) -> Fixed {
    Fixed::from_ratio(n, d)
}

#[test]
fn the_fluids_floor_loads_onto_the_mechanical_floor() {
    // Wave 2 extends wave 1 rather than standing alone: its laws read the shared mech.* and mat.*
    // axes, so it merges onto the mechanical floor and the whole revalidates.
    let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    reg.extend(data_path("fluids_floor.toml")).unwrap();
    // 39 mechanical + 24 fluids axes; 20 + 18 laws; 2 + 2 substances (the three acoustic
    // channel-physics axes and the acoustic_absorption and tube_resonance laws are the 2026-07-03 add;
    // fluid.moisture_content, the terrain-wetness floor identity, is the Arc 5 T4 add; the three critical-point
    // axes chem.critical_{temperature,pressure} + chem.acentric_factor feed the transport-property derivations; plus the four volatile primitives chem.boiling_point / vaporization_enthalpy / fusion_enthalpy / triple_point_temperature for the saturation curve).
    assert_eq!(
        reg.axis_count(),
        67,
        "the mechanical and fluids axes together"
    );
    assert_eq!(reg.law_count(), 39, "the wave-1 and wave-2 fluid laws");
    assert_eq!(reg.substance_count(), 4, "iron, oak, air, water");
    // The load validated every cross-reference: the fluid laws read the mechanical axes, and air and
    // water carry values only on existing axes and participate only in existing laws.
    assert!(reg.axis("fluid.lift_coefficient").is_some());
    assert!(reg.law("law.aerodynamic_lift").is_some());
    assert!(reg.substance("air").is_some());
}

#[test]
fn the_fluids_ranges_are_owner_set_and_read_back_exactly() {
    // The owner ratified the wave-2 fluids ranges from their cited bounds (2026-07-02). A set range
    // now reads back exactly. The only reserved axes over the mechanical-and-fluids stack are the
    // three acoustic channel-physics axes added 2026-07-03, whose ranges are surfaced reserved-with-
    // basis (the owner's to set, never fabricated), the reserved-value discipline.
    let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    reg.extend(data_path("fluids_floor.toml")).unwrap();
    let (lo, hi) = reg
        .axis("fluid.dynamic_viscosity")
        .unwrap()
        .range
        .require("fluid.dynamic_viscosity")
        .unwrap();
    assert_eq!(lo, Fixed::from_ratio(1, 100000), "0.00001 Pa*s (air)");
    assert_eq!(hi, Fixed::from_int(100), "100 Pa*s (lava end)");
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
fn a_standalone_fluids_floor_fails_because_it_reads_the_mechanical_axes() {
    // The fluids floor is not self-contained by design: loading it alone must error on the missing
    // mechanical axes rather than silently skip the dangling reference.
    let r = PhysicsRegistry::load(data_path("fluids_floor.toml"));
    assert!(
        r.is_err(),
        "the fluids floor alone has dangling mech.* references"
    );
}

#[test]
fn the_speed_of_sound_is_physical_for_air_and_water() {
    let c_max = Fixed::from_int(100000);
    // Air: K ~0.142 MPa, rho 1.225 -> ~340 m/s.
    let air = laws::speed_of_sound(f(142, 1000), f(1225, 1000), c_max).to_f64_lossy();
    assert!(
        (300.0..400.0).contains(&air),
        "air sound speed ~340, got {air}"
    );
    // Water: K 2200 MPa, rho 998 -> ~1480 m/s (the megapascal modulus never materialises in Pa).
    let water =
        laws::speed_of_sound(Fixed::from_int(2200), Fixed::from_int(998), c_max).to_f64_lossy();
    assert!(
        (1400.0..1600.0).contains(&water),
        "water sound speed ~1480, got {water}"
    );
}

#[test]
fn ideal_gas_density_recovers_air_at_one_atmosphere() {
    // P 0.101325 MPa, T 288.15 K, R_s 287 -> rho ~1.225 kg/m^3.
    let rho = laws::ideal_gas_density(
        f(101325, 1000000),
        f(28815, 100),
        Fixed::from_int(287),
        f(8, 100),
        Fixed::from_int(23000),
    )
    .to_f64_lossy();
    assert!((1.1..1.35).contains(&rho), "air density ~1.225, got {rho}");
    // Warmer air is lighter.
    let hot = laws::ideal_gas_density(
        f(101325, 1000000),
        Fixed::from_int(313),
        Fixed::from_int(287),
        f(8, 100),
        Fixed::from_int(23000),
    )
    .to_f64_lossy();
    assert!(hot < rho, "warmer air is less dense");
}

#[test]
fn lift_and_drag_scale_with_the_square_of_speed_and_the_coefficient() {
    let f_max = Fixed::from_int(1_000_000_000);
    let rho = f(1225, 1000);
    let area = Fixed::from_int(2);
    let slow =
        laws::aerodynamic_lift(Fixed::ONE, rho, area, Fixed::from_int(10), f_max).to_f64_lossy();
    let fast =
        laws::aerodynamic_lift(Fixed::ONE, rho, area, Fixed::from_int(20), f_max).to_f64_lossy();
    assert!(
        fast > slow * 3.5,
        "doubling speed roughly quadruples lift ({slow} -> {fast})"
    );
    // A higher lift coefficient gives more lift at the same speed; zero gives none.
    let more = laws::aerodynamic_lift(Fixed::from_int(2), rho, area, Fixed::from_int(10), f_max)
        .to_f64_lossy();
    assert!(more > slow, "a higher lift coefficient lifts more");
    assert_eq!(
        laws::aerodynamic_lift(Fixed::ZERO, rho, area, Fixed::from_int(10), f_max),
        Fixed::ZERO,
        "no lift coefficient, no lift"
    );
    // Drag shares the family.
    let drag = laws::drag_force(f(47, 100), rho, area, Fixed::from_int(10), f_max).to_f64_lossy();
    assert!(drag > 0.0, "a blunt body has drag");
}

#[test]
fn thermal_buoyancy_lifts_warm_air_and_sinks_cold() {
    let g = f(981, 100);
    let a_max = Fixed::from_int(100);
    let warm = laws::thermal_buoyancy(Fixed::from_int(293), Fixed::from_int(288), g, a_max);
    let cold = laws::thermal_buoyancy(Fixed::from_int(283), Fixed::from_int(288), g, a_max);
    assert!(warm > Fixed::ZERO, "a warmer parcel rises");
    assert!(cold < Fixed::ZERO, "a colder parcel sinks");
    assert_eq!(
        laws::thermal_buoyancy(Fixed::from_int(288), Fixed::from_int(288), g, a_max),
        Fixed::ZERO,
        "equal temperature, no buoyancy"
    );
}

#[test]
fn membrane_gas_flux_takes_up_from_a_richer_medium_and_off_gasses_to_a_poorer_one() {
    // The R-MEDIUM gas exchange: a respiratory surface exchanges the respirable species with the medium
    // it sits in, at a rate set by the transfer coefficient, the surface area, and the concentration
    // difference. Nothing tags the medium as air or water: only its respirable content differs, so the
    // same surface respires a rich medium and off-gasses to a poor one (Principle 9, emergence).
    let k = f(1, 100); // a transfer coefficient in range
    let area = f(1, 2); // an exchange area
    let internal = f(1, 20); // the body's internal concentration
    let j_max = Fixed::from_int(1000);

    let rich = f(27, 100); // an oxygen-rich medium (air-like)
    let poor = f(1, 100); // a poor medium (below the body's internal level)

    let uptake = laws::membrane_gas_flux(k, area, rich, internal, j_max);
    let loss = laws::membrane_gas_flux(k, area, poor, internal, j_max);
    let rest = laws::membrane_gas_flux(k, area, internal, internal, j_max);

    assert!(uptake > Fixed::ZERO, "a richer medium drives uptake");
    assert!(
        loss < Fixed::ZERO,
        "a poorer medium drives loss (off-gassing)"
    );
    assert_eq!(
        rest,
        Fixed::ZERO,
        "at equilibrium the flux is zero: no authored preference for a medium"
    );

    // The exchange is symmetric in the concentration gap: an equal gap up or down gives an equal and
    // opposite flux, so the law bakes in no direction of its own. The equality holds to within one
    // fixed-point unit, since the pinned multiply floors (a real arithmetic property, not a kernel
    // bias): floor(x) and floor(-x) differ by a ULP when x is not exactly representable.
    let up = laws::membrane_gas_flux(k, area, internal + f(1, 100), internal, j_max);
    let down = laws::membrane_gas_flux(k, area, internal - f(1, 100), internal, j_max);
    assert!(
        up > Fixed::ZERO && down < Fixed::ZERO,
        "opposite gaps, opposite signs"
    );
    assert!(
        (up.to_bits() + down.to_bits()).abs() <= 1,
        "uptake and loss are symmetric in the concentration gap to within a fixed-point unit"
    );
}

#[test]
fn membrane_gas_flux_needs_a_surface_and_saturates_at_the_signed_cap() {
    let k = f(1, 100);
    let area = f(1, 2);
    let internal = f(1, 20);
    let j_max = Fixed::from_int(1000);

    // No exchange surface (zero area or zero coefficient) means no exchange, whatever the medium: a
    // body with no respiratory organ cannot breathe, the physical basis the sim-side reserve leans on.
    assert_eq!(
        laws::membrane_gas_flux(k, Fixed::ZERO, Fixed::ONE, internal, j_max),
        Fixed::ZERO,
        "no area, no exchange"
    );
    assert_eq!(
        laws::membrane_gas_flux(Fixed::ZERO, area, Fixed::ONE, internal, j_max),
        Fixed::ZERO,
        "no transfer coefficient, no exchange"
    );

    // A huge coefficient and gap saturate to the signed cap rather than overflow or wrap.
    let big = Fixed::from_int(1_000_000);
    let cap = Fixed::from_int(5);
    assert_eq!(
        laws::membrane_gas_flux(big, big, big, Fixed::ZERO, cap),
        cap,
        "an unbounded uptake saturates at +J_MAX"
    );
    assert_eq!(
        laws::membrane_gas_flux(big, big, Fixed::ZERO, big, cap),
        Fixed::ZERO - cap,
        "an unbounded loss saturates at -J_MAX"
    );
}

#[test]
fn evaporation_rises_with_the_deficit_and_the_wind_and_stops_at_saturation() {
    let e_max = Fixed::from_int(1000);
    let e_s = f(74, 10000); // saturation ~0.0074 MPa
    let dry = laws::evaporation_rate(f(20, 10000), e_s, Fixed::ZERO, f(1, 100), f(1, 50), e_max)
        .to_f64_lossy();
    let windy = laws::evaporation_rate(
        f(20, 10000),
        e_s,
        Fixed::from_int(5),
        f(1, 100),
        f(1, 50),
        e_max,
    )
    .to_f64_lossy();
    assert!(windy > dry, "wind speeds evaporation");
    // Saturated air (ambient at saturation) does not evaporate.
    assert_eq!(
        laws::evaporation_rate(e_s, e_s, Fixed::from_int(5), f(1, 100), f(1, 50), e_max),
        Fixed::ZERO,
        "no deficit, no evaporation"
    );
}

#[test]
fn hydrostatic_pressure_rises_with_depth_and_buoyancy_with_volume() {
    let p_max = Fixed::from_int(100000);
    let f_max = Fixed::from_int(1_000_000_000);
    let g = f(981, 100);
    let shallow = laws::hydrostatic_pressure(Fixed::from_int(998), g, Fixed::from_int(1), p_max);
    let deep = laws::hydrostatic_pressure(Fixed::from_int(998), g, Fixed::from_int(10), p_max);
    assert!(deep > shallow, "deeper water is higher pressure");
    let small = laws::buoyant_force(Fixed::from_int(998), g, Fixed::ONE, f_max);
    let big = laws::buoyant_force(Fixed::from_int(998), g, Fixed::from_int(5), f_max);
    assert!(big > small, "a larger displaced volume floats harder");
}

#[test]
fn extreme_velocity_routes_to_the_cap_rather_than_panicking() {
    // Regression (wave-2 audit): the Fixed::MIN velocity magnitude must route to the cap, not panic
    // on abs (i64::MIN negation). The kernels are total, never panicking on an out-of-domain input.
    let p_max = Fixed::from_int(100000);
    let f_max = Fixed::from_int(1_000_000_000);
    assert_eq!(
        laws::dynamic_pressure(Fixed::from_int(1), Fixed::MIN, p_max),
        p_max
    );
    assert_eq!(
        laws::drag_force(Fixed::ONE, Fixed::ONE, Fixed::ONE, Fixed::MIN, f_max),
        f_max
    );
    assert_eq!(
        laws::reynolds_number(Fixed::ONE, Fixed::MIN, Fixed::ONE, Fixed::ONE, f_max),
        f_max
    );
}

#[test]
fn reynolds_gates_the_regime_and_the_kernels_replay() {
    let re_max = Fixed::from_int(1_000_000_000);
    // Water pipe: rho 998, v 1, L 0.1, mu 1e-3 -> Re ~ 99800 (turbulent).
    let re = laws::reynolds_number(
        Fixed::from_int(998),
        Fixed::ONE,
        f(1, 10),
        f(1, 1000),
        re_max,
    )
    .to_f64_lossy();
    assert!(re > 2300.0, "a fast water pipe is turbulent, Re {re}");
    // Determinism: every kernel is a pure function of its inputs.
    let a = laws::speed_of_sound(
        Fixed::from_int(2200),
        Fixed::from_int(998),
        Fixed::from_int(100000),
    );
    let b = laws::speed_of_sound(
        Fixed::from_int(2200),
        Fixed::from_int(998),
        Fixed::from_int(100000),
    );
    assert_eq!(a, b, "the same inputs replay bit for bit");
}

// --- Acoustic channel physics (2026-07-03): frequency-squared absorption and quarter-wave tube
// resonance, and the non-steering divergence the perceptual-geometry read-out derives over. ---

#[test]
fn the_acoustic_laws_load_and_pass_the_graph_dimensional_check() {
    // Load proof: the two new laws bind their kernels, their monomials close on the produced axes
    // (alpha = beta*f^2 reduces to 1/m; c/L reduces to Hz), and the three acoustic axes are present,
    // so the dimensional-neutrality check at load accepts both. Both read only ground axes (a reserved
    // range is still a ground axis), so they derive to tier 1.
    let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    reg.extend(data_path("fluids_floor.toml")).unwrap();
    assert!(reg.law("law.acoustic_absorption").is_some());
    assert!(reg.law("law.tube_resonance").is_some());
    assert!(reg.axis("acoustic.absorption_reference").is_some());
    assert!(reg.axis("acoustic.resonator_length").is_some());
    assert!(reg.axis("acoustic.formant_frequency").is_some());
    assert_eq!(reg.derived_tier("law.acoustic_absorption"), Some(1));
    assert_eq!(reg.derived_tier("law.tube_resonance"), Some(1));
}

#[test]
fn a_wrong_absorption_dimension_fails_loud() {
    // The absorption monomial is checked, not asserted: wiring the reference to a plain 1/m axis (one
    // that drops the residual 1/Hz^2) makes beta*f^2 reduce to 1/(m*s^2), not 1/m, and the load rejects
    // it. The closed-monomial discipline: a checkable law is verified, never waived to Asserted.
    let toml = r#"
[[axis]]
id = "acoustic.frequency"
dimension = "0,0,-1,0"
scale = "Hz"
range_lo = "1"
range_hi = "200000"
real = "audition"

[[axis]]
id = "acoustic.bad_reference"
dimension = "-1,0,0,0"
scale = "1/m"
range_lo = "0"
range_hi = "1"
real = "a mis-dimensioned reference"

[[axis]]
id = "acoustic.absorption_coefficient"
dimension = "-1,0,0,0"
scale = "1/m"
range_lo = "0"
range_hi = "10"
real = "absorption"

[[law]]
id = "law.acoustic_absorption"
kernel = "acoustic_absorption"
ports = [
  { role = "reference", axis = "acoustic.bad_reference" },
  { role = "frequency", axis = "acoustic.frequency" },
]
produces = ["acoustic.absorption_coefficient"]
dimension = "-1,0,0,0"
"#;
    assert!(
        matches!(
            PhysicsRegistry::from_toml_str(toml).unwrap_err(),
            civsim_physics::PhysicsError::DimensionUnreachable { .. }
        ),
        "a reference that drops the 1/Hz^2 residual must fail the monomial check"
    );
}

#[test]
fn absorption_rises_with_frequency_and_saturates() {
    let alpha_max = Fixed::from_int(10);
    // A representative reference (scale-bridged; the true SI beta is sub-epsilon, its exact bridge the
    // reserved R-UNITS-PIN per-quantity scale), chosen so the audible band stays below the cap.
    let beta = f(1, 1000000);
    let low = laws::acoustic_absorption(beta, Fixed::from_int(100), alpha_max);
    let mid = laws::acoustic_absorption(beta, Fixed::from_int(200), alpha_max);
    let high = laws::acoustic_absorption(beta, Fixed::from_int(300), alpha_max);
    assert!(
        low < mid && mid < high,
        "absorption rises with frequency (f^2)"
    );
    assert!(
        mid.to_f64_lossy() > 3.5 * low.to_f64_lossy(),
        "doubling frequency quadruples absorption"
    );
    assert_eq!(
        laws::acoustic_absorption(beta, Fixed::ZERO, alpha_max),
        Fixed::ZERO,
        "zero frequency, no absorption"
    );
    assert_eq!(
        laws::acoustic_absorption(beta, Fixed::from_int(200000), alpha_max),
        alpha_max,
        "an ultrasonic frequency past the overflow-safe ceiling saturates, never wraps"
    );
    // A huge reference overflows the staged product and routes to the cap, never wraps.
    assert_eq!(
        laws::acoustic_absorption(
            Fixed::from_int(1_000_000),
            Fixed::from_int(40000),
            alpha_max
        ),
        alpha_max
    );
}

#[test]
fn a_small_reference_absorbs_finitely_above_the_old_frequency_ceiling() {
    // Regression (audit defect 16): there is no frequency-alone early return. A frequency above the
    // former sqrt(2^31) ~ 46340 ceiling used to route straight to the cap, over-saturating a medium
    // whose reference is small enough that reference*f*f is still representable. With a tiny reference
    // a high frequency now yields a FINITE absorption strictly below the cap, equal to the staged
    // product, rather than the cap.
    let alpha_max = Fixed::from_int(10);
    let beta = f(1, 1_000_000_000); // 1e-9, representable in Q32.32
    let freq = Fixed::from_int(50_000); // above the old 46340 ceiling
    let a = laws::acoustic_absorption(beta, freq, alpha_max);
    assert!(
        a > Fixed::ZERO && a < alpha_max,
        "a low-reference medium absorbs finitely above the old ceiling, not saturated: {a:?}"
    );
    // It equals the exact staged product beta*f*f (no early return truncated it).
    assert_eq!(a, beta.mul(freq).mul(freq));
}

#[test]
fn resonance_rises_with_speed_and_falls_with_length_and_hits_human_formants() {
    let freq_max = Fixed::from_int(100000);
    let l = f(17, 100); // 0.17 m, a neutral human vocal tract
    let air_c = Fixed::from_int(343);
    // HUMAN ROW: L = 0.17 m in air (c = 343 m/s) yields the schwa formant series near 500/1500/2500 Hz,
    // the closed-open quarter-wave odd-harmonic series (F1..F3 in the cardinal-vowel formant range).
    let f1 = laws::tube_resonance(Fixed::from_int(1), air_c, l, freq_max).to_f64_lossy();
    let f2 = laws::tube_resonance(Fixed::from_int(2), air_c, l, freq_max).to_f64_lossy();
    let f3 = laws::tube_resonance(Fixed::from_int(3), air_c, l, freq_max).to_f64_lossy();
    assert!(
        (250.0..800.0).contains(&f1),
        "F1 in the cardinal-vowel range, got {f1}"
    );
    assert!(
        (600.0..2300.0).contains(&f2),
        "F2 in the cardinal-vowel range, got {f2}"
    );
    assert!((2200.0..2800.0).contains(&f3), "F3 near 2500, got {f3}");
    assert!(f1 < f2 && f2 < f3, "the odd-harmonic series rises");
    // Rises with the sound speed, falls with the resonator length.
    let faster = laws::tube_resonance(Fixed::from_int(1), Fixed::from_int(1480), l, freq_max);
    let slower = laws::tube_resonance(Fixed::from_int(1), air_c, l, freq_max);
    assert!(faster > slower, "a faster medium resonates higher");
    let longer = laws::tube_resonance(Fixed::from_int(1), air_c, f(34, 100), freq_max);
    let shorter = laws::tube_resonance(Fixed::from_int(1), air_c, f(17, 100), freq_max);
    assert!(longer < shorter, "a longer tube resonates lower");
}

#[test]
fn tube_resonance_overflow_and_zero_guards() {
    let freq_max = Fixed::from_int(100000);
    let air_c = Fixed::from_int(343);
    // A near-zero length overflows the divide and reads the cap, never wraps.
    assert_eq!(
        laws::tube_resonance(Fixed::from_int(1), air_c, Fixed::from_bits(1), freq_max),
        freq_max,
        "a near-zero tube reads the cap"
    );
    assert_eq!(
        laws::tube_resonance(Fixed::from_int(1), air_c, Fixed::ZERO, freq_max),
        freq_max
    );
    // Zero sound speed reads zero (no medium, no resonance).
    assert_eq!(
        laws::tube_resonance(Fixed::from_int(1), Fixed::ZERO, f(17, 100), freq_max),
        Fixed::ZERO
    );
    // A non-positive mode has no resonance.
    assert_eq!(
        laws::tube_resonance(Fixed::ZERO, air_c, f(17, 100), freq_max),
        Fixed::ZERO
    );
}

#[test]
fn the_acoustic_kernels_are_deterministic() {
    let alpha_max = Fixed::from_int(10);
    let freq_max = Fixed::from_int(100000);
    let a = laws::acoustic_absorption(f(1, 1000000), Fixed::from_int(440), alpha_max);
    let b = laws::acoustic_absorption(f(1, 1000000), Fixed::from_int(440), alpha_max);
    assert_eq!(a, b, "absorption replays bit for bit");
    let r1 = laws::tube_resonance(
        Fixed::from_int(2),
        Fixed::from_int(343),
        f(17, 100),
        freq_max,
    );
    let r2 = laws::tube_resonance(
        Fixed::from_int(2),
        Fixed::from_int(343),
        f(17, 100),
        freq_max,
    );
    assert_eq!(r1, r2, "resonance replays bit for bit");
}

#[test]
fn two_media_diverge_in_formants_from_physics_alone_no_authored_table() {
    // The non-steering divergence (Principle 9): the SAME resonator length fed through tube_resonance
    // in two real media gives two different formant vectors, purely because each medium's sound speed
    // differs (c from law.speed_of_sound over its floor bulk_modulus and density). No per-race
    // confusability table, no RaceId: the derived geometry diverges from the channel physics alone.
    let c_max = Fixed::from_int(100000);
    let freq_max = Fixed::from_int(100000);
    let air = laws::speed_of_sound(f(142, 1000), f(1225, 1000), c_max); // ~340 m/s
    let water = laws::speed_of_sound(Fixed::from_int(2200), Fixed::from_int(998), c_max); // ~1480 m/s
    let l = f(17, 100);
    let air_f: Vec<Fixed> = (1..=3)
        .map(|n| laws::tube_resonance(Fixed::from_int(n), air, l, freq_max))
        .collect();
    let water_f: Vec<Fixed> = (1..=3)
        .map(|n| laws::tube_resonance(Fixed::from_int(n), water, l, freq_max))
        .collect();
    assert_ne!(
        air_f, water_f,
        "the same length in two media gives different formant vectors"
    );
    for (a, w) in air_f.iter().zip(&water_f) {
        assert!(
            w > a,
            "the faster medium raises every formant, monotone in c"
        );
    }
    // Neither vector is a hardcoded reference: the air F1 is the physics-derived schwa value, not a
    // round authored 500 Hz.
    assert_ne!(
        air_f[0],
        Fixed::from_int(500),
        "no authored reference table; the value is derived from physics"
    );
}
