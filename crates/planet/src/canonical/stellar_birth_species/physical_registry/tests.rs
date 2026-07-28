use super::super::SpeciesContentIdentity;
use super::{
    inspect_physical_registry, model::*, producer, repository_input,
    repository_physical_registry_frontier, resolve_repository_physical_species_registry,
    root_admission_census, watchdog,
};
use civsim_units::{bignum::BigUint, digest::sha256};

#[derive(Debug, Clone, Copy)]
enum FixtureDerivation {
    FloorLinked,
    Unfamiliar,
}

#[derive(Debug, Clone, Copy, Default)]
struct FixtureDefects {
    wrong_mass_kind: bool,
    wrong_state_kind: bool,
    wrong_sector_kind: bool,
    wrong_validity_kind: bool,
    unexpected_dependency: bool,
    zero_projection: bool,
    massless_subject_outside_requirements: bool,
    massless_symmetry_outside_requirements: bool,
    massless_subject_equals_symmetry: bool,
    massless_duplicate_receipts: bool,
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
    AdmittedArtifact::from_exact_test_recomputation(
        claimed_identity,
        derived_admission(tag),
        payload,
    )
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

fn refresh_vocabulary_binding(input: &mut PhysicalRegistryInput) {
    input.vocabulary_binding = super::vocabulary::derive_binding(&input.admitted_artifacts)
        .expect("synthetic admitted roots classify");
}

fn descriptor(artifacts: &mut Vec<AdmittedArtifact>, name: &str, tag: u8) -> ArtifactIdentity {
    push_artifact(
        artifacts,
        ArtifactPayload::PhysicalDescriptor(content(name, tag)),
        tag,
    )
}

fn relation(role: ArtifactIdentity, target: ArtifactIdentity) -> ArtifactRelation {
    ArtifactRelation { role, target }
}

fn build_fixture(
    base_tag: u8,
    derivation: FixtureDerivation,
    massless: bool,
    defects: FixtureDefects,
) -> Fixture {
    let mut input = repository_input().expect("repository bindings are available");
    let mut artifacts = Vec::new();

    let mass_numerator = if defects.zero_projection { 0 } else { 7 };
    let scalar = push_artifact(
        &mut artifacts,
        ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
            coordinate: content("mass-coordinate", base_tag),
            exact_value: rational(mass_numerator, 3),
            dimension: MASS_DIMENSION,
        })),
        base_tag,
    );
    let derivation_kind = descriptor(
        &mut artifacts,
        match derivation {
            FixtureDerivation::FloorLinked => "floor-linked-derivation-kind",
            FixtureDerivation::Unfamiliar => "thaumic-law-derivation-kind",
        },
        base_tag.wrapping_add(1),
    );
    let source_role = descriptor(
        &mut artifacts,
        "floor-coordinate-input-role",
        base_tag.wrapping_add(2),
    );
    let field_role = descriptor(
        &mut artifacts,
        "unfamiliar-field-input-role",
        base_tag.wrapping_add(3),
    );
    let field = descriptor(&mut artifacts, "unfamiliar-field", base_tag.wrapping_add(4));
    let operator_role = descriptor(
        &mut artifacts,
        "unfamiliar-operator-input-role",
        base_tag.wrapping_add(5),
    );
    let operator = descriptor(
        &mut artifacts,
        "unfamiliar-operator",
        base_tag.wrapping_add(6),
    );
    let state_role = descriptor(
        &mut artifacts,
        "state-coordinate-requirement-role",
        base_tag.wrapping_add(7),
    );
    let state = descriptor(
        &mut artifacts,
        "unfamiliar-state-coordinate",
        base_tag.wrapping_add(8),
    );
    let sector_role = descriptor(
        &mut artifacts,
        "interaction-sector-requirement-role",
        base_tag.wrapping_add(9),
    );
    let sector = descriptor(
        &mut artifacts,
        "unfamiliar-interaction-sector",
        base_tag.wrapping_add(10),
    );
    let validity_role = descriptor(
        &mut artifacts,
        "validity-regime-requirement-role",
        base_tag.wrapping_add(11),
    );
    let validity = descriptor(
        &mut artifacts,
        "unfamiliar-validity-regime",
        base_tag.wrapping_add(12),
    );
    let stability_role = descriptor(
        &mut artifacts,
        "stability-constraint-role",
        base_tag.wrapping_add(13),
    );
    let transition_role = descriptor(
        &mut artifacts,
        "transition-constraint-role",
        base_tag.wrapping_add(14),
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
            scope: MassProjectionScope::SpeciesRestMass,
            uncertainty_transport: Some(MassUncertaintyTransportProof {
                sources: vec![MassUncertaintySourceProof {
                    source_coordinate: scalar,
                    source_pair_receipt: receipt(
                        "test-mass-source-pair",
                        base_tag.wrapping_add(30),
                    ),
                }],
                producer_receipt: receipt(
                    "test-mass-uncertainty-producer",
                    base_tag.wrapping_add(31),
                ),
                watchdog_receipt: receipt(
                    "test-mass-uncertainty-watchdog",
                    base_tag.wrapping_add(32),
                ),
            }),
        }),
        base_tag.wrapping_add(15),
    );

    let state_relation_role = if defects.wrong_state_kind {
        scalar
    } else {
        state_role
    };
    let sector_relation_role = if defects.wrong_sector_kind {
        scalar
    } else {
        sector_role
    };
    let validity_relation_role = if defects.wrong_validity_kind {
        scalar
    } else {
        validity_role
    };
    let dependencies = if defects.unexpected_dependency {
        vec![SpeciesContentIdentity([base_tag; 32])]
    } else {
        Vec::new()
    };
    let requirements = RequirementSet {
        artifact_relations: vec![
            relation(field_role, field),
            relation(state_relation_role, state),
            relation(sector_relation_role, sector),
            relation(validity_relation_role, validity),
        ],
        species_dependencies: dependencies,
    };
    let massless_subject = if defects.massless_subject_outside_requirements {
        operator
    } else {
        field
    };
    let massless_symmetry = if defects.massless_subject_equals_symmetry {
        massless_subject
    } else if defects.massless_symmetry_outside_requirements {
        operator
    } else {
        sector
    };
    let excluded_term = content("rest-mass-term", base_tag.wrapping_add(20));
    let exclusion_producer_receipt =
        receipt("massless-exclusion-producer", base_tag.wrapping_add(22));
    let exclusion_watchdog_receipt = if defects.massless_duplicate_receipts {
        exclusion_producer_receipt.clone()
    } else {
        receipt("massless-exclusion-watchdog", base_tag.wrapping_add(23))
    };
    let massless_law = push_artifact(
        &mut artifacts,
        ArtifactPayload::ExactMasslessLaw(MasslessLawArtifact {
            requirements: requirements.clone(),
            proof: ExactZeroMassProof {
                subject: massless_subject,
                excluded_term,
                symmetry: massless_symmetry,
                applicability_receipt: receipt("massless-applicability", base_tag.wrapping_add(21)),
                exclusion_producer_receipt,
                exclusion_watchdog_receipt,
            },
        }),
        base_tag.wrapping_add(16),
    );
    let shared_constraint = push_artifact(
        &mut artifacts,
        ArtifactPayload::ConstraintLaw(ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
        base_tag.wrapping_add(17),
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
        constraint_laws: vec![
            relation(stability_role, shared_constraint),
            relation(transition_role, shared_constraint),
        ],
    };
    let member =
        producer::derive_member_identity_for_test(&blueprint).expect("fixture member hashes");
    let artifact_inputs = match derivation {
        FixtureDerivation::FloorLinked => vec![relation(source_role, scalar)],
        FixtureDerivation::Unfamiliar => vec![
            relation(field_role, field),
            relation(operator_role, operator),
        ],
    };
    push_artifact(
        &mut artifacts,
        ArtifactPayload::SpeciesDerivation(SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs,
            constituents: Vec::new(),
            output: blueprint,
        }),
        base_tag.wrapping_add(19),
    );
    input.admitted_artifacts = artifacts;
    refresh_vocabulary_binding(&mut input);
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
        FixtureDerivation::Unfamiliar,
        false,
        FixtureDefects::default(),
    )
}

