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

//! The composition evaluator's behavioural proof (design Part 41). The centrepiece is the non-steering
//! test: the same structural design over two materials with divergent physics produces a divergent
//! viability and vector, purely from the material vectors, with the kernel path taking only Fixed.

use civsim_compose::{
    evaluate_node, evaluate_uncached, promote, promoted_library, CombinatorKernel,
    CombinatorRegistry, ComponentRef, CompositionNode, DesignEvidence, EvalParams, FormId,
    FormRegistry, IntentRef, InterfaceRegistry, Interval, JoinId, JoinRegistry, Memo, NodeBody,
    PenaltyCurve, Promotion, PromotionParams, ProxyRegistry, ProxyWeights,
};
use civsim_core::Fixed;
use civsim_physics::PhysicsRegistry;

fn fx(s: &str) -> Fixed {
    Fixed::from_decimal_str(s).unwrap()
}

/// A labelled fixture physics registry: the material axes the two divergent substances carry, plus the
/// interface-bound mechanical axes, all with set ranges. The elastic-modulus values are illustrative
/// (chosen on a scale comparable to the declared masses so the resonance proxy exercises a sign change),
/// not real megapascals.
fn physics() -> PhysicsRegistry {
    let toml = r#"
[[axis]]
id = "mat.fracture_strength"
dimension = "pressure"
scale = "MPa"
range_lo = "1"
range_hi = "10000"
real = "fixture"

[[axis]]
id = "mat.fracture_energy"
dimension = "0,1,-2,0"
scale = "J/m^2"
range_lo = "1"
range_hi = "1000000"
real = "fixture"

[[axis]]
id = "mat.yield_strength"
dimension = "pressure"
scale = "MPa"
range_lo = "1"
range_hi = "10000"
real = "fixture"

[[axis]]
id = "mat.elastic_modulus"
dimension = "pressure"
scale = "MPa"
range_lo = "1"
range_hi = "1200000"
real = "fixture"

[[axis]]
id = "mech.mass"
dimension = "mass"
scale = "kg"
range_lo = "0.001"
range_hi = "100000"
real = "fixture"

[[axis]]
id = "mech.restitution"
dimension = "dimensionless"
scale = "1"
range_lo = "0"
range_hi = "1"
real = "fixture"

[[axis]]
id = "mech.force"
dimension = "force"
scale = "N"
range_lo = "0.01"
range_hi = "100000000"
real = "fixture"

# Hard-brittle: high fracture strength, LOW toughness (low fracture energy), stiff.
[[substance]]
id = "stone"
real = "fixture: a hard-brittle stone"
values = [
  { axis = "mat.fracture_strength", value = "400" },
  { axis = "mat.fracture_energy", value = "50" },
  { axis = "mat.yield_strength", value = "380" },
  { axis = "mat.elastic_modulus", value = "400" },
]

# The same physics as stone under a different name (the identity-blindness fixture).
[[substance]]
id = "elf_stone"
real = "fixture: a hard-brittle stone"
values = [
  { axis = "mat.fracture_strength", value = "400" },
  { axis = "mat.fracture_energy", value = "50" },
  { axis = "mat.yield_strength", value = "380" },
  { axis = "mat.elastic_modulus", value = "400" },
]

# Tough-ductile: lower fracture strength, HIGH toughness, more compliant.
[[substance]]
id = "bronze"
real = "fixture: a tough-ductile bronze"
values = [
  { axis = "mat.fracture_strength", value = "200" },
  { axis = "mat.fracture_energy", value = "20000" },
  { axis = "mat.yield_strength", value = "150" },
  { axis = "mat.elastic_modulus", value = "100" },
]
"#;
    PhysicsRegistry::from_toml_str(toml).expect("fixture physics registry loads")
}

fn material(reg: &PhysicsRegistry, id: &str) -> u128 {
    reg.substance(id).expect("fixture substance").content_id()
}

/// The reserved-with-basis eval params: a penalty curve with a half-unit adaptable range that floors at
/// 0.5, and the three proxy criticality weights. Labelled dev values, not owner production numbers.
fn params() -> EvalParams {
    let mut weights = ProxyWeights::new();
    weights
        .set(ProxyRegistry::ID_RESONANCE, fx("0.5"))
        .set(ProxyRegistry::ID_THERMAL, fx("0.5"))
        .set(ProxyRegistry::ID_CONTROL, fx("0.1"));
    EvalParams {
        penalty_curve: PenaltyCurve::new([(fx("0"), fx("0")), (fx("0.5"), fx("0.5"))]),
        proxy_weights: weights,
    }
}

