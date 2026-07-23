use super::super::SpeciesContentIdentity;
use super::{
    inspect_physical_registry, model::*, producer, repository_input,
    resolve_repository_physical_species_registry, watchdog,
};

#[derive(Debug, Clone, Copy)]
enum FixtureRoute {
    Direct,
    Elementary,
}

#[derive(Debug, Clone, Copy, Default)]
struct FixtureDefects {
    wrong_mass_kind: bool,
    wrong_state_kind: bool,
    wrong_sector_kind: bool,
    wrong_validity_kind: bool,
    unexpected_dependency: bool,
    zero_projection: bool,
}

#[derive(Debug, Clone)]
struct Fixture {
    input: PhysicalRegistryInput,
    member: SpeciesContentIdentity,
    scalar: ArtifactIdentity,
}

fn magnitude(value: u128) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }
    let bytes = value.to_be_bytes();
    bytes
        .iter()
        .position(|byte| *byte != 0)
        .map_or_else(|| vec![0], |start| bytes[start..].to_vec())
}

fn rational(numerator: u128, denominator: u128) -> ExactRationalWire {
    ExactRationalWire {
        negative: false,
        numerator_be: magnitude(numerator),
        denominator_be: magnitude(if numerator == 0 { 1 } else { denominator }),
    }
}

fn receipt(name: &str, tag: u8) -> ReceiptBinding {
    let mut digest = [tag; 32];
    digest[31] = tag.wrapping_add(1);
    ReceiptBinding {
        schema_id: format!("synthetic.{name}.v1"),
        digest_sha256: digest,
    }
}

fn derived_admission(tag: u8) -> RootAdmission {
    RootAdmission {
        tier: LedgerTier::Universal,
        provenance: ProvenanceMark::Derived,
        route: AdmissionRoute::Derived(DerivedAdmission {
            ancestry_receipt: receipt("ancestry", tag),
            semantic_checker_receipt: receipt("semantic-checker", tag.wrapping_add(1)),
            independent_watchdog_receipt: receipt("independent-watchdog", tag.wrapping_add(2)),
        }),
    }
}

fn irreducible_admission(tag: u8, slot: &str, provenance: ProvenanceMark) -> RootAdmission {
    RootAdmission {
        tier: LedgerTier::Residue,
        provenance,
        route: AdmissionRoute::Irreducible(Box::new(IrreducibleAdmission {
            derivation_exhaustion_receipt: receipt("derivation-exhaustion", tag),
            buckingham_pi_receipt: receipt("buckingham-pi", tag.wrapping_add(1)),
            gap_law_receipt: receipt("gap-law", tag.wrapping_add(2)),
            chaos_protocol_receipt: receipt("chaos-protocol", tag.wrapping_add(3)),
            residual_law_receipt: receipt("residual-law", tag.wrapping_add(4)),
            residual_slot_id: slot.to_owned(),
            residual_slot_receipt: receipt("residual-slot", tag.wrapping_add(5)),
            owner_admission_receipt: receipt("owner-admission", tag.wrapping_add(6)),
            independent_watchdog_receipt: receipt("independent-watchdog", tag.wrapping_add(7)),
        })),
    }
}

fn content(kind: &str, tag: u8) -> CanonicalArtifact {
    CanonicalArtifact {
        schema_id: format!("synthetic.{kind}.v1"),
        canonical_bytes: vec![tag, tag.wrapping_mul(17), tag.rotate_left(1)],
    }
}

fn admitted(payload: ArtifactPayload, tag: u8) -> AdmittedArtifact {
    let claimed_identity =
        producer::derive_artifact_identity_for_test(&payload).expect("fixture artifact hashes");
    AdmittedArtifact {
        claimed_identity,
        admission: derived_admission(tag),
        payload,
    }
}

fn push_artifact(
    artifacts: &mut Vec<AdmittedArtifact>,
    payload: ArtifactPayload,
    tag: u8,
) -> ArtifactIdentity {
    let artifact = admitted(payload, tag);
    let identity = artifact.claimed_identity;
    artifacts.push(artifact);
    identity
}