fn massless_fixture(base_tag: u8) -> Fixture {
    build_fixture(
        base_tag,
        FixtureDerivation::Unfamiliar,
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
    let projection = artifacts
        .iter()
        .find_map(|artifact| {
            matches!(&artifact.payload, ArtifactPayload::MassProjection(_))
                .then_some(artifact.claimed_identity)
        })
        .expect("projection fixture");
    let derivation_kind = descriptor(&mut artifacts, "bound-aggregate-derivation-kind", 141);
    let operator_role = descriptor(&mut artifacts, "aggregate-operator-input-role", 142);
    let operator = descriptor(&mut artifacts, "unfamiliar-aggregate-operator", 143);
    let state_role = descriptor(&mut artifacts, "aggregate-state-role", 144);
    let state = descriptor(&mut artifacts, "aggregate-state", 145);
    let sector_role = descriptor(&mut artifacts, "aggregate-sector-role", 146);
    let sector = descriptor(&mut artifacts, "aggregate-sector", 147);
    let validity_role = descriptor(&mut artifacts, "aggregate-validity-role", 148);
    let validity = descriptor(&mut artifacts, "aggregate-validity", 149);
    let stability_role = descriptor(&mut artifacts, "aggregate-stability-role", 150);
    let transition_role = descriptor(&mut artifacts, "aggregate-transition-role", 151);
    let requirements = RequirementSet {
        artifact_relations: vec![
            relation(state_role, state),
            relation(sector_role, sector),
            relation(validity_role, validity),
        ],
        species_dependencies: vec![first.member, second.member],
    };
    let shared_constraint = push_artifact(
        &mut artifacts,
        ArtifactPayload::ConstraintLaw(ConstraintLawArtifact {
            requirements: requirements.clone(),
        }),
        152,
    );
    let blueprint = MemberBlueprint {
        physical_content: content("unfamiliar-composite-state", 154),
        requirements,
        mass_proof: MassProofReference::Projection(projection),
        constraint_laws: vec![
            relation(stability_role, shared_constraint),
            relation(transition_role, shared_constraint),
        ],
    };
    let member =
        producer::derive_member_identity_for_test(&blueprint).expect("composite member hashes");
    push_artifact(
        &mut artifacts,
        ArtifactPayload::SpeciesDerivation(SpeciesDerivationArtifact {
            derivation_kind,
            artifact_inputs: vec![relation(operator_role, operator)],
            constituents: vec![second.member, first.member],
            output: blueprint,
        }),
        155,
    );
    input.admitted_artifacts = artifacts;
    refresh_vocabulary_binding(&mut input);
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

fn minimum_evaluation_budget(input: &PhysicalRegistryInput, producer_path: bool) -> u64 {
    let mut lower = 0_u64;
    let mut upper = ValidationCaps::PRODUCTION.evaluation_steps;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let caps = ValidationCaps {
            evaluation_steps: middle,
            ..ValidationCaps::PRODUCTION
        };
        let result = if producer_path {
            producer::validate_and_encode_with_caps(input, caps)
        } else {
            watchdog::validate_and_encode_with_caps(input, caps)
        };
        match result {
            Ok(_) => upper = middle,
            Err(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded) => lower = middle + 1,
            Err(other) => panic!("unexpected evaluation-budget refusal: {}", other.id()),
        }
    }
    lower
}

fn minimum_closure_budget(input: &PhysicalRegistryInput, producer_path: bool) -> u64 {
    let mut lower = 0_u64;
    let mut upper = ValidationCaps::PRODUCTION.closure_steps;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let caps = ValidationCaps {
            closure_steps: middle,
            ..ValidationCaps::PRODUCTION
        };
        let result = if producer_path {
            producer::validate_and_encode_with_caps(input, caps)
        } else {
            watchdog::validate_and_encode_with_caps(input, caps)
        };
        match result {
            Ok(_) => upper = middle,
            Err(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded) => lower = middle + 1,
            Err(other) => panic!("unexpected closure-budget refusal: {}", other.id()),
        }
    }
    lower
}

fn replace_elementary_projection(
    input: &mut PhysicalRegistryInput,
    expression: ExactExpression,
) -> SpeciesContentIdentity {
    replace_elementary_projection_with_scope(
        input,
        expression,
        MassProjectionScope::SpeciesRestMass,
    )
}