fn memo(interface: InterfaceRegistry) -> Memo {
    Memo::new(
        interface,
        CombinatorRegistry::dev_seed(),
        ProxyRegistry::dev_seed(),
        FormRegistry::dev_seed(),
        JoinRegistry::dev_seed(),
        params(),
    )
    .expect("all proxies weighted")
}

/// A structural leaf: a beam of one material, welded, rated over a reference-load interval and an
/// impact-energy interval, with a declared envelope mass.
fn leaf(
    material_id: u128,
    load_lo: &str,
    load_hi: &str,
    mass: &str,
    e_lo: &str,
    e_hi: &str,
) -> CompositionNode {
    CompositionNode::new(
        IntentRef(0),
        NodeBody::Leaf {
            primitives: vec![FormId(0)], // the beam form
            material: material_id,
            joining: JoinId(0), // welded
        },
        InterfaceRegistry::dev_seed_base().empty_vector(),
        vec![fx(load_lo), fx(load_hi), fx(mass), fx(e_lo), fx(e_hi)],
    )
}

// === The non-steering test: divergence from the material vectors alone ===

#[test]
fn the_same_design_over_divergent_materials_diverges_from_the_material_alone() {
    let reg = physics();
    let mut m = memo(InterfaceRegistry::dev_seed_base());
    // Identical structural design (same beam form, same weld join, same reference-load and energy
    // contract) over two materials with divergent physics: a hard-brittle stone and a tough-ductile
    // bronze. Only the material content id differs.
    let stone = leaf(material(&reg, "stone"), "5000", "8000", "10", "0.15", "0.3");
    let bronze = leaf(
        material(&reg, "bronze"),
        "5000",
        "8000",
        "10",
        "0.15",
        "0.3",
    );

    let stone_eval = evaluate_node(&reg, &mut m, &stone);
    let bronze_eval = evaluate_node(&reg, &mut m, &bronze);

    // The viability intervals diverge purely from the material vectors: the brittle stone is toughness
    // limited (a lower viability floor), the ductile bronze is stress limited.
    assert_ne!(
        stone_eval.viability, bronze_eval.viability,
        "viability must diverge from the material physics"
    );
    // The aggregated vectors diverge too (the stress-margin and toughness-margin slots differ).
    assert_ne!(
        stone_eval.vector, bronze_eval.vector,
        "the interface vector must diverge from the material physics"
    );
    // And it is a genuine trade: the stone's stress margin (slot 0) is higher (stronger) while the
    // bronze's toughness margin (slot 1) is higher (tougher). Neither material dominates.
    assert!(
        stone_eval.vector.interval_at(0).lo > bronze_eval.vector.interval_at(0).lo,
        "the hard-brittle material wins the stress margin"
    );
    assert!(
        bronze_eval.vector.interval_at(1).lo > stone_eval.vector.interval_at(1).lo,
        "the tough-ductile material wins the toughness margin"
    );
}

#[test]
fn matter_is_identified_by_physics_not_by_label_at_the_leaf() {
    // The anti-identity-steering guarantee at the composition layer: two substances with identical
    // physics but different names resolve to the same content id, so the leaf gives the same result.
    // The kernel path takes only Fixed, never a substance name.
    let reg = physics();
    let mut m = memo(InterfaceRegistry::dev_seed_base());
    let stone = leaf(material(&reg, "stone"), "5000", "8000", "10", "0.15", "0.3");
    let twin = leaf(
        material(&reg, "elf_stone"),
        "5000",
        "8000",
        "10",
        "0.15",
        "0.3",
    );
    assert_eq!(
        stone.content_id(),
        twin.content_id(),
        "same physics is the same design"
    );
    assert_eq!(
        evaluate_node(&reg, &mut m, &stone),
        evaluate_node(&reg, &mut m, &twin),
        "same physics evaluates identically"
    );
}

// === Non-templatedness: one physics, two peoples, different libraries ===