fn build_fixture(
    base_tag: u8,
    route: FixtureRoute,
    massless: bool,
    defects: FixtureDefects,
) -> Fixture {
    let mut input = repository_input().expect("repository bindings are available");
    let mut artifacts = Vec::new();

    let mass_numerator = if defects.zero_projection { 0 } else { 7 };
    let scalar = push_artifact(
        &mut artifacts,
        ArtifactPayload::ScalarCoordinate(ScalarCoordinateArtifact {
            coordinate: content("mass-coordinate", base_tag),
            exact_value: rational(mass_numerator, 3),
            dimension: MASS_DIMENSION,
        }),
        base_tag,
    );
    let field = push_artifact(
        &mut artifacts,
        ArtifactPayload::FieldContent(content("unfamiliar-field", base_tag.wrapping_add(1))),
        base_tag.wrapping_add(1),
    );
    let operator = push_artifact(
        &mut artifacts,
        ArtifactPayload::Operator(content("unfamiliar-operator", base_tag.wrapping_add(2))),
        base_tag.wrapping_add(2),
    );
    let state = push_artifact(
        &mut artifacts,
        ArtifactPayload::StateCoordinate(content(
            "unfamiliar-state-coordinate",
            base_tag.wrapping_add(3),
        )),
        base_tag.wrapping_add(3),
    );
    let sector = push_artifact(
        &mut artifacts,
        ArtifactPayload::InteractionSector(content(
            "unfamiliar-interaction-sector",
            base_tag.wrapping_add(4),
        )),
        base_tag.wrapping_add(4),
    );
    let validity = push_artifact(
        &mut artifacts,
        ArtifactPayload::ValidityRegime(content(
            "unfamiliar-validity-regime",
            base_tag.wrapping_add(5),
        )),
        base_tag.wrapping_add(5),
    );

    let projection = push_artifact(
        &mut artifacts,
        ArtifactPayload::MassProjection(MassProjectionArtifact {
            expression: ExactExpression {
                nodes: vec![
                    ExactExpressionNode::Coordinate(scalar),
                    ExactExpressionNode::IntegerPower {
                        base: 0,
                        exponent: 0,
                    },
                    ExactExpressionNode::Subtract { left: 1, right: 1 },
                    ExactExpressionNode::Add { left: 1, right: 2 },
                    ExactExpressionNode::Multiply { left: 0, right: 3 },
                    ExactExpressionNode::Divide {
                        numerator: 4,
                        denominator: 3,
                    },
                ],
                output_node: 5,
            },
        }),
        base_tag.wrapping_add(6),
    );
    let massless_law = push_artifact(
        &mut artifacts,
        ArtifactPayload::ExactMasslessLaw(MasslessLawArtifact {
            state_coordinates: vec![state],
            active_sectors: vec![sector],
            validity_regimes: vec![validity],
        }),
        base_tag.wrapping_add(7),
    );

    let state_reference = if defects.wrong_state_kind {
        sector
    } else {
        state
    };
    let sector_reference = if defects.wrong_sector_kind {
        state
    } else {
        sector
    };
    let validity_reference = if defects.wrong_validity_kind {
        sector
    } else {
        validity
    };
    let dependencies = if defects.unexpected_dependency {
        vec![SpeciesContentIdentity([base_tag; 32])]
    } else {
        Vec::new()
    };
    let requirements = RequirementSet {
        state_coordinates: vec![state_reference],
        active_sectors: vec![sector_reference],
        validity_regimes: vec![validity_reference],
        species_dependencies: dependencies,
    };
    let stability = push_artifact(
        &mut artifacts,
        ArtifactPayload::StabilityLaw(ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
        base_tag.wrapping_add(8),
    );
    let transition = push_artifact(
        &mut artifacts,
        ArtifactPayload::TransitionLaw(ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
        base_tag.wrapping_add(9),
    );
    let mass_proof = if defects.wrong_mass_kind {
        MassProofReference::Projection(sector)
    } else if massless {
        MassProofReference::ExactMassless(massless_law)
    } else {
        MassProofReference::Projection(projection)
    };
    let blueprint = MemberBlueprint {
        physical_content: content("unfamiliar-species-state", base_tag.wrapping_add(10)),
        requirements,
        mass_proof,
        stability_law: stability,
        transition_law: transition,
    };
    let member =
        producer::derive_member_identity_for_test(&blueprint).expect("fixture member hashes");
    let rule = match route {
        FixtureRoute::Direct => {
            ArtifactPayload::DirectFloorSpecies(DirectFloorSpeciesArtifact { output: blueprint })
        }
        FixtureRoute::Elementary => {
            ArtifactPayload::ElementaryExcitation(ElementaryExcitationArtifact {
                fields: vec![field],
                operators: vec![operator],
                output: blueprint,
            })
        }
    };
    push_artifact(&mut artifacts, rule, base_tag.wrapping_add(11));
    input.admitted_artifacts = artifacts;
    input.declared_members = vec![member];
    Fixture {
        input,
        member,
        scalar,
    }
}

fn elementary_fixture(base_tag: u8) -> Fixture {
    build_fixture(
        base_tag,
        FixtureRoute::Elementary,
        false,
        FixtureDefects::default(),
    )
}

fn massless_fixture(base_tag: u8) -> Fixture {
    build_fixture(
        base_tag,
        FixtureRoute::Elementary,
        true,
        FixtureDefects::default(),
    )
}

fn composite_fixture() -> Fixture {
    let first = elementary_fixture(11);
    let second = massless_fixture(71);
    let mut input = repository_input().expect("repository bindings are available");
    let mut by_identity = std::collections::BTreeMap::new();
    for artifact in first
        .input
        .admitted_artifacts
        .into_iter()
        .chain(second.input.admitted_artifacts)
    {
        by_identity.insert(artifact.claimed_identity, artifact);
    }
    let mut artifacts = by_identity.into_values().collect::<Vec<_>>();
    let state = artifacts
        .iter()
        .find_map(|artifact| {
            matches!(&artifact.payload, ArtifactPayload::StateCoordinate(_))
                .then_some(artifact.claimed_identity)
        })
        .expect("state fixture");
    let sector = artifacts
        .iter()
        .find_map(|artifact| {
            matches!(&artifact.payload, ArtifactPayload::InteractionSector(_))
                .then_some(artifact.claimed_identity)
        })
        .expect("sector fixture");
    let validity = artifacts
        .iter()
        .find_map(|artifact| {
            matches!(&artifact.payload, ArtifactPayload::ValidityRegime(_))
                .then_some(artifact.claimed_identity)
        })
        .expect("validity fixture");
    let operator = artifacts
        .iter()
        .find_map(|artifact| {
            matches!(&artifact.payload, ArtifactPayload::Operator(_))
                .then_some(artifact.claimed_identity)
        })
        .expect("operator fixture");
    let projection = artifacts
        .iter()
        .find_map(|artifact| {
            matches!(&artifact.payload, ArtifactPayload::MassProjection(_))
                .then_some(artifact.claimed_identity)
        })
        .expect("projection fixture");
    let requirements = RequirementSet {
        state_coordinates: vec![state],
        active_sectors: vec![sector],
        validity_regimes: vec![validity],
        species_dependencies: vec![first.member, second.member],
    };
    let stability = push_artifact(
        &mut artifacts,
        ArtifactPayload::StabilityLaw(ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
        141,
    );
    let transition = push_artifact(
        &mut artifacts,
        ArtifactPayload::TransitionLaw(ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
        142,
    );
    let blueprint = MemberBlueprint {
        physical_content: content("unfamiliar-composite-state", 143),
        requirements,
        mass_proof: MassProofReference::Projection(projection),
        stability_law: stability,
        transition_law: transition,
    };
    let member =
        producer::derive_member_identity_for_test(&blueprint).expect("composite member hashes");
    push_artifact(
        &mut artifacts,
        ArtifactPayload::CompositeBoundState(CompositeBoundStateArtifact {
            constituents: vec![second.member, first.member],
            operators: vec![operator],
            output: blueprint,
        }),
        144,
    );
    input.admitted_artifacts = artifacts;
    input.declared_members = vec![member, second.member, first.member];
    Fixture {
        input,
        member,
        scalar: first.scalar,
    }
}

fn assert_both_refuse(input: &PhysicalRegistryInput, expected: PhysicalRegistryRefusalCode) {
    assert_eq!(producer::validate_and_encode(input), Err(expected));
    assert_eq!(watchdog::validate_and_encode(input), Err(expected));
    assert_eq!(inspect_physical_registry(input).unwrap_err().code, expected);
}

fn assert_both_refuse_with_caps(
    input: &PhysicalRegistryInput,
    caps: ValidationCaps,
    expected: PhysicalRegistryRefusalCode,
) {
    assert_eq!(
        producer::validate_and_encode_with_caps(input, caps),
        Err(expected)
    );
    assert_eq!(
        watchdog::validate_and_encode_with_caps(input, caps),
        Err(expected)
    );
}

fn replace_elementary_projection(
    input: &mut PhysicalRegistryInput,
    expression: ExactExpression,
) -> SpeciesContentIdentity {
    let projection_index = input
        .admitted_artifacts
        .iter()
        .position(|artifact| matches!(artifact.payload, ArtifactPayload::MassProjection(_)))
        .expect("projection fixture");
    let prior_projection = input.admitted_artifacts[projection_index].claimed_identity;
    input.admitted_artifacts[projection_index].payload =
        ArtifactPayload::MassProjection(MassProjectionArtifact { expression });
    let projection_identity = producer::derive_artifact_identity_for_test(
        &input.admitted_artifacts[projection_index].payload,
    )
    .expect("replacement projection hashes");
    input.admitted_artifacts[projection_index].claimed_identity = projection_identity;

    let rule = input
        .admitted_artifacts
        .iter_mut()
        .find(|artifact| matches!(artifact.payload, ArtifactPayload::ElementaryExcitation(_)))
        .expect("elementary rule fixture");
    let ArtifactPayload::ElementaryExcitation(rule_payload) = &mut rule.payload else {
        unreachable!("matched elementary rule")
    };
    assert_eq!(
        rule_payload.output.mass_proof,
        MassProofReference::Projection(prior_projection)
    );
    rule_payload.output.mass_proof = MassProofReference::Projection(projection_identity);
    let member = producer::derive_member_identity_for_test(&rule_payload.output)
        .expect("replacement member hashes");
    rule.claimed_identity = producer::derive_artifact_identity_for_test(&rule.payload)
        .expect("replacement rule hashes");
    input.declared_members = vec![member];
    member
}

fn exponent_one_chain(scalar: ArtifactIdentity, node_count: usize) -> ExactExpression {
    assert!(node_count >= 1);
    let mut nodes = Vec::with_capacity(node_count);
    nodes.push(ExactExpressionNode::Coordinate(scalar));
    for index in 1..node_count {
        nodes.push(ExactExpressionNode::IntegerPower {
            base: u32::try_from(index - 1).expect("bounded fixture index"),
            exponent: 1,
        });
    }
    ExactExpression {
        nodes,
        output_node: u32::try_from(node_count - 1).expect("bounded fixture output"),
    }
}

fn reverse_expression_storage(expression: &ExactExpression) -> ExactExpression {
    let length = expression.nodes.len();
    let remap = |index: u32| {
        let index = usize::try_from(index).expect("fixture index");
        u32::try_from(length - 1 - index).expect("fixture remap")
    };
    let nodes = expression
        .nodes
        .iter()
        .rev()
        .map(|node| match node {
            ExactExpressionNode::Coordinate(identity) => ExactExpressionNode::Coordinate(*identity),
            ExactExpressionNode::Add { left, right } => ExactExpressionNode::Add {
                left: remap(*left),
                right: remap(*right),
            },
            ExactExpressionNode::Subtract { left, right } => ExactExpressionNode::Subtract {
                left: remap(*left),
                right: remap(*right),
            },
            ExactExpressionNode::Multiply { left, right } => ExactExpressionNode::Multiply {
                left: remap(*left),
                right: remap(*right),
            },
            ExactExpressionNode::Divide {
                numerator,
                denominator,
            } => ExactExpressionNode::Divide {
                numerator: remap(*numerator),
                denominator: remap(*denominator),
            },
            ExactExpressionNode::IntegerPower { base, exponent } => {
                ExactExpressionNode::IntegerPower {
                    base: remap(*base),
                    exponent: *exponent,
                }
            }
        })
        .collect();
    ExactExpression {
        nodes,
        output_node: remap(expression.output_node),
    }
}

#[test]
fn repository_result_is_the_exact_non_admitting_refusal() {
    let refusal = resolve_repository_physical_species_registry().unwrap_err();
    assert_eq!(
        refusal.code,
        PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRoots
    );
    assert_eq!(refusal.code.id(), "no_admitted_species_derivation_roots");
    assert_eq!(refusal.member_count, 0);
    assert!(!refusal.coverage_claim);
    assert_eq!(refusal.authority_effect.id(), "none");
    assert_eq!(
        refusal.open_obligations,
        [
            "floor_species_property_attribution",
            "admitted_interaction_sector_membership",
            "admitted_state_coordinate_membership",
            "stable_excitation_or_bound_state_derivation",
            "complete_registry_closure_domain",
            "certified_mass_projection",
        ]
    );
    let input = repository_input().unwrap();
    assert!(input.admitted_artifacts.is_empty());
    assert!(input.declared_members.is_empty());
}

#[test]
fn all_three_routes_and_massless_unfamiliar_content_are_identity_blind() {
    assert_eq!(
        MASS_DIMENSION,
        DimensionVector([0, 1, 0, 0, 0, 0, 0]),
        "the fixed SI axis order is length, mass, time, current, temperature, amount, luminous intensity"
    );
    let direct = build_fixture(3, FixtureRoute::Direct, false, FixtureDefects::default());
    let elementary = elementary_fixture(31);
    let massless = massless_fixture(91);
    let composite = composite_fixture();
    for fixture in [&direct, &elementary, &massless, &composite] {
        let verified = inspect_physical_registry(&fixture.input).unwrap();
        assert!(!verified.members.is_empty());
        assert!(!verified.canonical_bytes.is_empty());
        assert!(verified
            .members
            .iter()
            .all(|member| !member.physical_content.canonical_bytes.is_empty()));
        assert!(verified
            .canonical_bytes
            .windows(PRODUCER_ID.len())
            .any(|window| window == PRODUCER_ID.as_bytes()));
        assert!(verified
            .canonical_bytes
            .windows(WATCHDOG_ID.len())
            .any(|window| window == WATCHDOG_ID.as_bytes()));
        assert_eq!(verified.producer_id, PRODUCER_ID);
        assert_eq!(verified.watchdog_id, WATCHDOG_ID);
        assert_eq!(verified.authority_effect.id(), "none");
    }
    let massless_result = inspect_physical_registry(&massless.input).unwrap();
    assert_eq!(massless_result.members[0].rest_mass_si.numerator_be, [0]);
    assert_eq!(massless_result.members[0].mass_dimension, MASS_DIMENSION);
    assert_eq!(
        composite
            .input
            .declared_members
            .iter()
            .filter(|identity| **identity == composite.member)
            .count(),
        1
    );
}

#[test]
fn graph_and_registry_order_do_not_select_the_result() {
    let fixture = composite_fixture();
    let forward = inspect_physical_registry(&fixture.input).unwrap();
    let mut permuted = fixture.input;
    permuted.admitted_artifacts.reverse();
    permuted.declared_members.rotate_left(1);
    let reversed = inspect_physical_registry(&permuted).unwrap();
    assert_eq!(forward.members, reversed.members);
    assert_eq!(forward.canonical_bytes, reversed.canonical_bytes);
}

#[test]
fn expression_storage_order_and_reduced_exact_arithmetic_do_not_select_acceptance() {
    let fixture = elementary_fixture(12);
    let forward_expression = exponent_one_chain(fixture.scalar, 800);
    let reverse_expression = reverse_expression_storage(&forward_expression);
    let mut forward_input = fixture.input.clone();
    replace_elementary_projection(&mut forward_input, forward_expression);
    let mut reverse_input = fixture.input;
    replace_elementary_projection(&mut reverse_input, reverse_expression);
    let forward = inspect_physical_registry(&forward_input).expect("forward graph closes");
    let reverse = inspect_physical_registry(&reverse_input).expect("reverse graph closes");
    assert_eq!(forward.members, reverse.members);
    assert_eq!(forward.canonical_bytes, reverse.canonical_bytes);

    let mut reduced = elementary_fixture(13).input;
    let dimensionless = push_artifact(
        &mut reduced.admitted_artifacts,
        ArtifactPayload::ScalarCoordinate(ScalarCoordinateArtifact {
            coordinate: content("large-odd-denominator", 238),
            exact_value: ExactRationalWire {
                negative: false,
                numerator_be: vec![1],
                denominator_be: vec![0x80, 0, 0, 1],
            },
            dimension: DimensionVector([0; 7]),
        }),
        238,
    );
    let mass = reduced
        .admitted_artifacts
        .iter()
        .find_map(|artifact| {
            matches!(artifact.payload, ArtifactPayload::ScalarCoordinate(_))
                .then_some(artifact.claimed_identity)
        })
        .expect("mass coordinate fixture");
    replace_elementary_projection(
        &mut reduced,
        ExactExpression {
            nodes: vec![
                ExactExpressionNode::Coordinate(mass),
                ExactExpressionNode::Coordinate(dimensionless),
                ExactExpressionNode::IntegerPower {
                    base: 1,
                    exponent: 10,
                },
                ExactExpressionNode::Add { left: 2, right: 2 },
                ExactExpressionNode::Multiply { left: 0, right: 3 },
            ],
            output_node: 4,
        },
    );
    let caps = ValidationCaps {
        intermediate_component_bits: 500,
        ..ValidationCaps::PRODUCTION
    };
    let produced =
        producer::validate_and_encode_with_caps(&reduced, caps).expect("producer cross-reduces");
    let watched =
        watchdog::validate_and_encode_with_caps(&reduced, caps).expect("watchdog cross-reduces");
    assert_eq!(produced, watched);
}

#[test]
fn a_name_only_mass_coordinate_and_evidence_only_citation_never_create_membership() {
    let fixture = elementary_fixture(17);
    let scalar = fixture
        .input
        .admitted_artifacts
        .iter()
        .find(|artifact| artifact.claimed_identity == fixture.scalar)
        .unwrap()
        .clone();
    let mut name_only = repository_input().unwrap();
    let mut scalar = scalar;
    let ArtifactPayload::ScalarCoordinate(coordinate) = &mut scalar.payload else {
        panic!("fixture scalar");
    };
    coordinate.coordinate.canonical_bytes = b"m_e".to_vec();
    scalar.claimed_identity =
        producer::derive_artifact_identity_for_test(&scalar.payload).expect("name-only hash");
    name_only.admitted_artifacts = vec![scalar];
    assert_both_refuse(
        &name_only,
        PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRoots,
    );

    let mut cited = fixture.input;
    cited.admitted_artifacts[0].admission = RootAdmission {
        tier: LedgerTier::Reference,
        provenance: ProvenanceMark::Closure,
        route: AdmissionRoute::EvidenceCustodyOnly {
            source_receipt: receipt("held-source", 219),
        },
    };
    assert_both_refuse(
        &cited,
        PhysicalRegistryRefusalCode::EvidenceCustodyIsNotAdmission,
    );
}

#[test]
fn complete_two_route_admission_is_enforced_independently_of_accounting_marks() {
    assert_eq!(
        [
            ProvenanceMark::Derived,
            ProvenanceMark::Measured,
            ProvenanceMark::Estimator,
            ProvenanceMark::Closure,
            ProvenanceMark::Authored,
            ProvenanceMark::WrittenState,
            ProvenanceMark::Contingency,
        ]
        .map(|provenance| {
            provenance
                .bracket_tag()
                .expect("canonical provenance has a bracket tag")
        }),
        ["[D]", "[M]", "[E]", "[C]", "[A]", "[W]", "[X]"]
    );
    assert_eq!(
        [
            LedgerTier::Universal,
            LedgerTier::Reference,
            LedgerTier::Residue,
            LedgerTier::Contingency,
        ]
        .map(LedgerTier::id),
        ["universal", "reference", "residue", "contingency"]
    );

    let mut admitted_citation = elementary_fixture(21).input;
    admitted_citation.admitted_artifacts[0].admission =
        irreducible_admission(201, "synthetic.slot.201", ProvenanceMark::Closure);
    inspect_physical_registry(&admitted_citation).unwrap();

    let mut wrong_derived_mark = elementary_fixture(22).input;
    wrong_derived_mark.admitted_artifacts[0]
        .admission
        .provenance = ProvenanceMark::Measured;
    assert_both_refuse(
        &wrong_derived_mark,
        PhysicalRegistryRefusalCode::DerivedAdmissionProvenanceMismatch,
    );

    let mut generated = elementary_fixture(23).input;
    generated.admitted_artifacts[0].admission.provenance = ProvenanceMark::WrittenState;
    assert_both_refuse(
        &generated,
        PhysicalRegistryRefusalCode::GeneratedProvenanceCannotBeRoot,
    );

    let mut duplicate_slot = elementary_fixture(24).input;
    duplicate_slot.admitted_artifacts[0].admission =
        irreducible_admission(210, "synthetic.same-slot", ProvenanceMark::Measured);
    duplicate_slot.admitted_artifacts[1].admission =
        irreducible_admission(220, "synthetic.same-slot", ProvenanceMark::Estimator);
    assert_both_refuse(
        &duplicate_slot,
        PhysicalRegistryRefusalCode::DuplicateResidualSlot,
    );

    let mut duplicate_receipt = elementary_fixture(25).input;
    let AdmissionRoute::Derived(route) =
        &mut duplicate_receipt.admitted_artifacts[0].admission.route
    else {
        panic!("derived fixture admission")
    };
    route.semantic_checker_receipt = route.ancestry_receipt.clone();
    assert_both_refuse(
        &duplicate_receipt,
        PhysicalRegistryRefusalCode::DuplicateAdmissionReceipt,
    );

    let mut noncanonical = elementary_fixture(26).input;
    noncanonical.admitted_artifacts[0].admission.provenance =
        ProvenanceMark::UnverifiedMeasurementCandidate;
    assert_both_refuse(
        &noncanonical,
        PhysicalRegistryRefusalCode::NoncanonicalProvenance,
    );
}

#[test]
fn closure_omission_addition_and_descriptor_collision_refuse() {
    let mut missing = elementary_fixture(27).input;
    missing.declared_members.clear();
    assert_both_refuse(&missing, PhysicalRegistryRefusalCode::MissingClosureMember);

    let mut extra = elementary_fixture(28).input;
    extra
        .declared_members
        .push(SpeciesContentIdentity([249; 32]));
    assert_both_refuse(&extra, PhysicalRegistryRefusalCode::ExtraClosureMember);

    let mut collision = elementary_fixture(29).input;
    let mut second = collision.admitted_artifacts[0].clone();
    second.payload = ArtifactPayload::FieldContent(content("collision-payload", 250));
    collision.admitted_artifacts.push(second);
    assert_both_refuse(
        &collision,
        PhysicalRegistryRefusalCode::ArtifactIdentityCollision,
    );

    let mut dangling = composite_fixture();
    let prior_composite = dangling.member;
    let unknown = SpeciesContentIdentity([247; 32]);
    let composite_index = dangling
        .input
        .admitted_artifacts
        .iter()
        .position(|artifact| matches!(artifact.payload, ArtifactPayload::CompositeBoundState(_)))
        .expect("composite rule fixture");
    let (prior_stability, prior_transition) =
        match &dangling.input.admitted_artifacts[composite_index].payload {
            ArtifactPayload::CompositeBoundState(rule) => {
                (rule.output.stability_law, rule.output.transition_law)
            }
            _ => unreachable!("matched composite rule"),
        };
    let mut replacement_laws = Vec::new();
    for prior in [prior_stability, prior_transition] {
        let law_index = dangling
            .input
            .admitted_artifacts
            .iter()
            .position(|artifact| artifact.claimed_identity == prior)
            .expect("composite law fixture");
        match &mut dangling.input.admitted_artifacts[law_index].payload {
            ArtifactPayload::StabilityLaw(law) | ArtifactPayload::TransitionLaw(law) => {
                law.requirements.species_dependencies = vec![unknown];
            }
            _ => panic!("composite law kind"),
        }
        let identity = producer::derive_artifact_identity_for_test(
            &dangling.input.admitted_artifacts[law_index].payload,
        )
        .expect("replacement law hashes");
        dangling.input.admitted_artifacts[law_index].claimed_identity = identity;
        replacement_laws.push(identity);
    }
    let composite_rule = &mut dangling.input.admitted_artifacts[composite_index];
    let ArtifactPayload::CompositeBoundState(rule) = &mut composite_rule.payload else {
        unreachable!("matched composite rule")
    };
    rule.constituents = vec![unknown];
    rule.output.requirements.species_dependencies = vec![unknown];
    rule.output.stability_law = replacement_laws[0];
    rule.output.transition_law = replacement_laws[1];
    composite_rule.claimed_identity =
        producer::derive_artifact_identity_for_test(&composite_rule.payload)
            .expect("dangling rule hashes");
    dangling
        .input
        .declared_members
        .retain(|identity| *identity != prior_composite);
    assert_both_refuse(
        &dangling.input,
        PhysicalRegistryRefusalCode::UnknownSpeciesDependency,
    );
}

#[test]
fn mass_state_sector_validity_and_dependency_swaps_refuse() {
    for (defects, expected) in [
        (
            FixtureDefects {
                wrong_mass_kind: true,
                ..FixtureDefects::default()
            },
            PhysicalRegistryRefusalCode::ArtifactKindMismatch,
        ),
        (
            FixtureDefects {
                wrong_state_kind: true,
                ..FixtureDefects::default()
            },
            PhysicalRegistryRefusalCode::ArtifactKindMismatch,
        ),
        (
            FixtureDefects {
                wrong_sector_kind: true,
                ..FixtureDefects::default()
            },
            PhysicalRegistryRefusalCode::ArtifactKindMismatch,
        ),
        (
            FixtureDefects {
                wrong_validity_kind: true,
                ..FixtureDefects::default()
            },
            PhysicalRegistryRefusalCode::ArtifactKindMismatch,
        ),
        (
            FixtureDefects {
                unexpected_dependency: true,
                ..FixtureDefects::default()
            },
            PhysicalRegistryRefusalCode::DependencyMismatch,
        ),
    ] {
        let fixture = build_fixture(37, FixtureRoute::Elementary, false, defects);
        assert_both_refuse(&fixture.input, expected);
    }
}

#[test]
fn exact_zero_needs_a_massless_law_and_expression_cycles_refuse() {
    let zero = build_fixture(
        41,
        FixtureRoute::Elementary,
        false,
        FixtureDefects {
            zero_projection: true,
            ..FixtureDefects::default()
        },
    );
    assert_both_refuse(&zero.input, PhysicalRegistryRefusalCode::NonPositiveMass);

    let mut cyclic = elementary_fixture(42).input;
    let projection = cyclic
        .admitted_artifacts
        .iter_mut()
        .find(|artifact| matches!(&artifact.payload, ArtifactPayload::MassProjection(_)))
        .unwrap();
    projection.payload = ArtifactPayload::MassProjection(MassProjectionArtifact {
        expression: ExactExpression {
            nodes: vec![ExactExpressionNode::Add { left: 0, right: 0 }],
            output_node: 0,
        },
    });
    assert_both_refuse(&cyclic, PhysicalRegistryRefusalCode::ExpressionCycle);

    let mut too_deep = elementary_fixture(43).input;
    let projection = too_deep
        .admitted_artifacts
        .iter_mut()
        .find(|artifact| matches!(artifact.payload, ArtifactPayload::MassProjection(_)))
        .expect("projection fixture");
    projection.payload = ArtifactPayload::MassProjection(MassProjectionArtifact {
        expression: exponent_one_chain(ArtifactIdentity([1; 32]), 1_026),
    });
    assert_both_refuse(
        &too_deep,
        PhysicalRegistryRefusalCode::ExpressionDepthExceeded,
    );
}

#[test]
fn floor_schema_and_every_resource_domain_fail_closed() {
    let fixture = elementary_fixture(51);
    let mut floor = fixture.input.clone();
    floor.floor_binding.digest_sha256[0] ^= 1;
    assert_both_refuse(&floor, PhysicalRegistryRefusalCode::FloorBindingMismatch);

    let mut schema = fixture.input.clone();
    schema
        .structure_binding
        .interaction_sector_registry_schema_id
        .push_str(".changed");
    assert_both_refuse(
        &schema,
        PhysicalRegistryRefusalCode::StructureBindingMismatch,
    );

    let cases = [
        (
            ValidationCaps {
                artifact_count: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ArtifactCapacityExceeded,
        ),
        (
            ValidationCaps {
                registry_member_count: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::RegistryCapacityExceeded,
        ),
        (
            ValidationCaps {
                references_per_artifact: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ReferenceCapacityExceeded,
        ),
        (
            ValidationCaps {
                total_reference_count: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ReferenceCapacityExceeded,
        ),
        (
            ValidationCaps {
                expression_node_count: 1,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded,
        ),
        (
            ValidationCaps {
                expression_edge_count: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ExpressionEdgeCapacityExceeded,
        ),
        (
            ValidationCaps {
                expression_depth: 1,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ExpressionDepthExceeded,
        ),
        (
            ValidationCaps {
                rational_component_bits: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::RationalComponentLimitExceeded,
        ),
        (
            ValidationCaps {
                intermediate_component_bits: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::IntermediateComponentLimitExceeded,
        ),
        (
            ValidationCaps {
                dimension_abs_exponent: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded,
        ),
        (
            ValidationCaps {
                evaluation_steps: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded,
        ),
        (
            ValidationCaps {
                closure_steps: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ClosureStepLimitExceeded,
        ),
        (
            ValidationCaps {
                canonical_bytes: 1,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded,
        ),
        (
            ValidationCaps {
                canonical_token_bytes: 1,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::CanonicalTextInvalid,
        ),
        (
            ValidationCaps {
                content_bytes: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::ContentByteLimitExceeded,
        ),
    ];
    for (caps, expected) in cases {
        assert_both_refuse_with_caps(&fixture.input, caps, expected);
    }
}

#[test]
fn physical_pair_has_no_reducer_or_authority_minting_surface() {
    let production = [
        include_str!("mod.rs"),
        include_str!("model.rs"),
        include_str!("producer.rs"),
        include_str!("watchdog.rs"),
    ]
    .join("\n");
    for forbidden in [
        concat!("SpeciesRegistry", "AuthoritySeal"),
        concat!("derive_mean_", "particle_mass("),
        concat!("resolve_repository_", "species_state_support("),
        concat!("VerifiedSpecies", "StateSupport {"),
    ] {
        assert!(
            !production.contains(forbidden),
            "physical registry contains forbidden authority surface {forbidden}"
        );
    }
}