fn replace_elementary_projection_with_scope(
    input: &mut PhysicalRegistryInput,
    expression: ExactExpression,
    scope: MassProjectionScope,
) -> SpeciesContentIdentity {
    let source_coordinates = expression
        .nodes
        .iter()
        .filter_map(|node| match node {
            ExactExpressionNode::Coordinate(identity) => Some(*identity),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let uncertainty_transport = match scope {
        MassProjectionScope::MembershipNeutral => None,
        MassProjectionScope::SpeciesRestMass => Some(MassUncertaintyTransportProof {
            sources: source_coordinates
                .into_iter()
                .map(|source_coordinate| {
                    let mut receipt_bytes = b"synthetic.replacement-mass-source-pair.v1".to_vec();
                    receipt_bytes.extend_from_slice(&source_coordinate.0);
                    MassUncertaintySourceProof {
                        source_coordinate,
                        source_pair_receipt: ReceiptBinding {
                            schema_id: "synthetic.replacement-mass-source-pair.v1".to_owned(),
                            digest_sha256: sha256(&receipt_bytes),
                        },
                    }
                })
                .collect(),
            producer_receipt: receipt("replacement-mass-uncertainty-producer", 212),
            watchdog_receipt: receipt("replacement-mass-uncertainty-watchdog", 213),
        }),
    };
    replace_elementary_projection_with_transport(input, expression, scope, uncertainty_transport)
}

fn replace_elementary_projection_with_transport(
    input: &mut PhysicalRegistryInput,
    expression: ExactExpression,
    scope: MassProjectionScope,
    uncertainty_transport: Option<MassUncertaintyTransportProof>,
) -> SpeciesContentIdentity {
    let projection_index = input
        .admitted_artifacts
        .iter()
        .position(|artifact| matches!(artifact.payload, ArtifactPayload::MassProjection(_)))
        .expect("projection fixture");
    let prior_projection = input.admitted_artifacts[projection_index].claimed_identity;
    input.admitted_artifacts[projection_index].payload =
        ArtifactPayload::MassProjection(MassProjectionArtifact {
            expression,
            scope,
            uncertainty_transport,
        });
    let projection_identity = producer::derive_artifact_identity_for_test(
        &input.admitted_artifacts[projection_index].payload,
    )
    .expect("replacement projection hashes");
    input.admitted_artifacts[projection_index].claimed_identity = projection_identity;
    input.admitted_artifacts[projection_index].refresh_exact_test_capability();

    let rule = input
        .admitted_artifacts
        .iter_mut()
        .find(|artifact| {
            matches!(
                &artifact.payload,
                ArtifactPayload::SpeciesDerivation(rule)
                    if rule.output.mass_proof == MassProofReference::Projection(prior_projection)
            )
        })
        .expect("species derivation fixture");
    let ArtifactPayload::SpeciesDerivation(rule_payload) = &mut rule.payload else {
        unreachable!("matched species derivation rule")
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
    rule.refresh_exact_test_capability();
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
fn repository_result_closes_four_local_members_without_global_authority() {
    let registry = resolve_repository_physical_species_registry().expect("local profile closes");
    assert_eq!(registry.members.len(), 4);
    assert_eq!(registry.authority_effect.id(), "none");
    let frontier = repository_physical_registry_frontier().expect("repository frontier");
    assert_eq!(frontier.registry_refusal_code, "none");
    assert_eq!(frontier.registry_member_count, 4);
    assert!(!frontier.registry_coverage_claim);
    assert_eq!(frontier.registry_authority_effect, "none");
    assert_eq!(frontier.admitted_root_count, 4);
    assert_eq!(frontier.admitted_artifact_count, 105);
    assert_eq!(frontier.primitive_profile.artifact_count, 29);
    assert_eq!(frontier.primitive_profile.admission_census.len(), 29);
    assert_eq!(
        frontier
            .primitive_profile
            .admission_census
            .iter()
            .filter(|row| row.provenance_tag == "[A]" && row.route_id == "irreducible")
            .count(),
        1
    );
    assert_eq!(
        frontier
            .primitive_profile
            .admission_census
            .iter()
            .filter(|row| row.provenance_tag == "[D]" && row.route_id == "derived")
            .count(),
        28
    );
    assert!(registry
        .members
        .iter()
        .any(|member| member.identity.0 == frontier.primitive_profile.member_sha256));
    assert_eq!(frontier.primitive_profile.symmetry_basis_element_count, 10);
    assert_eq!(
        frontier.primitive_profile.symmetry_excluded_operator_count,
        1
    );
    assert_eq!(
        frontier.primitive_profile.derive_first_status_id,
        "executed_open_frontier"
    );
    assert_eq!(
        frontier.primitive_profile.buckingham_pi_status_id,
        "semantic_inapplicability_paired"
    );
    assert_eq!(
        frontier.primitive_profile.gap_law_status_id,
        "executed_and_bound"
    );
    assert_eq!(
        frontier.primitive_profile.chaos_protocol_status_id,
        "nondynamical_inapplicability_paired"
    );
    assert_eq!(
        frontier.primitive_profile.residual_law_status_id,
        "executed_and_bound"
    );
    assert_eq!(
        frontier.primitive_profile.residual_slot_status_id,
        "collision_checked_unique"
    );
    assert_eq!(frontier.charged_profile.artifact_count, 39);
    assert_eq!(frontier.charged_profile.admission_census.len(), 39);
    assert_eq!(frontier.charged_profile.member_sha256.len(), 2);
    assert_eq!(
        frontier
            .charged_profile
            .admission_census
            .iter()
            .filter(|row| row.provenance_tag == "[A]" && row.route_id == "irreducible")
            .count(),
        1
    );
    assert_eq!(
        frontier
            .charged_profile
            .admission_census
            .iter()
            .filter(|row| row.provenance_tag == "[D]" && row.route_id == "derived")
            .count(),
        38
    );
    assert!(frontier
        .charged_profile
        .member_sha256
        .iter()
        .all(|identity| registry
            .members
            .iter()
            .any(|member| member.identity.0 == *identity)));
    assert_ne!(
        frontier.charged_profile.charge_conjugation_producer_sha256,
        frontier.charged_profile.charge_conjugation_watchdog_sha256
    );
    assert_ne!(
        frontier.charged_profile.mass_transport_producer_sha256,
        frontier.charged_profile.mass_transport_watchdog_sha256
    );
    assert_eq!(frontier.neutral_bound_profile.artifact_count, 33);
    assert_eq!(frontier.neutral_bound_profile.admission_census.len(), 33);
    assert_eq!(
        frontier
            .neutral_bound_profile
            .admission_census
            .iter()
            .filter(|row| row.provenance_tag == "[A]" && row.route_id == "irreducible")
            .count(),
        1
    );
    assert_eq!(
        frontier
            .neutral_bound_profile
            .admission_census
            .iter()
            .filter(|row| row.provenance_tag == "[D]" && row.route_id == "derived")
            .count(),
        32
    );
    assert!(registry
        .members
        .iter()
        .any(|member| member.identity.0 == frontier.neutral_bound_profile.member_sha256));
    assert_eq!(
        frontier.neutral_bound_profile.binding_disposition_id,
        "strictly_below_free_constituent_threshold"
    );
    assert_eq!(
        frontier.neutral_bound_profile.decay_disposition_id,
        "energetically_open_neutral_massless_carrier_family"
    );
    assert!(!frontier.neutral_bound_profile.conditioned_support_authority);
    assert!(!frontier.neutral_bound_profile.global_stability_claim);
    for (producer, watchdog) in [
        (
            frontier.neutral_bound_profile.solver_producer_sha256,
            frontier.neutral_bound_profile.solver_watchdog_sha256,
        ),
        (
            frontier.neutral_bound_profile.normalization_producer_sha256,
            frontier.neutral_bound_profile.normalization_watchdog_sha256,
        ),
        (
            frontier
                .neutral_bound_profile
                .threshold_coverage_producer_sha256,
            frontier
                .neutral_bound_profile
                .threshold_coverage_watchdog_sha256,
        ),
        (
            frontier
                .neutral_bound_profile
                .uncertainty_transport_producer_sha256,
            frontier
                .neutral_bound_profile
                .uncertainty_transport_watchdog_sha256,
        ),
        (
            frontier.neutral_bound_profile.conservation_producer_sha256,
            frontier.neutral_bound_profile.conservation_watchdog_sha256,
        ),
    ] {
        assert_ne!(producer, watchdog);
    }
    assert_eq!(frontier.root_admission_census.len(), 4);
    assert!(frontier.root_admission_census.iter().all(|admission| {
        admission.tier_id == "universal"
            && admission.provenance_tag == "[D]"
            && admission.route_id == "derived"
    }));
    let input = repository_input().unwrap();
    assert_eq!(
        input
            .admitted_artifacts
            .iter()
            .filter(|artifact| {
                matches!(&artifact.payload, ArtifactPayload::ScalarCoordinate(_))
            })
            .count(),
        5
    );
    assert_eq!(
        input
            .admitted_artifacts
            .iter()
            .filter(|artifact| {
                matches!(
                    &artifact.payload,
                    ArtifactPayload::MassProjection(MassProjectionArtifact {
                        scope: MassProjectionScope::MembershipNeutral,
                        ..
                    })
                )
            })
            .count(),
        1
    );
    assert_eq!(
        input
            .admitted_artifacts
            .iter()
            .filter(|artifact| {
                artifact.admission_capability_kind() == AdmissionCapabilityKind::RepositoryRoot
            })
            .count(),
        4
    );
    assert_eq!(
        input
            .admitted_artifacts
            .iter()
            .filter(|artifact| {
                artifact.admission_capability_kind() == AdmissionCapabilityKind::PrimitiveProfile
            })
            .count(),
        29
    );
    assert_eq!(
        input
            .admitted_artifacts
            .iter()
            .filter(|artifact| {
                artifact.admission_capability_kind() == AdmissionCapabilityKind::ChargedProfile
            })
            .count(),
        39
    );
    assert_eq!(
        input
            .admitted_artifacts
            .iter()
            .filter(|artifact| {
                artifact.admission_capability_kind() == AdmissionCapabilityKind::NeutralBoundProfile
            })
            .count(),
        33
    );
    assert_eq!(input.declared_members.len(), 4);
    assert_eq!(input.vocabulary_binding.root_count, 105);
    assert!(!input
        .vocabulary_binding
        .descriptor_role_identities
        .is_empty());
    assert_eq!(
        input.vocabulary_binding.relation_target_identities.len(),
        105
    );
    assert!(!input
        .vocabulary_binding
        .constraint_law_identities
        .is_empty());
    assert!(input.vocabulary_binding.current_input_partition_complete);
    assert!(!input.vocabulary_binding.global_physical_vocabulary_coverage);
    assert!(!input.vocabulary_binding.membership_authority);

    let mut changed_binding = input;
    changed_binding.vocabulary_binding.receipt_sha256[0] ^= 1;
    assert_both_refuse(
        &changed_binding,
        PhysicalRegistryRefusalCode::PhysicalVocabularyBindingMismatch,
    );
}

#[test]
fn local_closure_does_not_require_a_false_global_vocabulary_claim() {
    let repository = repository_input().expect("repository profile is available");
    assert!(repository.admitted_artifacts.iter().any(|artifact| {
        artifact.admission_capability_kind() == AdmissionCapabilityKind::RepositoryRoot
    }));
    assert!(
        !repository
            .vocabulary_binding
            .global_physical_vocabulary_coverage
    );
    let registry = inspect_physical_registry(&repository).expect("bounded local closure");
    assert_eq!(registry.members.len(), 4);
    assert_eq!(registry.authority_effect, AuthorityEffect::None);
}

#[test]
fn root_admission_census_preserves_unfamiliar_irreducible_routes() {
    let mut fixture = elementary_fixture(27);
    let identity = fixture.input.admitted_artifacts[0].claimed_identity;
    fixture.input.admitted_artifacts[0].admission =
        irreducible_admission(230, "synthetic.unfamiliar-root", ProvenanceMark::Measured);
    fixture.input.admitted_artifacts[0].refresh_exact_test_capability();

    let census = root_admission_census(&fixture.input.admitted_artifacts).expect("exact census");
    let unfamiliar = census
        .iter()
        .find(|admission| admission.identity_sha256 == identity.0)
        .expect("unfamiliar admission remains represented");
    assert_eq!(unfamiliar.tier_id, "residue");
    assert_eq!(unfamiliar.provenance_tag, "[M]");
    assert_eq!(unfamiliar.route_id, "irreducible");
}

#[test]
fn data_defined_derivations_and_massless_unfamiliar_content_are_identity_blind() {
    assert_eq!(
        MASS_DIMENSION.terms(),
        [DimensionTerm {
            axis: SI_MASS_AXIS,
            exponent: 1,
        }],
        "the current mass floor uses its admitted SI mass-axis identity"
    );
    let floor_linked = build_fixture(
        3,
        FixtureDerivation::FloorLinked,
        false,
        FixtureDefects::default(),
    );
    let elementary = elementary_fixture(31);
    let massless = massless_fixture(91);
    let composite = composite_fixture();
    for fixture in [&floor_linked, &elementary, &massless, &composite] {
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
        assert!(verified.members.iter().all(|member| {
            fixture.input.admitted_artifacts.iter().any(|artifact| {
                artifact.claimed_identity == member.derivation_kind
                    && matches!(artifact.payload, ArtifactPayload::PhysicalDescriptor(_))
            })
        }));
    }
    assert!(inspect_physical_registry(&elementary.input)
        .unwrap()
        .canonical_bytes
        .windows("synthetic.thaumic-law-derivation-kind.v1".len())
        .any(|window| window == b"synthetic.thaumic-law-derivation-kind.v1"));
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
fn current_si_axis_identities_recompute_from_domain_separated_content() {
    assert_eq!(
        SI_DIMENSION_AXES,
        SI_DIMENSION_AXIS_IDENTITY_INPUTS.map(|content| DimensionAxisIdentity(sha256(content)))
    );
    assert!(
        SI_DIMENSION_AXIS_IDENTITY_INPUTS
            .iter()
            .all(|content| content.starts_with(b"civsim.dimension-axis.")
                && content.ends_with(b".v1"))
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
fn unrelated_unfamiliar_descriptor_is_extension_monotone_for_the_requested_member() {
    let mut fixture = massless_fixture(93);
    let baseline = inspect_physical_registry(&fixture.input).expect("baseline member closes");
    descriptor(
        &mut fixture.input.admitted_artifacts,
        "unrelated-thaumic-sector-descriptor",
        241,
    );
    refresh_vocabulary_binding(&mut fixture.input);

    let extended =
        inspect_physical_registry(&fixture.input).expect("unrelated descriptor does not steer");
    assert_eq!(extended.members, baseline.members);
    assert!(
        !fixture
            .input
            .vocabulary_binding
            .global_physical_vocabulary_coverage
    );
    assert!(!fixture.input.vocabulary_binding.membership_authority);
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
        ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
            coordinate: content("large-odd-denominator", 238),
            exact_value: ExactRationalWire {
                negative: false,
                numerator_be: vec![1],
                denominator_be: vec![0x80, 0, 0, 1],
            },
            dimension: DimensionVector::dimensionless(),
        })),
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
fn integer_power_bounds_follow_materialized_components_in_both_validators() {
    let fixture = elementary_fixture(15);
    let mass = fixture.scalar;
    let mut input = fixture.input;
    let dimensionless_four = push_artifact(
        &mut input.admitted_artifacts,
        ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
            coordinate: content("dimensionless-four", 237),
            exact_value: rational(4, 1),
            dimension: DimensionVector::dimensionless(),
        })),
        237,
    );
    replace_elementary_projection(
        &mut input,
        ExactExpression {
            nodes: vec![
                ExactExpressionNode::Coordinate(mass),
                ExactExpressionNode::Coordinate(dimensionless_four),
                ExactExpressionNode::IntegerPower {
                    base: 1,
                    exponent: 10,
                },
                ExactExpressionNode::Multiply { left: 0, right: 2 },
            ],
            output_node: 3,
        },
    );
    let caps = ValidationCaps {
        intermediate_component_bits: 25,
        ..ValidationCaps::PRODUCTION
    };
    let produced = producer::validate_and_encode_with_caps(&input, caps)
        .expect("left-to-right producer accepts the exact in-bound power");
    let watched = watchdog::validate_and_encode_with_caps(&input, caps)
        .expect("right-to-left watchdog accepts the exact in-bound power");
    assert_eq!(produced, watched);
}

#[test]
fn production_resource_contracts_are_independently_pinned() {
    assert_eq!(
        producer::resource_contract_sha256(),
        watchdog::resource_contract_sha256()
    );
    assert_ne!(producer::resource_contract_sha256(), [0; 32]);
    let mutated_profile = b"civsim.physical-species.semantic-work-profile.mutant";
    let produced_mutation =
        producer::resource_contract_sha256_with_profile_for_test(mutated_profile);
    let watched_mutation =
        watchdog::resource_contract_sha256_with_profile_for_test(mutated_profile);
    assert_eq!(produced_mutation, watched_mutation);
    assert_ne!(produced_mutation, producer::resource_contract_sha256());

    let mut input = elementary_fixture(16).input;
    input.resources.max_intermediate_component_bits += 1;
    assert_both_refuse(
        &input,
        PhysicalRegistryRefusalCode::ResourceContractMismatch,
    );
}

#[test]
fn integer_power_uses_one_semantic_work_budget_in_both_validators() {
    assert_eq!(producer::power_work_units_for_test(32_767).unwrap(), 60);
    assert_eq!(
        producer::power_work_units_for_test(32_767),
        watchdog::power_work_units_for_test(32_767)
    );

    let fixture = elementary_fixture(17);
    let mass = fixture.scalar;
    let mut input = fixture.input;
    let dimensionless_one = push_artifact(
        &mut input.admitted_artifacts,
        ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
            coordinate: content("dimensionless-one", 236),
            exact_value: rational(1, 1),
            dimension: DimensionVector::dimensionless(),
        })),
        236,
    );
    replace_elementary_projection(
        &mut input,
        ExactExpression {
            nodes: vec![
                ExactExpressionNode::Coordinate(mass),
                ExactExpressionNode::Coordinate(dimensionless_one),
                ExactExpressionNode::IntegerPower {
                    base: 1,
                    exponent: 32_767,
                },
                ExactExpressionNode::Multiply { left: 0, right: 2 },
            ],
            output_node: 3,
        },
    );

    let produced = minimum_evaluation_budget(&input, true);
    let watched = minimum_evaluation_budget(&input, false);
    assert_eq!(produced, 149);
    assert_eq!(produced, watched);
    assert_both_refuse_with_caps(
        &input,
        ValidationCaps {
            evaluation_steps: produced - 1,
            ..ValidationCaps::PRODUCTION
        },
        PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded,
    );
    assert!(producer::validate_and_encode_with_caps(
        &input,
        ValidationCaps {
            evaluation_steps: produced,
            ..ValidationCaps::PRODUCTION
        }
    )
    .is_ok());
    assert!(watchdog::validate_and_encode_with_caps(
        &input,
        ValidationCaps {
            evaluation_steps: watched,
            ..ValidationCaps::PRODUCTION
        }
    )
    .is_ok());
}