#[test]
fn two_peoples_with_different_interface_substrates_get_different_libraries() {
    let reg = physics();
    let bronze = material(&reg, "bronze");

    // The same intent stream: a light design and a heavy design, each a composite wrapping one bronze
    // beam leaf. They differ only in the child's declared envelope mass.
    let (light_child, light) = wrap(leaf(bronze, "5000", "8000", "10", "0.15", "0.3"));
    let (heavy_child, heavy) = wrap(leaf(bronze, "5000", "8000", "900", "0.15", "0.3"));

    let base_lib = library_under(
        &reg,
        InterfaceRegistry::dev_seed_base(),
        &[
            (light_child.clone(), light.clone()),
            (heavy_child.clone(), heavy.clone()),
        ],
    );
    let exotic_lib = library_under(
        &reg,
        InterfaceRegistry::dev_seed_exotic(),
        &[(light_child, light), (heavy_child, heavy)],
    );

    // Under the base substrate both designs promote (no resonance axis, so no resonance gate). Under the
    // exotic substrate the heavy design's resonance proxy (active only because the exotic stiffness axis
    // exposes its input port) pulls its viability below the collapse boundary, so it is rejected. Same
    // physics, same intent stream: a different technology library.
    assert_eq!(base_lib.len(), 2, "the base people promote both designs");
    assert_eq!(
        exotic_lib.len(),
        1,
        "the exotic people's resonance gate rejects the heavy design"
    );
    assert_ne!(
        base_lib, exotic_lib,
        "the promoted libraries differ in shape under one physics"
    );
}

/// Wrap a leaf in a single-child composite with a generous mass budget (so the envelope penalty does
/// not fire), returning both so the memo can be seeded bottom-up.
fn wrap(child: CompositionNode) -> (CompositionNode, CompositionNode) {
    let composite = CompositionNode::new(
        IntentRef(0),
        NodeBody::Composite {
            children: vec![ComponentRef {
                target: child.content_id(),
                transform: civsim_compose::TransformId(0),
                overrides: vec![],
            }],
            assembly_material: child.content_id(), // reuse a content id as the assembly matter tag
            assembly_join: JoinId(0),
        },
        InterfaceRegistry::dev_seed_base().empty_vector(),
        vec![fx("0"), fx("0"), fx("100000")], // param[2] a huge budget: no envelope mismatch
    );
    (child, composite)
}

fn library_under(
    reg: &PhysicsRegistry,
    interface: InterfaceRegistry,
    designs: &[(CompositionNode, CompositionNode)],
) -> Vec<u128> {
    let mut m = memo(interface);
    let params = PromotionParams {
        viability_floor: Fixed::ZERO,
        loss_rate: fx("0.25"),
        drift_rate: fx("0.01"),
        reuse_threshold: 1,
    };
    let mut evidence = Vec::new();
    for (child, composite) in designs {
        m.insert(child.clone());
        m.insert(composite.clone());
        let ev = evaluate_node(reg, &mut m, composite);
        evidence.push(DesignEvidence {
            id: composite.content_id(),
            viability: ev.viability,
            persisted_ticks: 1000,
            copies: vec![fx("0.5"), fx("0.5")],
            reuse_count: 5,
        });
    }
    promoted_library(&evidence, &params)
}

// === Content-id determinism ===

#[test]
fn same_design_same_content_id_across_builds() {
    let reg = physics();
    let a = leaf(material(&reg, "stone"), "5000", "8000", "10", "0.15", "0.3");
    let b = leaf(material(&reg, "stone"), "5000", "8000", "10", "0.15", "0.3");
    assert_eq!(a.id, b.id);
    assert_eq!(a.id, a.content_id(), "the stored id is the content id");
}

#[test]
fn permuted_children_canonicalize_to_the_same_id() {
    let reg = physics();
    let x = leaf(material(&reg, "stone"), "5000", "8000", "10", "0.15", "0.3");
    let y = leaf(
        material(&reg, "bronze"),
        "5000",
        "8000",
        "20",
        "0.15",
        "0.3",
    );
    let cref = |t: u128| ComponentRef {
        target: t,
        transform: civsim_compose::TransformId(0),
        overrides: vec![],
    };
    let forward = CompositionNode::new(
        IntentRef(7),
        NodeBody::Composite {
            children: vec![cref(x.content_id()), cref(y.content_id())],
            assembly_material: 0,
            assembly_join: JoinId(1),
        },
        InterfaceRegistry::dev_seed_base().empty_vector(),
        vec![],
    );
    let reversed = CompositionNode::new(
        IntentRef(99), // a different intent must not change the id
        NodeBody::Composite {
            children: vec![cref(y.content_id()), cref(x.content_id())],
            assembly_material: 0,
            assembly_join: JoinId(1),
        },
        InterfaceRegistry::dev_seed_base().empty_vector(),
        vec![],
    );
    assert_eq!(
        forward.content_id(),
        reversed.content_id(),
        "a permuted assembly canonicalizes to the same content id, and the intent is not folded"
    );
}

