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

//! The composition-layer hardening (R-DEEPTECH-PHYSICS, tier-derivation stage): the law
//! dataflow descriptor, its load-time checks, and the derived tier. These prove the mechanism
//! on the migrated electricity-and-magnetism floor: the kernel binding is verified against the
//! contract (the naming convention the old schema could not check), a law names two ports on
//! one axis (the two-participant case the flat input list could not express), the produced
//! output dimension is verified reachable from the input axes, and a law's tier is derived as
//! its depth in the law-output graph, filling the middle the authored stamps left empty.

use civsim_physics::{PhysicsError, PhysicsRegistry};

fn data_path(file: &str) -> String {
    format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
}

fn full_registry() -> PhysicsRegistry {
    let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    reg.extend(data_path("fluids_floor.toml")).unwrap();
    reg.extend(data_path("chem_optics_floor.toml")).unwrap();
    reg.extend(data_path("em_floor.toml")).unwrap();
    reg
}

#[test]
fn the_em_chain_derives_an_increasing_tier_from_the_graph() {
    // solenoid_field reads ground axes and writes mag.flux_density; flux_linkage reads that and
    // writes mag.flux; faraday_emf reads that. So the authored flat "tier 2" resolves to a real
    // 1 -> 2 -> 3 depth once tier is derived from the produced-axis edges.
    let reg = full_registry();
    assert_eq!(
        reg.derived_tier("law.solenoid_field"),
        Some(1),
        "solenoid_field reads only ground axes, so it is tier 1"
    );
    assert_eq!(
        reg.derived_tier("law.flux_linkage"),
        Some(2),
        "flux_linkage reads solenoid_field's produced flux density, so it is tier 2"
    );
    assert_eq!(
        reg.derived_tier("law.faraday_emf"),
        Some(3),
        "faraday_emf reads flux_linkage's produced flux, so it is tier 3"
    );
}

#[test]
fn the_derived_layering_fills_the_middle_the_authored_stamps_left_empty() {
    // The authored data uses only tier 0 and tier 2; the derived tiers span a contiguous run
    // that includes tier 1, the level nothing was ever stamped with.
    let reg = full_registry();
    let tiers = reg.derived_tiers();
    let present: std::collections::BTreeSet<u32> = tiers.values().copied().collect();
    assert!(present.contains(&1), "tier 1 is populated by derivation");
    assert!(
        present.contains(&2),
        "tier 2 emerges from the first produced-axis edge"
    );
    assert!(present.contains(&3), "tier 3 emerges from the second edge");
}

/// A minimal single-floor registry: the axes an Ohm's-law test needs, plus a law bound to the
/// `ohm_voltage` kernel. The dimensions make `current * resistance` reduce to `voltage`.
fn ohm_toml(resistance_axis_for_role: &str, role_name: &str) -> String {
    format!(
        r#"
[[axis]]
id = "elec.current"
dimension = "current"
scale = "A"
range_lo = "0"
range_hi = "1000"
real = "SI ampere"

[[axis]]
id = "elec.resistance"
dimension = "2,1,-3,0,-2"
scale = "ohm"
range_lo = "0"
range_hi = "1000000"
real = "CRC resistivity tables"

[[axis]]
id = "elec.potential"
dimension = "voltage"
scale = "V"
range_lo = "0"
range_hi = "100000"
real = "SI volt"

[[law]]
id = "law.ohm_voltage"
kernel = "ohm_voltage"
ports = [
  {{ role = "current", axis = "elec.current" }},
  {{ role = "{role_name}", axis = "{resistance_axis_for_role}" }},
]
output_measure = "V = I*R"
dimension = "voltage"
interval_bound = "[0, V_MAX]"
"#
    )
}

#[test]
fn a_correct_binding_and_monomial_loads() {
    let reg = PhysicsRegistry::from_toml_str(&ohm_toml("elec.resistance", "resistance")).unwrap();
    assert_eq!(reg.law_count(), 1);
    // ohm_voltage reads only ground axes, so its derived tier is 1.
    assert_eq!(reg.derived_tier("law.ohm_voltage"), Some(1));
}

#[test]
fn a_wrong_role_fails_the_binding_check() {
    // The kernel contract needs a `resistance` role; declaring `resistor` instead is the
    // mislabelled binding the old naming convention could not catch.
    let err = PhysicsRegistry::from_toml_str(&ohm_toml("elec.resistance", "resistor")).unwrap_err();
    assert!(
        matches!(err, PhysicsError::PortContractMismatch { .. }),
        "a role the kernel does not take must fail loud, got {err:?}"
    );
}

