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

//! The freezer consistency twin (gate-directed, #187): the self-diffusion barrier `E*` computed two ways must
//! agree within the per-class scatter. Form B is `E* = f * E_coh` (the built barrier); Form A is
//! `E* = g * R * T_m` over the DERIVED Lindemann `T_m`. Since `g = k * f` with `k = E_coh/(R*T_m)` the
//! cohesive-to-melting ratio, the two are the same barrier expressed via `E_coh` or via `T_m`, and their
//! agreement reduces to `k` being class-constant (`E_coh = k * R * T_m`). This twin validates the derived `T_m`
//! against the barrier and confirms the derive-first collapse (Form B) reproduces the g-framework (Form A).
//!
//! Sited in a test file per the twin-siting rule. It uses a float boundary read for the ratio tolerance, which
//! is sanctioned in a test file; the mechanism it exercises is integer-only. The reserved constants (the
//! vacancy fraction `f`, the Lindemann ratio `delta`) are test-only fixtures at their basis values, never
//! canonical entries; `R` is the derived molar gas constant.

use civsim_core::Fixed;
use civsim_materials::freezer::{diffusion_barrier, FreezerRoute};
use civsim_materials::metallic::MetallicRoute;
use civsim_physics::metal_eos::MetalEosAnchors;
use civsim_physics::periodic::PeriodicTable;

fn r_kj_per_mol_k() -> Fixed {
    Fixed::from_ratio(8314, 1_000_000) // R = 8.314e-3 kJ/(mol K), derived (N_A*k_B)
}
fn delta_fixture() -> Fixed {
    Fixed::from_ratio(86, 1000) // the Lindemann ratio delta ~ 0.086 (test-only)
}
fn f_fixture() -> Fixed {
    Fixed::from_ratio(55, 100) // the vacancy fraction f ~ 0.55 (test-only)
}

#[test]
fn the_barrier_forms_agree_within_a_bonding_class_and_diverge_across_classes() {
    let table = PeriodicTable::standard().expect("periodic table");
    let anchors = MetalEosAnchors::standard().expect("metal EOS anchors");
    let metallic = MetallicRoute::new(&table, &anchors);
    let freezer = FreezerRoute::new(&metallic, &anchors);

    // The cohesive-to-melting ratio k = E_coh / (R * T_m_derived) for an anchored metal, from the DERIVED
    // Lindemann T_m. Form A (g*R*T_m) and Form B (f*E_coh) agree exactly when g = k*f, so their agreement
    // within a class is exactly k being class-constant.
    let k_ratio = |symbol: &str| -> f64 {
        let e_coh = metallic.cohesive_energy(symbol).expect("E_coh");
        let t_m = freezer.melting_point(symbol, delta_fixture()).expect("T_m");
        let rt = r_kj_per_mol_k().checked_mul(t_m).expect("R*T_m");
        e_coh.to_f64_lossy() / rt.to_f64_lossy()
    };

    // Within the transition-metal class (Fe, Ti), k is close: the cohesive-to-melting ratio is class-constant,
    // so one per-class g = k*f serves the class. This is the twin holding.
    let k_fe = k_ratio("Fe");
    let k_ti = k_ratio("Ti");
    assert!(
        (k_fe / k_ti - 1.0).abs() < 0.20,
        "the transition-class cohesive-to-melting ratio is ~constant: Fe k={k_fe}, Ti k={k_ti}"
    );

    // The literal twin, over the transition class: with g = k*f for that class, Form A (g*R*T_m) and Form B
    // (f*E_coh) reproduce the same barrier within the class scatter, validating the derived T_m against the
    // barrier. g is formed from the class k (not hand-picked), so no value is planted.
    let g_transition = Fixed::from_ratio((k_fe * 100.0).round() as i64, 100)
        .checked_mul(f_fixture())
        .expect("g = k*f");
    for symbol in ["Fe", "Ti"] {
        let e_coh = metallic.cohesive_energy(symbol).expect("E_coh");
        let t_m = freezer.melting_point(symbol, delta_fixture()).expect("T_m");
        let form_b = diffusion_barrier(e_coh, f_fixture());
        let form_a = g_transition
            .checked_mul(r_kj_per_mol_k())
            .and_then(|x| x.checked_mul(t_m))
            .expect("g*R*T_m");
        let ratio = form_a.to_f64_lossy() / form_b.to_f64_lossy();
        assert!(
            (ratio - 1.0).abs() < 0.20,
            "{symbol}: g*R*T_m ({form_a:?}) and f*E_coh ({form_b:?}) agree within the transition-class scatter"
        );
    }

    // The per-class NECESSITY: an alkaline-earth metal (Mg) has a markedly different cohesive-to-melting ratio,
    // so the transition-class k over- or under-states its barrier well beyond the intra-class scatter. This is
    // the "real but partial" residual made concrete: the anchored set spans alkali, alkaline-earth, and
    // transition metals, so g (and delta, and f) are genuinely per-class, not one authored universal.
    let k_mg = k_ratio("Mg");
    assert!(
        (k_fe / k_mg - 1.0).abs() > 0.20,
        "a different bonding class carries a different cohesive-to-melting ratio: Fe k={k_fe}, Mg k={k_mg}"
    );
}