// === Memoisation soundness: cache equals no-cache, bit for bit ===

#[test]
fn evaluate_with_cache_equals_without() {
    let reg = physics();
    let child = leaf(
        material(&reg, "bronze"),
        "5000",
        "8000",
        "30",
        "0.15",
        "0.3",
    );
    let (child, composite) = wrap(child);
    let mut m = memo(InterfaceRegistry::dev_seed_exotic());
    m.insert(child);
    m.insert(composite.clone());

    let cached = evaluate_node(&reg, &mut m, &composite);
    let uncached = evaluate_uncached(&reg, &mut m, &composite);
    assert_eq!(
        cached, uncached,
        "the warm cache must equal the recomputed result"
    );

    // A cold re-evaluation after clearing the cache reproduces it.
    m.clear_cache();
    let recomputed = evaluate_node(&reg, &mut m, &composite);
    assert_eq!(
        cached, recomputed,
        "a cold re-evaluation reproduces the result"
    );
}

// === The viability gate ===

#[test]
fn a_lower_bound_below_the_collapse_boundary_is_rejected() {
    let reg = physics();
    let mut m = memo(InterfaceRegistry::dev_seed_base());
    // A design loaded from a gentle reference to one that exceeds the stone's fracture strength: the
    // viability interval straddles the collapse boundary (its lower bound is below zero while its upper
    // bound is above). The gate rejects it on the lower bound even though the upper would pass.
    let straddling = leaf(
        material(&reg, "stone"),
        "5000",
        "60000",
        "10",
        "0.15",
        "0.3",
    );
    let ev = evaluate_node(&reg, &mut m, &straddling);
    assert!(
        ev.viability.lo < Fixed::ZERO,
        "the high load pushes the lower bound past collapse"
    );
    assert!(
        ev.viability.hi > Fixed::ZERO,
        "the low load keeps the upper bound viable"
    );
    // A wide interval flags an unpinnable coupling.
    assert!(
        ev.viability.width() > fx("0.5"),
        "the straddling load is a wide, unpinnable interval"
    );

    let params = PromotionParams {
        viability_floor: Fixed::ZERO,
        loss_rate: fx("0.25"),
        drift_rate: fx("0.01"),
        reuse_threshold: 1,
    };
    let evidence = DesignEvidence {
        id: straddling.content_id(),
        viability: ev.viability,
        persisted_ticks: 1000,
        copies: vec![],
        reuse_count: 5,
    };
    assert_eq!(promote(&evidence, &params), Promotion::RejectedViability);

    // A well-pinned, fully-viable design promotes.
    let sound = leaf(material(&reg, "stone"), "5000", "6000", "10", "0.15", "0.2");
    let sound_ev = evaluate_node(&reg, &mut m, &sound);
    assert!(sound_ev.viability.lo >= Fixed::ZERO);
    let sound_evidence = DesignEvidence {
        id: sound.content_id(),
        viability: sound_ev.viability,
        persisted_ticks: 1000,
        copies: vec![],
        reuse_count: 5,
    };
    assert_eq!(promote(&sound_evidence, &params), Promotion::Promoted);
}

#[test]
fn the_transmission_gate_rejects_an_unstable_design() {
    // Gate two: a design that has not persisted long enough to outrun loss, or whose copies have
    // drifted past the similarity radius, is not promoted even when it is fully viable. The rates are
    // the transmission substrate's own (compose.transmission_stability derives set-equal to them).
    let params = PromotionParams {
        viability_floor: Fixed::ZERO,
        loss_rate: fx("0.25"),  // stability span = ceil(1/0.25) = 4 ticks
        drift_rate: fx("0.01"), // similarity radius = 0.02
        reuse_threshold: 1,
    };
    let base = DesignEvidence {
        id: 1,
        viability: Interval::point(fx("0.5")),
        persisted_ticks: 10,
        copies: vec![fx("0.50"), fx("0.51")], // within the 0.02 radius
        reuse_count: 5,
    };
    assert_eq!(promote(&base, &params), Promotion::Promoted);

    // Too young: has not outrun loss.
    let young = DesignEvidence {
        persisted_ticks: 2,
        ..base.clone()
    };
    assert_eq!(promote(&young, &params), Promotion::RejectedUnstable);

    // Drifted: its copies span beyond the similarity radius.
    let drifted = DesignEvidence {
        copies: vec![fx("0.4"), fx("0.7")],
        ..base.clone()
    };
    assert_eq!(promote(&drifted, &params), Promotion::RejectedUnstable);

    // Not reused enough: gate three.
    let unshared = DesignEvidence {
        reuse_count: 0,
        ..base
    };
    assert_eq!(promote(&unshared, &params), Promotion::RejectedReuse);
}

