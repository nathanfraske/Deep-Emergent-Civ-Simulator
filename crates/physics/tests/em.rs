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

//! The wave-3 electricity-and-magnetism floor (R-PHYS-W3, design Part 41, Part 55). It loads onto the
//! mechanical, fluids, and chem/optics floors, its axes carry the fifth base dimension (electric
//! current), every earlier axis is unchanged at current exponent zero (neutrality-by-construction
//! over five bases), the volt promotion of chem.standard_potential holds, and its closed-form kernels
//! compute physical, deterministic values.

use civsim_core::Fixed;
use civsim_physics::{laws, Dimension, PhysicsRegistry};

fn data_path(file: &str) -> String {
    format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
}
fn f(n: i64, d: i64) -> Fixed {
    Fixed::from_ratio(n, d)
}
fn full_registry() -> PhysicsRegistry {
    let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    reg.extend(data_path("fluids_floor.toml")).unwrap();
    reg.extend(data_path("chem_optics_floor.toml")).unwrap();
    reg.extend(data_path("em_floor.toml")).unwrap();
    reg
}

#[test]
fn the_em_floor_loads_onto_the_earlier_floors() {
    let reg = full_registry();
    // 38 mech + 15 fluids + 14 chem/optics + 14 em axes; 19 + 15 + 11 + 15 laws; 2 + 2 + 0 + 2 subs.
    assert_eq!(reg.axis_count(), 81, "the four floors' axes together");
    assert_eq!(reg.law_count(), 60, "the four floors' laws together");
    assert_eq!(reg.substance_count(), 6, "iron, oak, air, water, copper, lodestone");
    assert!(reg.law("law.faraday_emf").is_some());
    assert!(reg.axis("mag.flux").is_some());
    assert!(reg.substance("copper").is_some());
}

#[test]
fn the_fifth_base_is_backward_compatible_and_the_volt_promotion_holds() {
    let reg = full_registry();
    // Electricity carries the fifth base; earlier axes are unchanged at current exponent zero.
    assert_eq!(reg.axis("elec.current").unwrap().dimension, Dimension::CURRENT);
    assert_eq!(reg.axis("elec.charge").unwrap().dimension, Dimension::CHARGE);
    assert_eq!(reg.axis("mech.contact_area").unwrap().dimension.current, 0, "an earlier axis is unchanged");
    assert_eq!(reg.axis("mat.density").unwrap().dimension.current, 0);
    // The volt promotion: chem.standard_potential now shares the volt with elec.potential and elec.emf.
    assert_eq!(reg.axis("chem.standard_potential").unwrap().dimension, Dimension::VOLTAGE);
    assert_eq!(reg.axis("elec.potential").unwrap().dimension, Dimension::VOLTAGE);
    assert_eq!(reg.axis("elec.emf").unwrap().dimension, Dimension::VOLTAGE);
}

#[test]
fn ohm_and_the_circuit_extremes() {
    let v_max = Fixed::from_int(100_000_000);
    let i_max = Fixed::from_int(100_000);
    assert_eq!(laws::ohm_voltage(Fixed::from_int(2), Fixed::from_int(3), v_max), Fixed::from_int(6));
    // A 12 V source across 4 Ohm gives 3 A; a zero-resistance path is a short (the cap).
    assert_eq!(laws::circuit_current(Fixed::from_int(12), Fixed::from_int(4), i_max), Fixed::from_int(3));
    assert_eq!(laws::circuit_current(Fixed::from_int(12), Fixed::ZERO, i_max), i_max, "a short routes to the cap");
    // Resistance is geometric: a thin long wire resists more; a vanishing section is an open.
    let r_max = Fixed::from_int(1_000_000_000);
    let thin = laws::resistance(f(1, 100), Fixed::from_int(10), f(1, 100), r_max).to_f64_lossy();
    let thick = laws::resistance(f(1, 100), Fixed::from_int(10), Fixed::ONE, r_max).to_f64_lossy();
    assert!(thin > thick, "a thinner conductor resists more");
    assert_eq!(laws::resistance(f(1, 100), Fixed::from_int(10), Fixed::ZERO, r_max), r_max, "an open");
}