#[test]
fn a_wrong_axis_fails_the_dimensional_check() {
    // Wiring the resistance role to the current axis makes the monomial current*current, which
    // is not voltage; the reachability check rejects it.
    let err = PhysicsRegistry::from_toml_str(&ohm_toml("elec.current", "resistance")).unwrap_err();
    assert!(
        matches!(err, PhysicsError::DimensionUnreachable { .. }),
        "an output unreachable from the wired inputs must fail loud, got {err:?}"
    );
}

#[test]
fn a_law_can_name_two_ports_on_one_axis() {
    // Coulomb's force reads the charge axis twice (q1 and q2), the two-participants-of-one-axis
    // case the flat input list could not express; the derived input set dedups to one axis.
    let toml = r#"
[[axis]]
id = "elec.charge"
dimension = "charge"
scale = "C"
range_lo = "0"
range_hi = "1000"
real = "SI coulomb"

[[axis]]
id = "mech.arm_length"
dimension = "length"
scale = "m"
range_lo = "0"
range_hi = "1000"
real = "geometry"

[[law]]
id = "law.coulomb_force"
kernel = "coulomb_force"
ports = [
  { role = "q1", axis = "elec.charge" },
  { role = "q2", axis = "elec.charge" },
  { role = "r", axis = "mech.arm_length" },
]
output_measure = "electrostatic force"
dimension = "force"
interval_bound = "[0, F_MAX]"
"#;
    let reg = PhysicsRegistry::from_toml_str(toml).unwrap();
    let law = reg.law("law.coulomb_force").unwrap();
    assert_eq!(
        law.ports.len(),
        3,
        "three ports, two of them on the charge axis"
    );
    // The derived input list dedups the two charge ports to one axis.
    assert_eq!(law.inputs, vec!["elec.charge", "mech.arm_length"]);
}

#[test]
fn an_unknown_kernel_binding_fails_loud() {
    let toml = r#"
[[axis]]
id = "elec.current"
dimension = "current"
scale = "A"
range_lo = "0"
range_hi = "1000"
real = "SI ampere"

[[law]]
id = "law.made_up"
kernel = "no_such_kernel"
ports = [ { role = "current", axis = "elec.current" } ]
dimension = "current"
"#;
    let err = PhysicsRegistry::from_toml_str(toml).unwrap_err();
    assert!(
        matches!(err, PhysicsError::UnknownKernel { .. }),
        "a law binding a kernel with no contract must fail loud, got {err:?}"
    );
}

#[test]
fn the_mechanical_floor_migrates_and_names_two_arms_on_one_axis() {
    // The mechanical floor is fully migrated: all 21 laws bind a kernel (the former law.impact
    // compound is now the monomial law.kinetic_energy and law.impulse, and law.sensible_rise
    // surfaces the sensible_energy inverse), and every contract passes the dimensional check at
    // load. law.lever names its two arms as distinct ports on the one arm-length axis, the
    // mechanical two-participant case; law.impulse names its two masses.
    let reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
    let bound = reg.laws().filter(|l| !l.kernel.is_empty()).count();
    assert_eq!(
        bound, 21,
        "all 21 mechanical laws are migrated, none legacy"
    );
    let lever = reg.law("law.lever").unwrap();
    let arm_ports: Vec<&str> = lever
        .ports
        .iter()
        .filter(|p| p.axis == "mech.arm_length")
        .map(|p| p.role.as_str())
        .collect();
    assert_eq!(
        arm_ports,
        vec!["effort_arm", "load_arm"],
        "the lever reads the arm-length axis as two distinct roles"
    );
    // law.impulse is the other two-participant case: the striker and target masses on the mass axis.
    let impulse = reg.law("law.impulse").expect("law.impulse migrated");
    let mass_roles: Vec<&str> = impulse
        .ports
        .iter()
        .filter(|p| p.axis == "mech.mass")
        .map(|p| p.role.as_str())
        .collect();
    assert_eq!(mass_roles, vec!["striker_mass", "target_mass"]);
    // The migrated mechanical laws read only ground axes (no produced-axis edges this pass), so
    // they derive to tier 1; the compound id is retired.
    assert_eq!(reg.derived_tier("law.contact_pressure"), Some(1));
    assert_eq!(reg.derived_tier("law.kinetic_energy"), Some(1));
    assert_eq!(reg.derived_tier("law.impulse"), Some(1));
    assert!(reg.law("law.impact").is_none(), "the compound is retired");
}

