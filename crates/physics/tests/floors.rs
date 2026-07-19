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

//! Build phase 3: the two consolidated floors loaded as data through the registry.
//!
//! These tests prove the wave-0 and wave-1 floors are expressible as data and load into
//! the [`PhysicsRegistry`] with every cross-reference resolving (a law reads only
//! defined axes, a substance carries values only on defined axes), the reserved ranges
//! forming the owner's review queue, and the content hash deterministic. The worked
//! physics proof (the mace-versus-morningstar strike, edibility as a relation) lives in
//! the law harness in `laws.rs`; this is the data half of the proof.

use civsim_physics::periodic::PeriodicTable;
use civsim_physics::{PhysicsRegistry, Provenance};

fn data_path(file: &str) -> String {
    format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
}

fn mechanical() -> PhysicsRegistry {
    PhysicsRegistry::load(data_path("mechanical_floor.toml"))
        .expect("the mechanical floor loads and every cross-reference resolves")
}

fn biology() -> PhysicsRegistry {
    PhysicsRegistry::load(data_path("biology_floor.toml"))
        .expect("the biology floor loads and every cross-reference resolves")
}

#[test]
fn the_mechanical_floor_loads_with_its_axes_laws_and_substances() {
    let reg = mechanical();
    // The unified registry: the shared mechanical, bulk-material, and energy-thermal
    // axes plus the shared gravitational axis, and the wave-1 law set.
    assert_eq!(reg.axis_count(), 39, "the mechanical-and-materials axes");
    assert_eq!(
        reg.law_count(),
        21,
        "the wave-1 interaction laws (law.impact split into kinetic_energy and impulse; law.sensible_rise surfaces the sensible_energy inverse)"
    );
    assert_eq!(
        reg.substance_count(),
        2,
        "the iron and oak example materials"
    );
}

#[test]
fn the_sensible_energy_inverse_is_reachable_as_its_own_law() {
    // The sensible_rise kernel (dT from a delivered energy) had no law entry, so the data-driven
    // graph could not reach it though its forward sibling law.sensible_heat could. It is now a
    // first-class law binding the kernel, and the loader accepts its contract on load.
    let reg = mechanical();
    let rise = reg
        .law("law.sensible_rise")
        .expect("the inverse law is present");
    assert_eq!(rise.kernel, "sensible_rise");
    // A leaf read over registry input axes (mass and specific heat), the delivered energy
    // caller-composed, so it derives at the base tier like its forward sibling.
    assert_eq!(
        reg.derived_tier("law.sensible_rise"),
        reg.derived_tier("law.sensible_heat")
    );
    // Its forward sibling is still present and distinct.
    assert_eq!(
        reg.law("law.sensible_heat").unwrap().kernel,
        "sensible_energy"
    );
}

#[test]
fn the_biology_floor_loads_with_its_axes_and_three_laws() {
    let reg = biology();
    assert_eq!(
        reg.axis_count(),
        22,
        "the composition, toxin, and consumer axes, the respiratory-surface and convective-surface axes, the two matter-cycle decomposition axes, plus the two derived score axes"
    );
    assert_eq!(reg.law_count(), 3, "net nutrition, harm, and edibility");
    // edibility reads the produced net-nutrition and net-harm scores, so it derives one tier
    // above the two folds: the biology floor now carries an internal composition edge.
    assert_eq!(reg.derived_tier("law.net_nutrition"), Some(1));
    assert_eq!(reg.derived_tier("law.harm"), Some(1));
    assert_eq!(
        reg.derived_tier("law.edibility"),
        Some(2),
        "edibility composes the two fold outputs, so it is tier 2"
    );
    assert_eq!(
        reg.substance_count(),
        0,
        "the floor carries no species; R-BIOSPHERE generates them over it"
    );
}

#[test]
fn the_pressure_class_axes_share_the_pinned_megapascal_scale() {
    let reg = mechanical();
    for id in [
        "mat.indentation_hardness",
        "mat.yield_strength",
        "mat.ultimate_tensile_strength",
        "mat.compressive_strength",
        "mat.shear_strength",
        "mat.fracture_strength",
        "mat.elastic_modulus",
    ] {
        let axis = reg.axis(id).unwrap_or_else(|| panic!("{id} exists"));
        assert_eq!(
            axis.scale_unit, "MPa",
            "{id} is on the pinned pressure scale"
        );
    }
}