#[test]
fn the_daniell_cell_emf_and_coulomb_sign() {
    // Copper cathode +0.34 V, zinc anode -0.76 V: a Daniell cell is ~1.10 V.
    let emf = laws::battery_emf(f(34, 100), f(-76, 100)).to_f64_lossy();
    assert!((1.05..1.15).contains(&emf), "Daniell cell ~1.10 V, got {emf}");
    // Coulomb: like charges repel, unlike attract; nearer is stronger.
    let f_max = Fixed::from_int(1_000_000_000);
    let k = Fixed::from_int(9); // a stand-in coefficient on its reserved scale
    let (near, rep) = laws::coulomb_force(Fixed::from_int(2), Fixed::from_int(3), Fixed::ONE, k, f_max);
    let (far, _) = laws::coulomb_force(Fixed::from_int(2), Fixed::from_int(3), Fixed::from_int(2), k, f_max);
    assert!(rep, "like charges repel");
    assert!(near.to_f64_lossy() > far.to_f64_lossy(), "nearer charges push harder");
    let (_, attract) = laws::coulomb_force(Fixed::from_int(2), Fixed::from_int(-3), Fixed::ONE, k, f_max);
    assert!(!attract, "unlike charges attract");
}

#[test]
fn the_solenoid_field_and_the_motor_force() {
    let b_max = Fixed::from_int(1000);
    let f_max = Fixed::from_int(1_000_000_000);
    let mu_0 = f(12566371, 10_000_000_000_000); // 1.2566371e-6
    // Iron-core solenoid, mu_r 5000, n 1000 turns/m, I 1 A -> ~6.28 T.
    let b = laws::solenoid_field(Fixed::from_int(5000), Fixed::ONE, Fixed::from_int(1000), mu_0, b_max).to_f64_lossy();
    assert!((5.5..7.0).contains(&b), "iron solenoid ~6.28 T, got {b}");
    // Motor force F = B*I*L.
    assert_eq!(laws::motor_force(Fixed::from_int(2), Fixed::from_int(3), f(1, 2), f_max), Fixed::from_int(3));
    // Dipole torque tau = m*B (maximum), and flux linkage Phi = B*A.
    let tau = laws::dipole_torque(f(1, 10), Fixed::from_int(2), Fixed::from_int(1000)).to_f64_lossy();
    assert!((0.19..0.21).contains(&tau), "m*B ~0.2, got {tau}");
    assert_eq!(laws::flux_linkage(Fixed::from_int(2), f(1, 2), Fixed::from_int(1000)), Fixed::ONE);
}

#[test]
fn induction_is_a_signed_per_tick_delta_opposing_the_change() {
    let v_max = Fixed::from_int(100_000_000);
    let dt = f(1, 100); // 0.01 s tick
    // Rising flux (0.01 -> 0.02 Wb) over 1000 turns: EMF magnitude 1000 V, sign negative (Lenz opposes).
    let emf = laws::faraday_emf(f(2, 100), f(1, 100), Fixed::from_int(1000), dt, v_max);
    assert!(emf < Fixed::ZERO, "a rising flux induces an opposing (negative) EMF");
    assert!((900.0..1100.0).contains(&emf.to_f64_lossy().abs()), "~1000 V, got {}", emf.to_f64_lossy());
    // A falling flux flips the sign.
    let rev = laws::faraday_emf(f(1, 100), f(2, 100), Fixed::from_int(1000), dt, v_max);
    assert!(rev > Fixed::ZERO, "a falling flux induces the opposite sign");
    // Inductive back-EMF opposes a rising current; and the two energy stores mirror.
    let back = laws::inductive_emf(Fixed::from_int(2), Fixed::from_int(3), Fixed::ONE, dt, v_max);
    assert!(back < Fixed::ZERO, "a rising current makes a back-EMF");
    assert_eq!(laws::capacitor_energy(Fixed::from_int(2), Fixed::from_int(3), Fixed::from_int(1_000_000)), Fixed::from_int(9));
    assert_eq!(laws::inductor_energy(Fixed::from_int(2), Fixed::from_int(3), Fixed::from_int(46340), Fixed::from_int(1_000_000)), Fixed::from_int(9));
    // Determinism: a pure function of its inputs.
    let a = laws::faraday_emf(f(2, 100), f(1, 100), Fixed::from_int(1000), dt, v_max);
    assert_eq!(a, emf, "the same inputs replay bit for bit");
}

#[test]
fn the_electrical_kernels_are_total_on_adversarial_inputs() {
    // The fifth-base kernels are total: extreme velocity or charge routes to a cap, never panics.
    let f_max = Fixed::from_int(1_000_000_000);
    assert_eq!(laws::lorentz_force(Fixed::MIN, Fixed::MIN, Fixed::MIN, f_max), f_max);
    let (mag, _) = laws::coulomb_force(Fixed::MAX, Fixed::MAX, f(1, 1000), Fixed::MAX, f_max);
    assert_eq!(mag, f_max, "an overflowing Coulomb product routes to the cap");
}
