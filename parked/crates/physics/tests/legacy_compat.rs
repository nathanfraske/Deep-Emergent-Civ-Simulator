// Copyright 2026 Nathan M. Fraske
// Licensed under the Apache License, Version 2.0; see LICENSE.

use civsim_core::Fixed;
use civsim_physics_legacy::floor_provenance::FloorProvenance;
use civsim_physics_legacy::laws::*;
use civsim_physics_legacy::{Fold, PhysicsError, PhysicsRegistry};

fn biology() -> PhysicsRegistry {
    PhysicsRegistry::from_toml_str(include_str!("../data/biology_floor.toml"))
        .expect("the parked biology floor loads")
}

#[test]
fn the_parked_floor_and_former_combined_ground_registry_remain_available() {
    let biology = biology();
    assert_eq!(biology.axis_count(), 22);
    assert_eq!(biology.law_count(), 3);
    assert_eq!(biology.substance_count(), 0);
    assert_eq!(biology.derived_tier("law.net_nutrition"), Some(1));
    assert_eq!(biology.derived_tier("law.harm"), Some(1));
    assert_eq!(biology.derived_tier("law.edibility"), Some(2));

    let ground = PhysicsRegistry::ground().expect("the former combined registry loads");
    assert_eq!(ground.axis_count(), 124);
    assert_eq!(ground.law_count(), 76);
    assert_eq!(ground.substance_count(), 19);
    for id in [
        "mech.stroke_length",
        "fluid.respirable_content",
        "fluid.gas_transfer_coefficient",
        "acoustic.frequency",
        "acoustic.source_power",
        "acoustic.absorption_coefficient",
        "acoustic.absorption_reference",
        "acoustic.resonator_length",
        "acoustic.formant_frequency",
        "opt.emissivity.band_0",
        "opt.emissivity.band_1",
        "opt.emissivity.band_2",
        "opt.spectral_band",
    ] {
        assert!(ground.axis(id).is_some(), "{id} remains available here");
    }
    for id in [
        "law.membrane_gas_flux",
        "law.acoustic_absorption",
        "law.tube_resonance",
    ] {
        assert!(ground.law(id).is_some(), "{id} remains available here");
    }
    assert!(
        ground.substance("oak").is_some(),
        "oak remains available here"
    );
    for id in ["carrion", "oilseed", "spent_hull", "tuber"] {
        assert!(
            ground.substance(id).is_some(),
            "{id} remains available here"
        );
    }
}

#[test]
fn the_frozen_legacy_provenance_register_preserves_its_old_surface() {
    let provenance = FloorProvenance::embedded().expect("the frozen register parses");
    assert_eq!(provenance.grades.len(), 243);
    assert_eq!(
        provenance.authoring_surface(),
        vec![
            "bio.consumer.hill_exponent",
            "bio.decomposition_rate",
            "bio.net_harm",
            "chem.corrosion_susceptibility",
            "chem.solute_affinity",
            "opt.spectral_band",
        ]
    );
    assert_eq!(provenance.derive_first_defects().len(), 22);
}