#[test]
fn semantic_evaluation_budget_is_checker_neutral_across_reference_graphs() {
    for input in [
        elementary_fixture(19).input,
        massless_fixture(53).input,
        composite_fixture().input,
    ] {
        assert_eq!(
            minimum_evaluation_budget(&input, true),
            minimum_evaluation_budget(&input, false)
        );
    }

    let mut multibyte = elementary_fixture(91);
    let prior_scalar = multibyte.scalar;
    let scalar_artifact = multibyte
        .input
        .admitted_artifacts
        .iter_mut()
        .find(|artifact| artifact.claimed_identity == prior_scalar)
        .expect("scalar fixture");
    let ArtifactPayload::ScalarCoordinate(coordinate) = &mut scalar_artifact.payload else {
        panic!("scalar fixture kind")
    };
    coordinate.exact_value = rational(257, 3);
    let scalar =
        producer::derive_artifact_identity_for_test(&scalar_artifact.payload).expect("scalar hash");
    scalar_artifact.claimed_identity = scalar;
    scalar_artifact.refresh_exact_test_capability();
    let mut expression = multibyte
        .input
        .admitted_artifacts
        .iter()
        .find_map(|artifact| match &artifact.payload {
            ArtifactPayload::MassProjection(projection) => Some(projection.expression.clone()),
            _ => None,
        })
        .expect("projection fixture");
    for node in &mut expression.nodes {
        if let ExactExpressionNode::Coordinate(identity) = node {
            if *identity == prior_scalar {
                *identity = scalar;
            }
        }
    }
    replace_elementary_projection(&mut multibyte.input, expression);
    assert_eq!(
        minimum_evaluation_budget(&multibyte.input, true),
        minimum_evaluation_budget(&multibyte.input, false)
    );

    let mut reciprocal = elementary_fixture(117);
    let mass = reciprocal.scalar;
    let one = push_artifact(
        &mut reciprocal.input.admitted_artifacts,
        ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
            coordinate: content("reciprocal-one", 237),
            exact_value: rational(1, 1),
            dimension: DimensionVector::dimensionless(),
        })),
        237,
    );
    replace_elementary_projection(
        &mut reciprocal.input,
        ExactExpression {
            nodes: vec![
                ExactExpressionNode::Coordinate(mass),
                ExactExpressionNode::Coordinate(one),
                ExactExpressionNode::IntegerPower {
                    base: 1,
                    exponent: -17,
                },
                ExactExpressionNode::Multiply { left: 0, right: 2 },
            ],
            output_node: 3,
        },
    );
    assert_eq!(
        minimum_evaluation_budget(&reciprocal.input, true),
        minimum_evaluation_budget(&reciprocal.input, false)
    );
}