#[test]
fn ranges_are_set_with_only_the_per_class_tolerance_reserved() {
    use civsim_core::Fixed;
    // Every mechanical axis range is now set (the geometry second moment of area ratified
    // 2026-07-03 on its per-quantity scale, R-UNITS-PIN); the biology floor keeps only the
    // per-toxin-class consumer reference tolerance reserved, its scale being per class.
    let mech = mechanical();
    assert!(
        mech.reserved_axis_ids().is_empty(),
        "every mechanical axis range is set, got reserved {:?}",
        mech.reserved_axis_ids()
    );
    // A set range reads back the cited bound exactly.
    let (lo, hi) = mech
        .axis("mat.density")
        .unwrap()
        .range
        .require("mat.density")
        .unwrap();
    // The low bound was widened to the lightest gas for wave 2 (owner-signed 2026-07-01) so
    // atmospheric buoyancy and wind are expressible; the high bound is unchanged.
    assert_eq!(lo, Fixed::from_ratio(8, 100));
    assert_eq!(hi, Fixed::from_int(23000));
    // The second moment of area now reads back its ratified window; its 1e-12 low bound underflows
    // the stored Fixed to zero (the picofarad analog), the true magnitude living in the declared
    // decimal envelope the per-quantity scale derives from.
    let (smoa_lo, smoa_hi) = mech
        .axis("mech.second_moment_of_area")
        .unwrap()
        .range
        .require("mech.second_moment_of_area")
        .unwrap();
    assert_eq!(
        smoa_lo,
        Fixed::ZERO,
        "the 1e-12 low bound underflows to zero"
    );
    assert_eq!(smoa_hi, Fixed::ONE, "the 1 m^4 high bound");

    let bio = biology();
    assert_eq!(
        bio.reserved_axis_ids(),
        vec!["bio.consumer.reference_tolerance"],
        "only the per-toxin-class reference tolerance stays reserved, its scale being per class (R-UNITS-PIN)"
    );
}

#[test]
fn a_real_substance_value_reads_exactly() {
    use civsim_core::Fixed;
    let reg = mechanical();
    let iron = reg.substance("iron").expect("iron is defined");
    assert_eq!(
        iron.vector.get("mat.density"),
        Some(&Fixed::from_int(7870)),
        "the cited iron density reads back exactly"
    );
}

#[test]
fn the_ground_floor_extends_the_mechanical_floor_with_cited_ground_substances() {
    use civsim_core::Fixed;
    // The world material registry is the mechanical floor plus the ground substances, built from the
    // crate's embedded data (no filesystem path), so the sim's world-build can obtain it directly.
    let reg = PhysicsRegistry::ground()
        .expect("the ground registry loads and every cross-reference resolves");
    // The mechanical floor's own substances survive the extension (the ground floor extends, not
    // replaces).
    assert!(
        reg.substance("iron").is_some(),
        "the mechanical floor's iron survives the ground extension"
    );
    // The ground substances load, each carrying its cited bulk density (the datum a carry weighs).
    for (id, density) in [
        ("granite", 2700),
        ("loam", 1400),
        ("clay", 1900),
        ("hematite", 5260),
        ("halite", 2170),
    ] {
        let s = reg
            .substance(id)
            .unwrap_or_else(|| panic!("{id} is a ground substance"));
        assert_eq!(
            s.vector.get("mat.density"),
            Some(&Fixed::from_int(density)),
            "the cited {id} density reads back exactly"
        );
    }
    // Granite carries the rock-mechanics axes the extraction contest reads (the fracture-gating
    // strength the gate flagged for item 4), so mining rock has a real target when it is wired.
    let granite = reg.substance("granite").unwrap();
    assert_eq!(
        granite.vector.get("mat.fracture_strength"),
        Some(&Fixed::from_int(15))
    );
    assert_eq!(
        granite.vector.get("mat.compressive_strength"),
        Some(&Fixed::from_int(200))
    );
    // The embedded build is deterministic across calls (the sorted-walk content hash).
    assert_eq!(
        reg.content_id(),
        PhysicsRegistry::ground().unwrap().content_id()
    );
}