#[test]
fn biology_class_set_contracts_still_validate_in_the_compatibility_registry() {
    let registry = biology();
    let nutrition = registry.law("law.net_nutrition").unwrap();
    let supply = nutrition
        .ports
        .iter()
        .find(|port| port.role == "supply")
        .unwrap();
    assert_eq!(supply.members.len(), 3);
    assert!(supply.axis.is_empty());
    assert_eq!(supply.fold, Some(Fold::Min));

    let malformed = r#"
[[axis]]
id = "test.fraction"
dimension = "dimensionless"
scale = "1"
range_lo = "0"
range_hi = "1"
real = "test"

[[law]]
id = "law.bad_single"
kernel = "net_nutrition"
ports = [
  { role = "supply", axis = "test.fraction" },
  { role = "requirement", axis = "test.fraction" },
  { role = "assimilation", axis = "test.fraction" },
  { role = "fermentation", axis = "test.fraction" },
]
dimension = "dimensionless"
"#;
    let error = PhysicsRegistry::from_toml_str(malformed).unwrap_err();
    assert!(matches!(error, PhysicsError::PortContractMismatch { .. }));

    let mixed_dimensions = r#"
[[axis]]
id = "test.fraction"
dimension = "dimensionless"
scale = "1"
range_lo = "0"
range_hi = "1"
real = "test"

[[axis]]
id = "test.length"
dimension = "length"
scale = "m"
range_lo = "0"
range_hi = "1"
real = "test"

[[law]]
id = "law.bad_fold"
kernel = "net_nutrition"
ports = [
  { role = "supply", members = ["test.fraction", "test.length"], fold = "min" },
  { role = "requirement", axis = "test.fraction" },
  { role = "assimilation", axis = "test.fraction" },
  { role = "fermentation", axis = "test.fraction" },
]
dimension = "dimensionless"
"#;
    let error = PhysicsRegistry::from_toml_str(mixed_dimensions).unwrap_err();
    assert!(matches!(error, PhysicsError::BadPort { .. }));

    let contradictory_port = r#"
[[axis]]
id = "test.fraction"
dimension = "dimensionless"
scale = "1"
range_lo = "0"
range_hi = "1"
real = "test"

[[law]]
id = "law.bad_both"
kernel = "net_nutrition"
ports = [
  { role = "supply", axis = "test.fraction", members = ["test.fraction"], fold = "min" },
  { role = "requirement", axis = "test.fraction" },
  { role = "assimilation", axis = "test.fraction" },
  { role = "fermentation", axis = "test.fraction" },
]
dimension = "dimensionless"
"#;
    let error = PhysicsRegistry::from_toml_str(contradictory_port).unwrap_err();
    assert!(matches!(error, PhysicsError::BadPort { .. }));
}

#[test]
fn nutrition_harm_and_edibility_preserve_the_retired_extreme_behavior() {
    let half = Fixed::from_ratio(1, 2);
    let classes = [
        (Fixed::ONE, Fixed::ONE, Some(Fixed::ONE)),
        (half, Fixed::ONE, Some(Fixed::ONE)),
        (Fixed::ONE, Fixed::ONE, None),
    ];
    assert_eq!(net_nutrition(&classes), half);
    assert_eq!(
        satisfaction(Fixed::MAX, Fixed::MAX, Some(Fixed::ONE)),
        Fixed::ONE
    );
    assert_eq!(
        satisfaction(Fixed::ONE, Fixed::ONE, Some(Fixed::from_bits(1))),
        Fixed::ONE
    );

    let harm_cap = Fixed::ONE;
    let dose = Fixed::from_int(38_000);
    let tolerance = Fixed::from_decimal_str("0.000001").unwrap();
    assert_eq!(harm_class(dose, Some(tolerance), 3, harm_cap), harm_cap);
    assert!(harm_class(Fixed::from_int(1_290), Some(Fixed::ONE), 3, harm_cap) < harm_cap);
    assert_eq!(
        harm_class(Fixed::from_int(1_291), Some(Fixed::ONE), 3, harm_cap),
        harm_cap
    );
    assert_eq!(
        harm_class(Fixed::from_int(5), None, 2, harm_cap),
        Fixed::ZERO
    );
    assert_eq!(
        harm_class(Fixed::from_int(5), Some(Fixed::ZERO), 2, harm_cap),
        harm_cap
    );

    let harm_classes = [
        (Fixed::from_int(2), Some(Fixed::ONE), 2),
        (Fixed::from_int(3), Some(Fixed::from_int(2)), 1),
    ];
    let mut reversed = harm_classes;
    reversed.reverse();
    assert_eq!(
        net_harm(&harm_classes, harm_cap, Fixed::from_int(10)),
        net_harm(&reversed, harm_cap, Fixed::from_int(10))
    );

    let margin_cap = Fixed::from_int(1_000_000);
    let result = edibility(
        Fixed::ONE,
        Fixed::ZERO,
        Fixed::from_int(5_000),
        tolerance,
        margin_cap,
    );
    assert_eq!(result.margin, margin_cap);
}