#[test]
fn closure_uses_one_graph_derived_budget_in_both_validators() {
    let elementary = elementary_fixture(18).input;
    let produced_elementary = minimum_closure_budget(&elementary, true);
    let watched_elementary = minimum_closure_budget(&elementary, false);
    assert_eq!(produced_elementary, 3);
    assert_eq!(produced_elementary, watched_elementary);

    let mut composite = composite_fixture().input;
    let produced_composite = minimum_closure_budget(&composite, true);
    let watched_composite = minimum_closure_budget(&composite, false);
    assert_eq!(produced_composite, 13);
    assert_eq!(produced_composite, watched_composite);

    composite.admitted_artifacts.reverse();
    assert_eq!(minimum_closure_budget(&composite, true), produced_composite);
    assert_eq!(minimum_closure_budget(&composite, false), watched_composite);
    assert_both_refuse_with_caps(
        &composite,
        ValidationCaps {
            closure_steps: produced_composite - 1,
            ..ValidationCaps::PRODUCTION
        },
        PhysicalRegistryRefusalCode::ClosureStepLimitExceeded,
    );
}

#[test]
fn ambiguous_over_cap_products_refuse_before_multiplication() {
    let one = BigUint::from_u64(1);
    let left = one.shl_bits(40).sub(&one);
    let right = one.shl_bits(25).sub(&one);
    assert_eq!(left.bit_len() + right.bit_len(), 65);
    assert_eq!(left.mul(&right).bit_len(), 65);
    assert!(!producer::product_fits_component_cap_for_test(
        &left, &right, 64,
    ));
    assert!(!watchdog::product_fits_component_cap_for_test(
        &left, &right, 64,
    ));

    let fitting_left = one.shl_bits(39);
    let fitting_right = one.shl_bits(24);
    assert_eq!(fitting_left.bit_len() + fitting_right.bit_len(), 65);
    assert_eq!(fitting_left.mul(&fitting_right).bit_len(), 64);
    assert!(producer::product_fits_component_cap_for_test(
        &fitting_left,
        &fitting_right,
        64,
    ));
    assert!(watchdog::product_fits_component_cap_for_test(
        &fitting_left,
        &fitting_right,
        64,
    ));
}