#[test]
fn an_unweighted_proxy_fails_loud() {
    // A proxy with no criticality weight (compose.emergent_proxy_weights unset for it) cannot build an
    // evaluation substrate: the reserved value is never silently defaulted.
    let params = EvalParams {
        penalty_curve: PenaltyCurve::new([(fx("0"), fx("0"))]),
        proxy_weights: ProxyWeights::new(), // no weights at all
    };
    let built = Memo::new(
        InterfaceRegistry::dev_seed_base(),
        CombinatorRegistry::dev_seed(),
        ProxyRegistry::dev_seed(),
        FormRegistry::dev_seed(),
        JoinRegistry::dev_seed(),
        params,
    );
    assert!(
        built.is_err(),
        "an unweighted proxy must fail loud rather than contribute nothing"
    );
}

// === Combinator correctness ===

#[test]
fn limiting_min_is_the_weakest_child() {
    let children = [
        (10u128, Interval::new(fx("3"), fx("5"))),
        (20u128, Interval::new(fx("1"), fx("9"))),
        (5u128, Interval::new(fx("2"), fx("4"))),
    ];
    let folded = CombinatorKernel::LimitingMin.fold(&children);
    assert_eq!(
        folded,
        Interval::new(fx("1"), fx("4")),
        "elementwise minimum across the children"
    );
}

#[test]
fn saturating_sum_adds_and_saturates() {
    let children = [
        (1u128, Interval::point(fx("3"))),
        (2u128, Interval::point(fx("4"))),
    ];
    assert_eq!(
        CombinatorKernel::SaturatingSum.fold(&children),
        Interval::point(fx("7")),
        "capacities add"
    );
    // A sum past the representable ceiling saturates rather than wrapping.
    let big = [
        (1u128, Interval::point(Fixed::MAX)),
        (2u128, Interval::point(Fixed::MAX)),
    ];
    assert_eq!(
        CombinatorKernel::SaturatingSum.fold(&big),
        Interval::point(Fixed::MAX)
    );
}

#[test]
fn efficiency_product_compounds_loss_and_is_order_independent() {
    let a = (100u128, Interval::point(fx("0.5")));
    let b = (200u128, Interval::point(fx("0.4")));
    let c = (300u128, Interval::point(fx("0.8")));
    let forward = CombinatorKernel::EfficiencyProduct.fold(&[a, b, c]);
    let permuted = CombinatorKernel::EfficiencyProduct.fold(&[c, a, b]);
    assert_eq!(
        forward, permuted,
        "the product folds in canonical content-id order, order-independent"
    );
    // Loss compounds: 0.5 * 0.4 * 0.8 = 0.16.
    assert_eq!(forward, Interval::point(fx("0.16")));
}

// === Interface-mismatch penalty: graded then floored ===

#[test]
fn the_interface_penalty_grades_then_floors() {
    let curve = PenaltyCurve::new([(fx("0"), fx("0")), (fx("0.5"), fx("0.5"))]);
    // Inside the adaptable range the penalty is graded (linear here).
    assert_eq!(curve.eval(fx("0")), fx("0"));
    assert_eq!(
        curve.eval(fx("0.25")),
        fx("0.25"),
        "a graded penalty inside the adaptable range"
    );
    // Beyond the adaptable range the penalty FLOORS at the last point rather than diverging, so a large
    // mismatch is a bounded cost an adapter can pay, not a hard rejection.
    assert_eq!(curve.eval(fx("0.5")), fx("0.5"));
    assert_eq!(
        curve.eval(fx("5")),
        fx("0.5"),
        "beyond the range the penalty floors"
    );
    assert_eq!(curve.adaptable_range(), fx("0.5"));
}