#[test]
fn retired_sensory_response_and_discrimination_families_are_stable() {
    let activation_cap = Fixed::from_int(1_000_000);
    let magnitude = Fixed::from_int(16);
    let gain = Fixed::from_int(2);
    assert_eq!(
        transduce(
            magnitude,
            ResponseLaw::Linear,
            gain,
            Fixed::from_int(3),
            activation_cap
        ),
        magnitude.checked_mul(gain).unwrap()
    );
    for law in [
        ResponseLaw::Linear,
        ResponseLaw::Power,
        ResponseLaw::LogCompressive,
    ] {
        assert_eq!(
            transduce(
                Fixed::ZERO,
                law,
                gain,
                Fixed::from_ratio(1, 2),
                activation_cap
            ),
            Fixed::ZERO
        );
        assert!(
            transduce(
                Fixed::from_int(8),
                law,
                gain,
                Fixed::from_ratio(1, 2),
                activation_cap
            ) > transduce(
                Fixed::from_int(2),
                law,
                gain,
                Fixed::from_ratio(1, 2),
                activation_cap
            )
        );
    }

    let step = Fixed::from_ratio(1, 4);
    let value = Fixed::from_ratio(9, 4);
    assert_eq!(
        discriminate(value, DiscriminationLaw::AbsoluteStep, step),
        value.checked_div(step).unwrap().to_int() as i64
    );
    assert_eq!(
        discriminate(Fixed::ZERO, DiscriminationLaw::WeberRelative, step),
        0
    );
}

#[test]
fn retired_metabolism_and_uptake_bridges_preserve_their_limits() {
    let large_cap = Fixed::from_int(1_000_000);
    assert_eq!(
        basal_metabolic_rate(Fixed::from_int(16), Fixed::ONE, large_cap),
        Fixed::from_int(8)
    );
    assert_eq!(
        basal_metabolic_rate(Fixed::from_int(256), Fixed::ONE, large_cap),
        Fixed::from_int(64)
    );

    let sigma_scale = 55;
    let sigma_bits = civsim_units::bignum::BigRat::from_decimal_str("0.0000000567")
        .unwrap()
        .round_to_scale(sigma_scale)
        .unwrap() as i64;
    assert_eq!(
        resting_heat_loss(
            Fixed::from_ratio(1, 10),
            Fixed::from_int(2),
            Fixed::from_int(310),
            Fixed::from_int(310),
            Fixed::from_ratio(95, 100),
            sigma_bits,
            sigma_scale,
            large_cap,
        ),
        Fixed::ZERO
    );
    assert_eq!(
        metabolic_drain_fraction(
            Fixed::from_int(10),
            Fixed::ZERO,
            Fixed::from_int(100),
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ONE,
        ),
        Fixed::from_ratio(1, 10)
    );

    let common = (
        Fixed::from_int(2),
        Fixed::ONE,
        Fixed::ONE,
        Fixed::from_ratio(257, 10_000),
        Fixed::ONE,
        Fixed::ONE,
    );
    let forward = reversible_uptake_flux(
        Fixed::from_int(100),
        common.0,
        common.1,
        common.2,
        Fixed::from_ratio(8, 10),
        common.3,
        common.4,
        common.5,
    );
    assert!(forward > Fixed::ZERO);
    assert_eq!(
        reversible_uptake_flux(
            Fixed::from_int(100),
            common.0,
            common.1,
            common.2,
            Fixed::ZERO,
            common.3,
            common.4,
            common.5,
        ),
        Fixed::ZERO
    );
    let tiny_stock = Fixed::from_ratio(1, 100);
    assert!(
        reversible_uptake_flux(
            tiny_stock,
            common.0,
            common.1,
            common.2,
            Fixed::from_ratio(8, 10),
            common.3,
            common.4,
            common.5,
        ) <= tiny_stock
    );
}

#[test]
fn retired_language_cost_and_tilt_preserve_their_bounds() {
    let cap = Fixed::ONE;
    let memory = Fixed::from_int(4);
    assert_eq!(parse_cost(Fixed::ZERO, memory, cap), Fixed::ZERO);
    assert_eq!(parse_cost(memory, memory, cap), Fixed::from_ratio(1, 2));
    assert!(
        parse_cost(Fixed::from_int(8), memory, cap) > parse_cost(Fixed::from_int(2), memory, cap)
    );
    assert!(
        parse_cost(Fixed::from_int(4), Fixed::from_int(16), cap)
            < parse_cost(Fixed::from_int(4), Fixed::ONE, cap)
    );

    let tilt_cap = Fixed::from_int(32);
    assert_eq!(
        harmony_tilt(Fixed::ZERO, Fixed::from_ratio(1, 10), tilt_cap),
        Fixed::ONE
    );
    assert_eq!(
        harmony_tilt(Fixed::from_int(100), Fixed::from_ratio(1, 1_000), tilt_cap),
        tilt_cap
    );
    assert_eq!(
        harmony_tilt(Fixed::from_ratio(1, 4), Fixed::ZERO, tilt_cap),
        tilt_cap
    );
}