#[test]
fn variable_cardinality_dimension_basis_accepts_and_cancels_unfamiliar_axes() {
    let fixture = elementary_fixture(14);
    let mut input = fixture.input;
    let unfamiliar_dimension = DimensionVector::from_terms(
        (1_u8..=8)
            .map(|tag| DimensionTerm {
                axis: DimensionAxisIdentity([tag; 32]),
                exponent: i16::from(tag),
            })
            .collect(),
    )
    .expect("eight unfamiliar axes fit the admitted resource contract");
    assert_eq!(unfamiliar_dimension.terms().len(), 8);
    let unfamiliar = push_artifact(
        &mut input.admitted_artifacts,
        ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
            coordinate: content("thaumic-coordinate-system", 239),
            exact_value: rational(11, 5),
            dimension: unfamiliar_dimension,
        })),
        239,
    );
    replace_elementary_projection(
        &mut input,
        ExactExpression {
            nodes: vec![
                ExactExpressionNode::Coordinate(fixture.scalar),
                ExactExpressionNode::Coordinate(unfamiliar),
                ExactExpressionNode::Divide {
                    numerator: 1,
                    denominator: 1,
                },
                ExactExpressionNode::Multiply { left: 0, right: 2 },
            ],
            output_node: 3,
        },
    );
    let produced = producer::validate_and_encode(&input).expect("producer accepts cancellation");
    let watched = watchdog::validate_and_encode(&input).expect("watchdog accepts cancellation");
    assert_eq!(produced, watched);
    assert_eq!(produced.members[0].mass_dimension, MASS_DIMENSION);

    let too_many = (1_u8..=65)
        .map(|tag| DimensionTerm {
            axis: DimensionAxisIdentity([tag; 32]),
            exponent: 1,
        })
        .collect();
    assert_eq!(
        DimensionVector::from_terms(too_many),
        Err(PhysicalRegistryRefusalCode::DimensionTermCapacityExceeded)
    );
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
    scalar.refresh_exact_test_capability();
    name_only.admitted_artifacts = vec![scalar];
    assert_both_refuse(
        &name_only,
        PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRules,
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
    admitted_citation.admitted_artifacts[0].refresh_exact_test_capability();
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
    duplicate_slot.admitted_artifacts[0].refresh_exact_test_capability();
    duplicate_slot.admitted_artifacts[1].refresh_exact_test_capability();
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
fn receipt_shaped_values_cannot_mint_an_admission_capability() {
    let mut invented_derived = elementary_fixture(119).input;
    let AdmissionRoute::Derived(route) =
        &mut invented_derived.admitted_artifacts[0].admission.route
    else {
        panic!("derived fixture admission")
    };
    route.ancestry_receipt = receipt("invented-ancestry", 241);
    assert_both_refuse(
        &invented_derived,
        PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch,
    );

    let mut invented_irreducible = elementary_fixture(120).input;
    invented_irreducible.admitted_artifacts[0].admission = irreducible_admission(
        242,
        "synthetic.invented-residual-slot",
        ProvenanceMark::Estimator,
    );
    assert_both_refuse(
        &invented_irreducible,
        PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch,
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
    second.payload = ArtifactPayload::PhysicalDescriptor(content("collision-payload", 250));
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
        .position(|artifact| {
            matches!(
                &artifact.payload,
                ArtifactPayload::SpeciesDerivation(rule) if !rule.constituents.is_empty()
            )
        })
        .expect("composite rule fixture");
    let prior_constraints = match &dangling.input.admitted_artifacts[composite_index].payload {
        ArtifactPayload::SpeciesDerivation(rule) => rule.output.constraint_laws.clone(),
        _ => unreachable!("matched composite rule"),
    };
    let mut replacement_laws = std::collections::BTreeMap::new();
    for relation in &prior_constraints {
        if replacement_laws.contains_key(&relation.target) {
            continue;
        }
        let law_index = dangling
            .input
            .admitted_artifacts
            .iter()
            .position(|artifact| artifact.claimed_identity == relation.target)
            .expect("composite law fixture");
        match &mut dangling.input.admitted_artifacts[law_index].payload {
            ArtifactPayload::ConstraintLaw(law) => {
                law.requirements.species_dependencies = vec![unknown];
            }
            _ => panic!("composite law kind"),
        }
        let identity = producer::derive_artifact_identity_for_test(
            &dangling.input.admitted_artifacts[law_index].payload,
        )
        .expect("replacement law hashes");
        dangling.input.admitted_artifacts[law_index].claimed_identity = identity;
        dangling.input.admitted_artifacts[law_index].refresh_exact_test_capability();
        replacement_laws.insert(relation.target, identity);
    }
    let composite_rule = &mut dangling.input.admitted_artifacts[composite_index];
    let ArtifactPayload::SpeciesDerivation(rule) = &mut composite_rule.payload else {
        unreachable!("matched composite rule")
    };
    rule.constituents = vec![unknown];
    rule.output.requirements.species_dependencies = vec![unknown];
    for relation in &mut rule.output.constraint_laws {
        relation.target = replacement_laws[&relation.target];
    }
    composite_rule.claimed_identity =
        producer::derive_artifact_identity_for_test(&composite_rule.payload)
            .expect("dangling rule hashes");
    composite_rule.refresh_exact_test_capability();
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
fn an_unrequested_species_rule_island_cannot_enter_the_local_cone() {
    let mut requested = massless_fixture(55);
    let unrequested = elementary_fixture(155);
    requested
        .input
        .admitted_artifacts
        .extend(unrequested.input.admitted_artifacts);
    refresh_vocabulary_binding(&mut requested.input);

    assert_both_refuse(
        &requested.input,
        PhysicalRegistryRefusalCode::MissingClosureMember,
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
        let fixture = build_fixture(37, FixtureDerivation::Unfamiliar, false, defects);
        assert_both_refuse(&fixture.input, expected);
    }
}

#[test]
fn exact_zero_needs_a_massless_law_and_expression_cycles_refuse() {
    let zero = build_fixture(
        41,
        FixtureDerivation::Unfamiliar,
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
        scope: MassProjectionScope::SpeciesRestMass,
        uncertainty_transport: Some(MassUncertaintyTransportProof {
            sources: vec![MassUncertaintySourceProof {
                source_coordinate: ArtifactIdentity([1; 32]),
                source_pair_receipt: receipt("cycle-mass-source-pair", 221),
            }],
            producer_receipt: receipt("cycle-mass-uncertainty-producer", 222),
            watchdog_receipt: receipt("cycle-mass-uncertainty-watchdog", 223),
        }),
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
        scope: MassProjectionScope::SpeciesRestMass,
        uncertainty_transport: Some(MassUncertaintyTransportProof {
            sources: vec![MassUncertaintySourceProof {
                source_coordinate: ArtifactIdentity([1; 32]),
                source_pair_receipt: receipt("deep-mass-source-pair", 224),
            }],
            producer_receipt: receipt("deep-mass-uncertainty-producer", 225),
            watchdog_receipt: receipt("deep-mass-uncertainty-watchdog", 226),
        }),
    });
    assert_both_refuse(
        &too_deep,
        PhysicalRegistryRefusalCode::ExpressionDepthExceeded,
    );
}

#[test]
fn exact_zero_requires_scoped_content_and_independent_receipts() {
    for defects in [
        FixtureDefects {
            massless_subject_outside_requirements: true,
            ..FixtureDefects::default()
        },
        FixtureDefects {
            massless_symmetry_outside_requirements: true,
            ..FixtureDefects::default()
        },
        FixtureDefects {
            massless_subject_equals_symmetry: true,
            ..FixtureDefects::default()
        },
        FixtureDefects {
            massless_duplicate_receipts: true,
            ..FixtureDefects::default()
        },
    ] {
        let fixture = build_fixture(45, FixtureDerivation::Unfamiliar, true, defects);
        assert_both_refuse(
            &fixture.input,
            PhysicalRegistryRefusalCode::UnprovedExactZero,
        );
    }

    let fixture = massless_fixture(46);
    let mut invalid_payload = fixture
        .input
        .admitted_artifacts
        .iter()
        .find_map(|artifact| match &artifact.payload {
            ArtifactPayload::ExactMasslessLaw(law) => {
                Some(ArtifactPayload::ExactMasslessLaw(law.clone()))
            }
            _ => None,
        })
        .expect("massless fixture carries its exact-zero law");
    let ArtifactPayload::ExactMasslessLaw(law) = &mut invalid_payload else {
        unreachable!("selected an exact-zero law")
    };
    law.proof.excluded_term.canonical_bytes.clear();
    assert_eq!(
        producer::derive_artifact_identity_for_test(&invalid_payload),
        Err(PhysicalRegistryRefusalCode::ContentByteLimitExceeded)
    );
    assert_eq!(
        watchdog::derive_artifact_identity_for_authority(&invalid_payload),
        Err(PhysicalRegistryRefusalCode::ContentByteLimitExceeded)
    );
}

#[test]
fn a_membership_neutral_mass_projection_cannot_prove_a_species_mass() {
    let fixture = elementary_fixture(47);
    let mut input = fixture.input;
    let expression = input
        .admitted_artifacts
        .iter()
        .find_map(|artifact| match &artifact.payload {
            ArtifactPayload::MassProjection(projection) => Some(projection.expression.clone()),
            _ => None,
        })
        .expect("projection fixture");
    replace_elementary_projection_with_scope(
        &mut input,
        expression,
        MassProjectionScope::MembershipNeutral,
    );
    assert_both_refuse(
        &input,
        PhysicalRegistryRefusalCode::MassProjectionNotAuthorizedForMember,
    );
}

#[test]
fn species_mass_uncertainty_transport_needs_independent_receipts() {
    let fixture = elementary_fixture(48);
    let source_coordinate = fixture.scalar;
    let mut input = fixture.input;
    let expression = input
        .admitted_artifacts
        .iter()
        .find_map(|artifact| match &artifact.payload {
            ArtifactPayload::MassProjection(projection) => Some(projection.expression.clone()),
            _ => None,
        })
        .expect("projection fixture");
    let duplicate = receipt("duplicate-mass-uncertainty-checker", 241);
    replace_elementary_projection_with_transport(
        &mut input,
        expression,
        MassProjectionScope::SpeciesRestMass,
        Some(MassUncertaintyTransportProof {
            sources: vec![MassUncertaintySourceProof {
                source_coordinate,
                source_pair_receipt: receipt("independent-mass-source-pair", 240),
            }],
            producer_receipt: duplicate.clone(),
            watchdog_receipt: duplicate,
        }),
    );
    assert_both_refuse(
        &input,
        PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid,
    );
}

#[test]
fn multi_source_mass_uncertainty_transport_is_complete_unique_and_canonical() {
    let fixture = elementary_fixture(49);
    let first_source = fixture.scalar;
    let mut base = fixture.input;
    let second_source = push_artifact(
        &mut base.admitted_artifacts,
        ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
            coordinate: content("second-mass-coordinate", 242),
            exact_value: rational(11, 5),
            dimension: MASS_DIMENSION,
        })),
        242,
    );
    let expression = ExactExpression {
        nodes: vec![
            ExactExpressionNode::Coordinate(first_source),
            ExactExpressionNode::Coordinate(second_source),
            ExactExpressionNode::Add { left: 0, right: 1 },
        ],
        output_node: 2,
    };
    let mut sources = vec![
        MassUncertaintySourceProof {
            source_coordinate: first_source,
            source_pair_receipt: receipt("first-mass-source-pair", 243),
        },
        MassUncertaintySourceProof {
            source_coordinate: second_source,
            source_pair_receipt: receipt("second-mass-source-pair", 244),
        },
    ];
    sources.sort_by_key(|source| source.source_coordinate);
    let transport = MassUncertaintyTransportProof {
        sources,
        producer_receipt: receipt("two-source-mass-uncertainty-producer", 245),
        watchdog_receipt: receipt("two-source-mass-uncertainty-watchdog", 246),
    };

    let mut valid = base.clone();
    replace_elementary_projection_with_transport(
        &mut valid,
        expression.clone(),
        MassProjectionScope::SpeciesRestMass,
        Some(transport.clone()),
    );
    refresh_vocabulary_binding(&mut valid);
    assert!(producer::validate_and_encode(&valid).is_ok());
    assert!(watchdog::validate_and_encode(&valid).is_ok());

    let mut missing = base.clone();
    let mut missing_transport = transport.clone();
    missing_transport.sources.pop();
    replace_elementary_projection_with_transport(
        &mut missing,
        expression.clone(),
        MassProjectionScope::SpeciesRestMass,
        Some(missing_transport),
    );
    assert_both_refuse(
        &missing,
        PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid,
    );

    let mut duplicate = base.clone();
    let mut duplicate_transport = transport.clone();
    duplicate_transport.sources[1] = duplicate_transport.sources[0].clone();
    replace_elementary_projection_with_transport(
        &mut duplicate,
        expression.clone(),
        MassProjectionScope::SpeciesRestMass,
        Some(duplicate_transport),
    );
    assert_both_refuse(
        &duplicate,
        PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid,
    );

    let mut reordered = base;
    let mut reordered_transport = transport;
    reordered_transport.sources.reverse();
    replace_elementary_projection_with_transport(
        &mut reordered,
        expression,
        MassProjectionScope::SpeciesRestMass,
        Some(reordered_transport),
    );
    assert_both_refuse(
        &reordered,
        PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid,
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
                dimension_term_count: 0,
                ..ValidationCaps::PRODUCTION
            },
            PhysicalRegistryRefusalCode::DimensionTermCapacityExceeded,
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