#[test]
fn an_over_envelope_composite_is_penalized_but_not_hard_rejected() {
    let reg = physics();
    let bronze = material(&reg, "bronze");
    // A composite whose child demands more envelope mass than the composite's declared budget: an
    // over-envelope mismatch. The penalty grades it down but does not annihilate it.
    let child = leaf(bronze, "5000", "6000", "150", "0.15", "0.2"); // 150 kg demanded
    let composite = CompositionNode::new(
        IntentRef(0),
        NodeBody::Composite {
            children: vec![ComponentRef {
                target: child.content_id(),
                transform: civsim_compose::TransformId(0),
                overrides: vec![],
            }],
            assembly_material: 0,
            assembly_join: JoinId(0),
        },
        InterfaceRegistry::dev_seed_base().empty_vector(),
        vec![fx("0"), fx("0"), fx("100")], // budget 100 kg, so 150 kg is a 0.5 over-fraction
    );
    let mut m = memo(InterfaceRegistry::dev_seed_base());
    m.insert(child.clone());
    m.insert(composite.clone());
    let with_penalty = evaluate_node(&reg, &mut m, &composite);

    // The same child under a budget it fits: no penalty. The over-envelope composite has strictly lower
    // viability, but is not driven to negative infinity (the penalty floored at 0.5).
    let fitting = CompositionNode::new(
        IntentRef(0),
        NodeBody::Composite {
            children: vec![ComponentRef {
                target: child.content_id(),
                transform: civsim_compose::TransformId(0),
                overrides: vec![],
            }],
            assembly_material: 0,
            assembly_join: JoinId(0),
        },
        InterfaceRegistry::dev_seed_base().empty_vector(),
        vec![fx("0"), fx("0"), fx("100000")], // a budget it fits
    );
    m.insert(fitting.clone());
    let no_penalty = evaluate_node(&reg, &mut m, &fitting);

    assert!(
        with_penalty.viability.lo < no_penalty.viability.lo,
        "the over-envelope design is penalized"
    );
    assert!(
        with_penalty.viability.lo > no_penalty.viability.lo - fx("0.6"),
        "the penalty floored (bounded), so an adapter could bridge it rather than a hard rejection"
    );
}

// === Cross-tier additive invariant ===

#[test]
fn additive_mass_is_conserved_exactly_but_a_nonlinear_proxy_is_tier_dependent() {
    let reg = physics();
    let bronze = material(&reg, "bronze");
    let c1 = leaf(bronze, "5000", "6000", "40", "0.15", "0.2");
    let c2 = leaf(bronze, "5000", "6000", "90", "0.15", "0.2");
    let composite = CompositionNode::new(
        IntentRef(0),
        NodeBody::Composite {
            children: vec![
                ComponentRef {
                    target: c1.content_id(),
                    transform: civsim_compose::TransformId(0),
                    overrides: vec![],
                },
                ComponentRef {
                    target: c2.content_id(),
                    transform: civsim_compose::TransformId(0),
                    overrides: vec![],
                },
            ],
            assembly_material: 0,
            assembly_join: JoinId(0),
        },
        InterfaceRegistry::dev_seed_exotic().empty_vector(),
        vec![fx("0"), fx("0"), fx("100000")],
    );
    let mut m = memo(InterfaceRegistry::dev_seed_exotic());
    m.insert(c1.clone());
    m.insert(c2.clone());
    m.insert(composite.clone());

    let e1 = evaluate_node(&reg, &mut m, &c1);
    let e2 = evaluate_node(&reg, &mut m, &c2);
    let ec = evaluate_node(&reg, &mut m, &composite);

    // The additive envelope-mass slot (slot 2, the ConservedBudget axis) is the EXACT sum of the
    // children's masses, to the bit: a conserved cross-tier projection.
    let mass_slot = 2;
    let child_sum = civsim_compose::Interval::new(
        Fixed::from_bits(
            e1.vector.interval_at(mass_slot).lo.to_bits()
                + e2.vector.interval_at(mass_slot).lo.to_bits(),
        ),
        Fixed::from_bits(
            e1.vector.interval_at(mass_slot).hi.to_bits()
                + e2.vector.interval_at(mass_slot).hi.to_bits(),
        ),
    );
    assert_eq!(
        ec.vector.interval_at(mass_slot),
        child_sum,
        "parent envelope mass equals the sum of children exactly"
    );

    // A nonlinear read over the additive mass is tier-resolution-dependent BY CONSTRUCTION: the square
    // root of the lump mass differs from the sum of the children's square roots. This difference is
    // allowed and must not be asserted away as tier-invariant (the honest coarser-physics gap).
    let lump = ec.vector.interval_at(mass_slot).hi.sqrt();
    let per_child =
        e1.vector.interval_at(mass_slot).hi.sqrt() + e2.vector.interval_at(mass_slot).hi.sqrt();
    assert_ne!(
        lump, per_child,
        "a nonlinear proxy over the additive quantity is tier-dependent"
    );
}