#[test]
fn the_spent_hull_trace_substance_is_perceivable_and_weatherable() {
    use civsim_core::Fixed;
    // The physical-trace cultural-persistence substrate (the lifetime/demography keystone, pillar 2, trace
    // slice A): the spent hull the extract-and-eat technique leaves behind is a ground substance carrying
    // the physics a durable trace needs. It is HARD (a positive mat.fracture_strength, so a being can sense
    // it as fracturable matter) and WEATHERABLE (a mineral-ash fraction and a decomposition barrier, the two
    // axes the matter cycle gates on, so an unvisited trace fades over time). Authored environment physics
    // (Principle 9); deposited nowhere by default, so this is inert until the deposit hook (trace slice B).
    let reg = PhysicsRegistry::ground().expect("the ground registry loads");
    let hull = reg
        .substance("spent_hull")
        .expect("spent_hull is a ground substance");
    assert_eq!(
        hull.vector.get("mat.density"),
        Some(&Fixed::from_int(1200)),
        "the cited spent-hull density reads back exactly (a carry weighs it)"
    );
    // Perceivable: a positive fracture strength, so its FracturePotential reads positive (the only
    // MaterialField-read percept today, the interim until the per-substance trace percept lands).
    assert_eq!(
        hull.vector.get("mat.fracture_strength"),
        Some(&Fixed::from_int(12)),
        "the spent hull is hard enough to be sensed as fracturable matter"
    );
    // Weatherable: the two axes step_matter_cycle gates on, so the matter cycle fades the trace and an
    // unsupported residue does not persist forever (falsifiability by physics).
    assert!(
        hull.vector.contains_key("bio.mineral_ash_fraction"),
        "the spent hull carries the ash fraction the matter cycle needs to weather it"
    );
    assert_eq!(
        hull.vector.get("bio.decomposition_barrier"),
        Some(&Fixed::from_int(273)),
        "the spent hull weathers only above its freezing decomposition barrier"
    );
    // It carries NO energy: a hull is residue, never food (the kernel's energy stayed in the oilseed).
    assert!(
        !hull.vector.contains_key("bio.energy_density"),
        "the spent hull is inedible residue, not a food"
    );
}

#[test]
fn each_floor_hashes_deterministically_across_loads() {
    assert_eq!(mechanical().content_id(), mechanical().content_id());
    assert_eq!(biology().content_id(), biology().content_id());
    // The two floors are distinct content.
    assert_ne!(mechanical().content_id(), biology().content_id());
}

#[test]
fn every_law_input_resolves_to_a_defined_axis() {
    // A successful load already validates this, but assert it explicitly over both floors
    // so the guarantee is on record: no law reaches for an absent axis.
    for reg in [mechanical(), biology()] {
        for law in reg.laws() {
            for axis_id in &law.inputs {
                assert!(
                    reg.axis(axis_id).is_some(),
                    "law {} reads undefined axis {axis_id}",
                    law.id
                );
            }
        }
    }
}

// The provenance string a floor value carries, real-with-source or fantasy-reserved; both variants hold the
// ground for the value (the citation or the basis), which the born-provenance invariant requires non-empty.
fn provenance_ground(p: &Provenance) -> &str {
    match p {
        Provenance::RealWithSource(s) | Provenance::FantasyReserved(s) => s,
    }
}

