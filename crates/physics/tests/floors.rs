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

use civsim_physics::{PhysicsError, PhysicsRegistry};

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
    assert_eq!(reg.axis_count(), 38, "the mechanical-and-materials axes");
    assert_eq!(reg.law_count(), 19, "the wave-1 interaction laws");
    assert_eq!(
        reg.substance_count(),
        2,
        "the iron and oak example materials"
    );
}

#[test]
fn the_biology_floor_loads_with_its_axes_and_three_laws() {
    let reg = biology();
    assert_eq!(
        reg.axis_count(),
        16,
        "the composition, toxin, and consumer axes"
    );
    assert_eq!(reg.law_count(), 3, "net nutrition, harm, and edibility");
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
fn every_reserved_range_is_in_the_review_queue_and_fails_loud() {
    let reg = mechanical();
    // The mechanical floor sets only the pressure scale; every range is reserved, so the
    // whole axis set is the owner's review queue and reading any range fails loud.
    assert_eq!(reg.reserved_axis_ids().len(), reg.axis_count());
    let density = reg.axis("mat.density").unwrap();
    assert_eq!(
        density.range.require("mat.density").unwrap_err(),
        PhysicsError::ReservedRange("mat.density".to_string())
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
