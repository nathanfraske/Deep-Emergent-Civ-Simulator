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

//! The consolidated abiotic floors loaded as data through the registry.
//!
//! These tests prove the active floors are expressible as data and load into the
//! [`PhysicsRegistry`] with every cross-reference resolving (a law reads only
//! defined axes, a substance carries values only on defined axes), the reserved ranges
//! forming the owner's review queue, and the content hash deterministic. The worked
//! mechanics proofs live in the law harness in `laws.rs`; this is the data half.

use civsim_physics::periodic::PeriodicTable;
use civsim_physics::{PhysicsRegistry, Provenance};

fn data_path(file: &str) -> String {
    format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
}

fn mechanical() -> PhysicsRegistry {
    PhysicsRegistry::load(data_path("mechanical_floor.toml"))
        .expect("the mechanical floor loads and every cross-reference resolves")
}

#[test]
fn the_mechanical_floor_loads_with_its_axes_laws_and_substances() {
    let reg = mechanical();
    // The unified registry: the shared mechanical, bulk-material, and energy-thermal
    // axes plus the shared gravitational axis, and the wave-1 law set.
    assert_eq!(
        reg.axis_count(),
        38,
        "the active mechanical-and-materials axes"
    );
    assert_eq!(
        reg.law_count(),
        21,
        "the wave-1 interaction laws (law.impact split into kinetic_energy and impulse; law.sensible_rise surfaces the sensible_energy inverse)"
    );
    assert_eq!(
        reg.substance_count(),
        1,
        "the retained iron reference material"
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
fn mechanical_ranges_are_set() {
    use civsim_core::Fixed;
    // Every mechanical axis range is set, including the geometry second moment of area
    // ratified 2026-07-03 on its per-quantity scale (R-UNITS-PIN).
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
fn the_active_ground_registry_is_abiotic_only() {
    let reg = PhysicsRegistry::ground().expect("the abiotic ground registry loads");
    assert_eq!(reg.axis_count(), 89);
    assert_eq!(reg.law_count(), 70);
    assert_eq!(reg.substance_count(), 14);
    assert!(reg.axes().all(|axis| !axis.id.starts_with("bio.")));
    const RETIRED_IDS: &[&str] = &[
        "oak",
        "mech.stroke_length",
        "fluid.respirable_content",
        "fluid.gas_transfer_coefficient",
        "acoustic.frequency",
        "acoustic.source_power",
        "acoustic.absorption_coefficient",
        "acoustic.absorption_reference",
        "acoustic.resonator_length",
        "acoustic.formant_frequency",
        "law.membrane_gas_flux",
        "law.acoustic_absorption",
        "law.tube_resonance",
        "opt.emissivity.band_0",
        "opt.emissivity.band_1",
        "opt.emissivity.band_2",
        "opt.spectral_band",
    ];
    for id in RETIRED_IDS {
        assert!(
            reg.axis(id).is_none() && reg.law(id).is_none() && reg.substance(id).is_none(),
            "retired candidate '{id}' must be available only through parked compatibility"
        );
    }
    for law in ["law.net_nutrition", "law.harm", "law.edibility"] {
        assert!(reg.law(law).is_none(), "retired law {law} stays parked");
    }
    for substance in ["carrion", "oilseed", "spent_hull", "tuber"] {
        assert!(
            reg.substance(substance).is_none(),
            "retired substance {substance} stays parked"
        );
    }
}

#[test]
fn each_floor_hashes_deterministically_across_loads() {
    assert_eq!(mechanical().content_id(), mechanical().content_id());
}

#[test]
fn every_law_input_resolves_to_a_defined_axis() {
    // A successful load already validates this, but assert it explicitly over the mechanical floor
    // so the guarantee is on record: no law reaches for an absent axis.
    let reg = mechanical();
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

// The evidence-ground string carried by a broad physics-data row. This legacy real/fantasy distinction is
// an audit input only; it does not admit the row to the canonical absolute floor.
fn provenance_ground(p: &Provenance) -> &str {
    match p {
        Provenance::RealWithSource(s) | Provenance::FantasyReserved(s) => s,
    }
}

#[test]
fn the_full_floor_is_born_provenance_tagged() {
    // Every candidate row carried by the broad physics registry (a QuantityAxis, a Substance, or a periodic
    // Element) declares an evidence ground. The Rust loader enforces this fail-loud (`provenance_from` returns
    // `MissingProvenance` when an entry declares neither `real` nor `fantasy`), so `ground()` and
    // `PeriodicTable::standard()` panicking on a missing tag IS the enforcement; this test exercises it over
    // the real embedded floor and asserts each entry's provenance ground is non-empty (a degenerate
    // `RealWithSource("")` is as much a defect as an absent tag). Laws are derivations and carry no candidate
    // evidence row. This test establishes audit completeness only. Canonical admission and exact counts are
    // separately enforced by civsim-ledger. The broad-data counts use non-empty lower bounds so new research
    // candidates do not break the invariant test.
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
fn every_broad_physics_candidate_has_a_provenance_audit_record() {
    use civsim_physics::floor_provenance::FloorCandidateRegistry;
    // This candidate registry must stay in sync with the broad loaded physics population. It is an audit and
    // migration surface, not the canonical admission list. The exact admitted floor is checked separately by
    // civsim-ledger and the generated four-tier by seven-mark inventory.
    let reg = FloorCandidateRegistry::embedded().expect("the candidate audit register parses");
    let ground = PhysicsRegistry::ground().expect("the ground floor loads");
    let table = PeriodicTable::standard().expect("the periodic table loads");
    for axis in ground.axes() {
        assert!(
            reg.record(&axis.id).is_some(),
            "physics axis '{}' has no candidate record in floor_provenance.toml",
            axis.id
        );
    }
    for sub in ground.substances() {
        assert!(
            reg.record(&sub.id).is_some(),
            "physics substance '{}' has no candidate record in floor_provenance.toml",
            sub.id
        );
    }
    for element in table.elements() {
        assert!(
            reg.record(&element.symbol).is_some(),
            "periodic element '{}' has no candidate record in floor_provenance.toml",
            element.symbol
        );
    }
    // Each phase carries a candidate record keyed `phase.<name>`. The prefix prevents a phase such as
    // hematite from colliding with a broad substance row of the same name.
    let phases = civsim_physics::petrology_data::PhaseRegistry::standard()
        .expect("the phase registry loads");
    for phase in phases.phases() {
        let key = format!("phase.{}", phase.name);
        let record = reg.record(&key).unwrap_or_else(|| {
            panic!("phase '{key}' has no candidate record in floor_provenance.toml")
        });
        // MOVED WITH THE TRUTH, 2026-07-19, and this row is the clearest illustration of why. A
        // candidate phase DOES carry cited thermodynamic data: every registry row has a real citation in
        // its `source` field. What it does not carry is a MACHINE-RESOLVABLE source id, a claim locator,
        // a measurement regime or an uncertainty, so nothing here can tell a well-evidenced value
        // from a conventionally labelled one without a human reading prose. An audit found 244 such
        // labels across both registers with zero machine-linked evidence between them.
        //
        // These are therefore `unverified_measurement_candidate`, not because the literature is absent
        // but because the LINK is. Each resolves one of two equally good ways: promotion once its claim
        // record exists, or truthful downgrade. The assertion still pins the grade, so a silent drift to
        // some other tag still fails.
        assert_eq!(
            record.status, "unverified_measurement_candidate",
            "a candidate phase cites its thermodynamics in prose but carries no machine-resolvable \
             source id, so it is an unverified measurement candidate until that link exists"
        );
        assert!(
            record.derive_first_defect,
            "a phase's stored properties should derive from constituents in the materials buildout"
        );
    }
}