#[test]
fn the_full_floor_is_born_provenance_tagged() {
    // The floor-provenance born gate (provenance register Phase 2, the floor loader): the sibling of the
    // calibration born-provenance test one register level down. Every VALUE entry the physics floor carries
    // (a QuantityAxis, a Substance, a periodic Element) declares a provenance, so a floor value can never
    // ship untagged. The Rust loader already enforces this fail-loud (`provenance_from` returns
    // `MissingProvenance` when an entry declares neither `real` nor `fantasy`), so `ground()` and
    // `PeriodicTable::standard()` panicking on a missing tag IS the enforcement; this test exercises it over
    // the real embedded floor and asserts each entry's provenance ground is non-empty (a degenerate
    // `RealWithSource("")` is as much a defect as an absent tag). Laws are derivations and carry no
    // provenance value (they are implicitly derived); phases derive their provenance from their constituent
    // substances (the seam-1 ruling), so neither is asserted here. Phase 2 refines these two tags into the
    // seven-tag register; this establishes the enforcement surface, byte-neutral. The counts are the current
    // floor totals (they grow with the world), asserted as non-empty lower bounds rather than exact so a new
    // floor entry does not break the invariant test.
    let reg = PhysicsRegistry::ground()
        .expect("the ground floor loads with every value born provenance-tagged");
    let mut axis_count = 0;
    for axis in reg.axes() {
        assert!(
            !provenance_ground(&axis.provenance).trim().is_empty(),
            "floor axis '{}' carries an empty provenance ground",
            axis.id
        );
        axis_count += 1;
    }
    let mut substance_count = 0;
    for sub in reg.substances() {
        assert!(
            !provenance_ground(&sub.provenance).trim().is_empty(),
            "floor substance '{}' carries an empty provenance ground",
            sub.id
        );
        substance_count += 1;
    }
    assert!(
        axis_count > 0 && substance_count > 0,
        "the ground floor is non-empty"
    );

    let table = PeriodicTable::standard()
        .expect("the periodic table loads with every element provenance-tagged");
    let mut element_count = 0;
    for element in table.elements() {
        assert!(
            !element.provenance.trim().is_empty(),
            "periodic element '{}' carries an empty provenance ground",
            element.symbol
        );
        element_count += 1;
    }
    assert!(element_count > 0, "the periodic table is non-empty");
}

#[test]
fn every_loaded_floor_entry_has_a_seven_tag_grade_in_the_register() {
    use civsim_physics::floor_provenance::FloorProvenance;
    // The floor grade register (Phase 2 slice 2) must stay in sync with the LOADED floor: every axis and
    // substance the ground registry carries, and every periodic element, has a seven-tag grade keyed by its
    // id. This is the cross-check the Python floor-provenance gate makes structurally, asserted here against
    // the real loaded structs so a new floor entry without a grade fails the build.
    let reg = FloorProvenance::embedded().expect("the floor grade register parses");
    let ground = PhysicsRegistry::ground().expect("the ground floor loads");
    let table = PeriodicTable::standard().expect("the periodic table loads");
    for axis in ground.axes() {
        assert!(
            reg.grade(&axis.id).is_some(),
            "floor axis '{}' has no grade in floor_provenance.toml",
            axis.id
        );
    }
    for sub in ground.substances() {
        assert!(
            reg.grade(&sub.id).is_some(),
            "floor substance '{}' has no grade in floor_provenance.toml",
            sub.id
        );
    }
    for element in table.elements() {
        assert!(
            reg.grade(&element.symbol).is_some(),
            "periodic element '{}' has no grade in floor_provenance.toml",
            element.symbol
        );
    }
    // The candidate phases (seam-1 reconciled): each carries a grade keyed "phase.<name>" (measured plus a
    // derive-first defect, its cited thermodynamic data being a measurement stored not derived). Keyed with
    // the "phase." prefix so a phase (hematite, Fe2O3) does not collide with a ground substance (hematite).
    let phases = civsim_physics::petrology_data::PhaseRegistry::standard()
        .expect("the phase registry loads");
    for phase in phases.phases() {
        let key = format!("phase.{}", phase.name);
        let grade = reg
            .grade(&key)
            .unwrap_or_else(|| panic!("phase '{key}' has no grade in floor_provenance.toml"));
        // MOVED WITH THE TRUTH, 2026-07-19, and this row is the clearest illustration of why. A
        // candidate phase DOES carry cited thermodynamic data: every registry row has a real citation in
        // its `source` field. What it does not carry is a MACHINE-RESOLVABLE source id, a claim locator,
        // a measurement regime or an uncertainty, so nothing here can tell a genuinely evidenced value
        // from a conventionally labelled one without a human reading prose. An audit found 244 such
        // labels across both registers with zero machine-linked evidence between them.
        //
        // These are therefore `unverified_measurement_candidate`, not because the literature is absent
        // but because the LINK is. Each resolves one of two equally good ways: promotion once its claim
        // record exists, or truthful downgrade. The assertion still pins the grade, so a silent drift to
        // some other tag still fails.
        assert_eq!(
            grade.grade, "unverified_measurement_candidate",
            "a candidate phase cites its thermodynamics in prose but carries no machine-resolvable \
             source id, so it is an unverified measurement candidate until that link exists"
        );
        assert!(
            grade.derive_first_defect,
            "a phase's stored properties should derive from constituents in the materials buildout"
        );
    }
}
