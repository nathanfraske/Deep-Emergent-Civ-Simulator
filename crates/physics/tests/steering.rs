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

//! Hardcoded anti-steering and anti-hallucination guarantees for the physics substrate (Principle 9,
//! Principle 11). These convert the substrate's most-repeated invariants from prose claims and
//! point-in-time red-team review into permanent, build-enforced checks:
//!
//! - matter is identified by its physics, never by its name (a race cannot get different substrate
//!   behaviour by relabelling its iron);
//! - no value enters the substrate without a provenance (a fabricated or hallucinated number cannot
//!   load silently);
//! - a kernel symmetric in interchangeable inputs carries no hidden bias;
//! - the canonical path is integer-only (no float can leak in and introduce rounding nondeterminism).
//!
//! The honest ceiling (the Part 41 Steering Audit still owns this): these catch MECHANICAL steering
//! (identity-keyed behaviour, an uncited value, a hidden asymmetry, a float leak). They cannot catch
//! SEMANTIC steering in the CHOICE of which axes and laws exist, which stays the adversarial review's
//! job. Kernel identity-blindness is currently structural (a kernel takes `Fixed` values, never an
//! id), so the relabelling guarantee is locked here at the substance layer, where identity does live
//! (`Substance::content_id`); it extends to a substance-to-law dispatch layer if one is ever built.

use civsim_core::Fixed;
use civsim_physics::{laws, PhysicsError, PhysicsRegistry, Provenance, Substance};
use std::collections::BTreeMap;

fn subst(id: &str, v: &[(&str, i64)]) -> Substance {
    let mut vector = BTreeMap::new();
    for (axis, val) in v {
        vector.insert((*axis).to_string(), Fixed::from_int(*val as i32));
    }
    Substance {
        id: id.to_string(),
        vector,
        participates_in: vec!["law.contact_pressure".to_string()],
        provenance: Provenance::RealWithSource("test".to_string()),
    }
}

#[test]
fn matter_is_identified_by_physics_not_by_label() {
    // The anti-steering-by-identity guarantee: two substances with identical physical vectors but
    // different names have the same content id, so nothing downstream can treat one race's iron
    // differently from another's by name alone. content_id must never fold self.id.
    let elf = subst(
        "elf_iron",
        &[("mat.density", 7870), ("mat.yield_strength", 250)],
    );
    let dwarf = subst(
        "dwarf_iron",
        &[("mat.density", 7870), ("mat.yield_strength", 250)],
    );
    assert_eq!(
        elf.content_id(),
        dwarf.content_id(),
        "same physics under a different name is the same matter (no identity steering)"
    );
    // And it stays physics-sensitive: a different vector is a different substance, so the hash is not
    // trivially constant.
    let lighter = subst(
        "elf_iron",
        &[("mat.density", 7860), ("mat.yield_strength", 250)],
    );
    assert_ne!(
        elf.content_id(),
        lighter.content_id(),
        "different physics is different matter"
    );
}

#[test]
fn substance_identity_is_assembly_order_independent() {
    // The composition is folded in canonical (sorted) order, so the same matter hashes identically
    // however it was assembled, a determinism guard on the content address.
    let a = subst("x", &[("mat.density", 7870), ("mat.yield_strength", 250)]);
    let mut reversed = BTreeMap::new();
    reversed.insert("mat.yield_strength".to_string(), Fixed::from_int(250));
    reversed.insert("mat.density".to_string(), Fixed::from_int(7870));
    let b = Substance {
        id: "x".to_string(),
        vector: reversed,
        participates_in: vec!["law.contact_pressure".to_string()],
        provenance: Provenance::RealWithSource("test".to_string()),
    };
    assert_eq!(a.content_id(), b.content_id());
}

#[test]
fn no_value_enters_the_substrate_without_a_provenance() {
    // Anti-hallucination: an axis (or substance) that carries neither a real citation nor a
    // fantasy-reserved basis is a fabricated value, and it must fail to load rather than enter
    // silently. Every number in the substrate is cited or reserved.
    let no_prov = "[[axis]]\nid = \"x\"\nmeasures = \"m\"\nunit = \"u\"\ndimension = \"dimensionless\"\nscale = \"u\"\ntier = 0\nrange_reserved = \"b\"\n";
    match PhysicsRegistry::from_toml_str(no_prov) {
        Err(PhysicsError::MissingProvenance(_)) => {}
        other => panic!("an unsourced value must be rejected as MissingProvenance, got {other:?}"),
    }
    // The same axis with a citation loads.
    let with_prov = format!("{no_prov}real = \"a cited source\"\n");
    assert!(PhysicsRegistry::from_toml_str(&with_prov).is_ok());
}

#[test]
fn a_symmetric_kernel_carries_no_hidden_bias() {
    // A kernel physically symmetric in two interchangeable inputs must be exactly symmetric; an
    // unjustified asymmetry would be a hidden bias (a steering or hallucination smell). The
    // electrostatic force is symmetric in the two charges.
    let f_max = Fixed::from_int(1_000_000_000);
    let k = Fixed::from_int(9);
    let r = Fixed::from_int(3);
    let q1 = Fixed::from_int(2);
    let q2 = Fixed::from_int(5);
    assert_eq!(
        laws::coulomb_force(q1, q2, r, k, f_max),
        laws::coulomb_force(q2, q1, r, k, f_max),
        "Coulomb's force is symmetric in the two charges"
    );
}

#[test]
fn the_canonical_kernel_path_is_integer_only() {
    // Part 3.4/5.4: no float appears anywhere on the canonical path, so there is no floating-point
    // variance to make a kernel nondeterministic or vendor-dependent. This scans the law-kernel
    // module itself; the only sanctioned float is the tagged view/boundary helper `to_f64_lossy`,
    // which lives in core, not here. A hallucinated float kernel added here fails this build.
    let src = include_str!("../src/laws.rs");
    for token in ["f32", "f64"] {
        assert!(
            !src.contains(token),
            "the canonical kernel module must contain no {token}; the canonical path is integer-only"
        );
    }
}