#[test]
fn the_chem_optics_floor_migrates_with_a_same_dimension_difference() {
    // The chem-and-optics floor extends the mechanical and fluids floors (it reads their axes), so
    // it loads onto the stack, not standalone. All 11 of its laws bind their kernel; law.reaction
    // reports an enthalpy difference verified SameAs its formation-enthalpy input (two ports on one
    // axis), and its derived input dedups the two enthalpy ports to one axis.
    let reg = full_registry();
    let chem = [
        "law.reaction",
        "law.corrosion",
        "law.carnot_limit",
        "law.dissolution",
        "law.radiant_emission",
        "law.wien_peak",
        "law.inverse_square_falloff",
        "law.interface_split",
        "law.optical_depth",
        "law.refractive_contrast",
        "law.radiative_equilibrium",
    ];
    for id in chem {
        assert!(!reg.law(id).unwrap().kernel.is_empty(), "{id} is migrated");
    }
    let reaction = reg.law("law.reaction").unwrap();
    let enthalpy_roles: Vec<&str> = reaction
        .ports
        .iter()
        .filter(|p| p.axis == "chem.formation_enthalpy")
        .map(|p| p.role.as_str())
        .collect();
    assert_eq!(enthalpy_roles, vec!["products_sum", "reactants_sum"]);
    assert_eq!(
        reaction
            .inputs
            .iter()
            .filter(|a| *a == "chem.formation_enthalpy")
            .count(),
        1,
        "the two enthalpy ports dedup to one derived input axis"
    );
}

#[test]
fn the_migrated_floor_still_hashes_deterministically() {
    // The descriptor fields fold into the content hash; the same data still hashes identically.
    assert_eq!(full_registry().content_id(), full_registry().content_id());
}

#[test]
fn the_flux_laws_declare_their_explicit_rate_dimension() {
    // Convention A (explicit rate): a flux-forming law declares the rate it computes. poiseuille is
    // now a verified Monomial (volume/time); the composed-difference fluxes declare power;
    // thermal_buoyancy declares acceleration,
    // verified SameAs its gravity port. Loading the floors already enforced each check; these lock
    // the declared dimensions so a regression to the old accumulated form fails.
    let reg = full_registry();
    let dim = |id: &str| {
        let d = reg
            .law(id)
            .unwrap_or_else(|| panic!("{id} is defined"))
            .output_dimension;
        (d.length, d.mass, d.time, d.temperature, d.current)
    };
    assert_eq!(dim("law.poiseuille_flow"), (3, 0, -1, 0, 0), "volume/time");
    assert_eq!(dim("law.convective_flux"), (2, 1, -3, 0, 0), "power");
    assert_eq!(
        dim("law.radiant_emission"),
        (2, 1, -3, 0, 0),
        "radiant power"
    );
    assert_eq!(
        dim("law.evaporation_rate"),
        (-2, 1, -1, 0, 0),
        "mass flux per area"
    );
    assert_eq!(
        dim("law.thermal_buoyancy"),
        (1, 0, -2, 0, 0),
        "acceleration"
    );
}

#[test]
fn poiseuille_now_fails_loud_on_a_wrong_output_dimension() {
    // The explicit-rate correction made poiseuille a Monomial, so its output is now enforced: the
    // old volume declaration (no time exponent) is rejected as unreachable from dp*r^4/(mu*L).
    let toml = r#"
[[axis]]
id = "fluid.driving_pressure"
dimension = "pressure"
scale = "Pa"
range_lo = "0"
range_hi = "1000"
real = "SI pascal"

[[axis]]
id = "fluid.channel_radius"
dimension = "length"
scale = "m"
range_lo = "0"
range_hi = "1000"
real = "geometry"

[[axis]]
id = "fluid.dynamic_viscosity"
dimension = "-1,1,-1,0"
scale = "Pa*s"
range_lo = "0"
range_hi = "1000"
real = "CRC viscosity tables"

[[axis]]
id = "mech.arm_length"
dimension = "length"
scale = "m"
range_lo = "0"
range_hi = "1000"
real = "geometry"

[[law]]
id = "law.poiseuille_flow"
kernel = "poiseuille_flow"
ports = [
  { role = "dp", axis = "fluid.driving_pressure" },
  { role = "radius", axis = "fluid.channel_radius" },
  { role = "viscosity", axis = "fluid.dynamic_viscosity" },
  { role = "length", axis = "mech.arm_length" },
]
output_measure = "flow"
dimension = "volume"
interval_bound = "[0, Q_MAX]"
"#;
    let err = PhysicsRegistry::from_toml_str(toml).unwrap_err();
    assert!(
        matches!(err, PhysicsError::DimensionUnreachable { .. }),
        "the old volume declaration is now rejected, got {err:?}"
    );
}
