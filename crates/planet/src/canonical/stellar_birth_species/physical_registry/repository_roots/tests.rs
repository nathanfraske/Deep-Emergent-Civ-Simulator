use super::super::model::{
    dimension_from_si_exponents, AdmissionRoute, ArtifactPayload, DimensionVector,
    ExactExpressionNode, LedgerTier, MassProjectionScope, ProvenanceMark, ReceiptBinding,
    MASS_DIMENSION,
};
use super::*;
use std::collections::BTreeSet;

fn si_dimension(exponents: [i16; 7]) -> DimensionVector {
    dimension_from_si_exponents(exponents).expect("synthetic SI dimension")
}

fn binding(schema: &str, tag: u8) -> ReceiptBinding {
    ReceiptBinding {
        schema_id: schema.to_owned(),
        digest_sha256: [tag; 32],
    }
}

fn source(
    symbol: &str,
    central_decimal: &str,
    uncertainty_decimal: &str,
    dimension: DimensionVector,
    tag: u8,
) -> RepositoryRootSource {
    RepositoryRootSource::new(
        format!("fundamental.{symbol}"),
        symbol,
        central_decimal,
        "standard",
        uncertainty_decimal,
        dimension,
        LedgerTier::Universal,
        ProvenanceMark::Measured,
        binding("civsim.test.floor-exhaustion.v1", tag),
    )
}

fn repository_packet() -> RepositoryRootPacket {
    RepositoryRootPacket::new(
        binding("civsim.test.floor-authority.v1", 1),
        vec![
            source(
                "alpha",
                "7.2973525693e-3",
                "1.1e-12",
                si_dimension([0, 0, 0, 0, 0, 0, 0]),
                11,
            ),
            source(
                "G",
                "6.67430e-11",
                "1.5e-15",
                si_dimension([3, -1, -2, 0, 0, 0, 0]),
                12,
            ),
            source("m_e", "9.1093837015e-31", "2.8e-40", MASS_DIMENSION, 13),
        ],
    )
}

#[test]
fn production_pair_independently_extracts_and_receipts_the_sealed_floor() {
    let producer_packet =
        producer::sealed_floor_packet().expect("producer extracts the sealed floor");
    let watchdog_packet =
        watchdog::sealed_floor_packet().expect("watchdog extracts the sealed floor");
    assert_eq!(producer_packet, watchdog_packet);
    assert_eq!(
        producer_packet
            .sources
            .iter()
            .map(|source| source.entry_id.as_str())
            .collect::<Vec<_>>(),
        ["fundamental.alpha", "fundamental.G", "fundamental.m_e"]
    );
    assert_eq!(
        producer::resource_contract_sha256().expect("producer resource contract"),
        watchdog::resource_contract_sha256().expect("watchdog resource contract")
    );

    let projection = project_repository_roots().expect("the sealed pair agrees");
    assert!(verify_projection(&projection));
    assert!(producer::verify_projection(&projection));
    assert!(watchdog::verify_projection(&projection));
    assert!(verify_projection_receipt(&projection.receipt));
    assert_eq!(
        projection.receipt.claim_id,
        "planet.stellar-species-floor-coordinate-projection"
    );
    assert_eq!(projection.receipt.decision_id, "agreed_projected");
    assert_eq!(
        projection.receipt.producer_result_sha256,
        projection.receipt.watchdog_result_sha256
    );
    assert_eq!(
        projection.receipt.result_sha256,
        projection.receipt.producer_result_sha256
    );
    assert_eq!(
        projection.receipt.canary_suite_id,
        REPOSITORY_ROOT_CANARY_SUITE_ID
    );
    assert_ne!(projection.receipt.canary_sha256, [0; 32]);
    assert_ne!(projection.post_projection_canary_sha256, [0; 32]);
    assert_eq!(
        projection.post_projection_canary_sha256,
        producer::post_projection_canary_digest(&projection)
            .expect("producer binds exact post-projection mutations")
    );
    assert_eq!(
        projection.post_projection_canary_sha256,
        watchdog::post_projection_canary_digest(&projection)
            .expect("watchdog binds exact post-projection mutations")
    );
    assert_ne!(projection.receipt.receipt_sha256, [0; 32]);
}

#[test]
fn paired_projection_verifier_rejects_post_projection_mutations() {
    let baseline = project_repository_roots().expect("the sealed pair agrees");
    let mut mutations = Vec::new();

    let mut omitted = baseline.clone();
    omitted.admitted_artifacts.pop();
    mutations.push(omitted);

    let mut duplicated = baseline.clone();
    duplicated
        .admitted_artifacts
        .push(baseline.admitted_artifacts[0].clone());
    mutations.push(duplicated);

    let mut scope = baseline.clone();
    let mass = scope
        .admitted_artifacts
        .iter_mut()
        .find_map(|artifact| match &mut artifact.payload {
            ArtifactPayload::MassProjection(projection) => Some(projection),
            _ => None,
        })
        .expect("the sealed floor has one pure-mass projection");
    mass.scope = MassProjectionScope::SpeciesRestMass;
    mutations.push(scope);

    let mut canonical_bytes = baseline.clone();
    canonical_bytes.canonical_bytes[0] ^= 1;
    mutations.push(canonical_bytes);

    let mut admission_receipt = baseline.clone();
    let AdmissionRoute::Derived(admission) =
        &mut admission_receipt.admitted_artifacts[0].admission.route
    else {
        panic!("root admission is derived");
    };
    admission.semantic_checker_receipt.digest_sha256[0] ^= 1;
    mutations.push(admission_receipt);

    let mut post_projection_canary = baseline.clone();
    post_projection_canary.post_projection_canary_sha256[0] ^= 1;
    mutations.push(post_projection_canary);

    for mutation in mutations {
        assert!(!producer::verify_projection(&mutation));
        assert!(!watchdog::verify_projection(&mutation));
        assert!(!verify_projection(&mutation));
    }
}

#[test]
fn same_code_packet_mutations_have_distinct_exact_preimages() {
    let baseline = producer::sealed_floor_packet().expect("producer extracts the sealed floor");
    let mut first = baseline.clone();
    first.schema_id.push_str(".first-substitution");
    let mut second = baseline;
    second.schema_id.push_str(".second-substitution");

    for mutation in [&first, &second] {
        assert_eq!(
            producer::project(mutation),
            Err(RepositoryRootRefusalCode::SchemaMismatch)
        );
        assert_eq!(
            watchdog::project(mutation),
            Err(RepositoryRootRefusalCode::SchemaMismatch)
        );
    }

    let producer_first = producer::canary_packet_preimage_sha256_for_test(&first)
        .expect("producer encodes first packet mutation");
    let watchdog_first = watchdog::canary_packet_preimage_sha256_for_test(&first)
        .expect("watchdog encodes first packet mutation");
    let producer_second = producer::canary_packet_preimage_sha256_for_test(&second)
        .expect("producer encodes second packet mutation");
    let watchdog_second = watchdog::canary_packet_preimage_sha256_for_test(&second)
        .expect("watchdog encodes second packet mutation");

    assert_eq!(producer_first, watchdog_first);
    assert_eq!(producer_second, watchdog_second);
    assert_ne!(producer_first, producer_second);
    assert_ne!(producer_first, watchdog_second);

    let producer_first_transcript =
        producer::canary_packet_observation_sha256_for_test("same_code", &first)
            .expect("producer records first packet outcome");
    let watchdog_first_transcript =
        watchdog::canary_packet_observation_sha256_for_test("same_code", &first)
            .expect("watchdog records first packet outcome");
    let producer_second_transcript =
        producer::canary_packet_observation_sha256_for_test("same_code", &second)
            .expect("producer records second packet outcome");
    let watchdog_second_transcript =
        watchdog::canary_packet_observation_sha256_for_test("same_code", &second)
            .expect("watchdog records second packet outcome");
    assert_eq!(producer_first_transcript, watchdog_first_transcript);
    assert_eq!(producer_second_transcript, watchdog_second_transcript);
    assert_ne!(producer_first_transcript, producer_second_transcript);
    assert_ne!(producer_first_transcript, watchdog_second_transcript);
}

#[test]
fn same_code_projection_mutations_cannot_silently_agree() {
    let baseline = project_repository_roots().expect("the sealed pair agrees");
    let mut omitted = baseline.clone();
    omitted.admitted_artifacts.pop();
    let mut duplicated = baseline.clone();
    duplicated
        .admitted_artifacts
        .push(baseline.admitted_artifacts[0].clone());

    for mutation in [&omitted, &duplicated] {
        assert_eq!(
            producer::inspect_projection_core_for_test(mutation),
            Err(RepositoryRootRefusalCode::ReceiptInvalid)
        );
        assert_eq!(
            watchdog::inspect_projection_core_for_test(mutation),
            Err(RepositoryRootRefusalCode::ReceiptInvalid)
        );
    }

    let producer_omitted = producer::canary_projection_preimage_sha256_for_test(&omitted)
        .expect("producer encodes omitted-artifact projection");
    let watchdog_omitted = watchdog::canary_projection_preimage_sha256_for_test(&omitted)
        .expect("watchdog encodes omitted-artifact projection");
    let producer_duplicated = producer::canary_projection_preimage_sha256_for_test(&duplicated)
        .expect("producer encodes duplicated-artifact projection");
    let watchdog_duplicated = watchdog::canary_projection_preimage_sha256_for_test(&duplicated)
        .expect("watchdog encodes duplicated-artifact projection");

    assert_eq!(producer_omitted, watchdog_omitted);
    assert_eq!(producer_duplicated, watchdog_duplicated);
    assert_ne!(producer_omitted, producer_duplicated);
    assert_ne!(producer_omitted, watchdog_duplicated);

    let producer_omitted_transcript =
        producer::canary_projection_observation_sha256_for_test("same_code", &omitted)
            .expect("producer records omitted-artifact outcome");
    let watchdog_omitted_transcript =
        watchdog::canary_projection_observation_sha256_for_test("same_code", &omitted)
            .expect("watchdog records omitted-artifact outcome");
    let producer_duplicated_transcript =
        producer::canary_projection_observation_sha256_for_test("same_code", &duplicated)
            .expect("producer records duplicated-artifact outcome");
    let watchdog_duplicated_transcript =
        watchdog::canary_projection_observation_sha256_for_test("same_code", &duplicated)
            .expect("watchdog records duplicated-artifact outcome");
    assert_eq!(producer_omitted_transcript, watchdog_omitted_transcript);
    assert_eq!(
        producer_duplicated_transcript,
        watchdog_duplicated_transcript
    );
    assert_ne!(producer_omitted_transcript, producer_duplicated_transcript);
    assert_ne!(producer_omitted_transcript, watchdog_duplicated_transcript);
}

fn rebind_projection_receipt(projection: &mut RepositoryRootProjection) {
    projection.receipt.producer_ancestry_manifest_sha256 =
        producer::ancestry_manifest_sha256(&projection.checker_candidates)
            .expect("producer ancestry manifest");
    projection.receipt.watchdog_ancestry_manifest_sha256 =
        watchdog::ancestry_manifest_sha256(&projection.checker_candidates)
            .expect("watchdog ancestry manifest");
    projection.receipt.receipt_sha256 = pair_receipt_digest(&projection.receipt);
    projection.admitted_artifacts = projection
        .checker_candidates
        .iter()
        .map(|candidate| admit_candidate(candidate, &projection.receipt))
        .collect();
}

#[test]
fn coordinated_ancestry_mutation_cannot_retain_projection_authority() {
    let baseline = project_repository_roots().expect("the sealed pair agrees");
    let mut mutation = baseline.clone();
    mutation.checker_candidates[0].ancestry_digest_sha256[0] ^= 1;
    mutation.admitted_artifacts = mutation
        .checker_candidates
        .iter()
        .map(|candidate| admit_candidate(candidate, &mutation.receipt))
        .collect();

    assert_eq!(mutation.canonical_bytes, baseline.canonical_bytes);
    assert!(verify_projection_receipt(&mutation.receipt));
    assert!(!producer::verify_projection(&mutation));
    assert!(!watchdog::verify_projection(&mutation));
    assert!(!verify_projection(&mutation));

    rebind_projection_receipt(&mut mutation);
    assert!(verify_projection_receipt(&mutation.receipt));
    assert!(!producer::verify_projection(&mutation));
    assert!(!watchdog::verify_projection(&mutation));
    assert!(!verify_projection(&mutation));
}

#[test]
fn final_verifiers_reconnect_a_self_consistent_projection_to_the_sealed_floor() {
    let baseline = project_repository_roots().expect("the sealed pair agrees");
    let mut unsealed_packet =
        producer::sealed_floor_packet().expect("producer extracts the sealed floor");
    unsealed_packet.sources[0].central_decimal = "8.2973525693e-3".to_owned();

    let produced =
        producer::project_unsealed(&unsealed_packet).expect("generic producer projection");
    let watched =
        watchdog::project_unsealed(&unsealed_packet).expect("generic watchdog projection");
    assert_eq!(produced, watched);

    let result_sha256 = sha256(&produced.canonical_bytes);
    let mut projection = RepositoryRootProjection {
        admitted_artifacts: Vec::new(),
        canonical_bytes: produced.canonical_bytes,
        receipt: RepositoryRootProjectionReceipt {
            input_sha256: produced.input_sha256,
            result_sha256,
            producer_result_sha256: result_sha256,
            watchdog_result_sha256: result_sha256,
            scalar_coordinate_count: produced.scalar_coordinate_count,
            mass_projection_count: produced.mass_projection_count,
            receipt_sha256: [0; 32],
            ..baseline.receipt
        },
        membership_authority: false,
        checker_candidates: produced.candidates,
        post_projection_canary_sha256: [0; 32],
    };
    rebind_projection_receipt(&mut projection);

    assert!(verify_projection_receipt(&projection.receipt));
    assert!(!producer::verify_projection(&projection));
    assert!(!watchdog::verify_projection(&projection));
    assert!(!verify_projection(&projection));
}

#[test]
fn final_verifiers_reconnect_resource_and_canary_attestations() {
    let baseline = project_repository_roots().expect("the sealed pair agrees");
    let mut mutations = Vec::new();

    let mut resource = baseline.clone();
    resource.receipt.producer_resource_contract_sha256 = [41; 32];
    resource.receipt.watchdog_resource_contract_sha256 = [41; 32];
    rebind_projection_receipt(&mut resource);
    mutations.push(resource);

    let mut canary = baseline;
    canary.receipt.canary_sha256 = [42; 32];
    rebind_projection_receipt(&mut canary);
    mutations.push(canary);

    for mutation in mutations {
        assert!(verify_projection_receipt(&mutation.receipt));
        assert!(!producer::verify_projection(&mutation));
        assert!(!watchdog::verify_projection(&mutation));
        assert!(!verify_projection(&mutation));
    }
}

#[test]
fn every_pair_receipt_field_is_bound_against_substitution() {
    let receipt = project_repository_roots()
        .expect("the sealed pair agrees")
        .receipt;
    let mut mutations = Vec::new();

    let mut changed = receipt.clone();
    changed.schema_id = "civsim.planet.substituted-root-receipt.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.claim_id = "planet.substituted-claim";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.input_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.result_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.producer_result_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.watchdog_result_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.producer_ancestry_manifest_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.watchdog_ancestry_manifest_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.producer_resource_contract_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.watchdog_resource_contract_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.canary_suite_id = "civsim.planet.substituted-root-canaries.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.canary_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.producer_id = "civsim.planet.substituted-producer.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.watchdog_id = "civsim.planet.substituted-watchdog.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.scalar_coordinate_count += 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.mass_projection_count += 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.membership_authority = true;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.decision_id = "substituted";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.receipt_sha256[0] ^= 1;
    mutations.push(changed);

    assert!(mutations
        .iter()
        .all(|mutation| !verify_projection_receipt(mutation)));
}

fn schema_refusal() -> RepositoryRootRefusal {
    let mut packet = repository_packet();
    packet.schema_id = "civsim.test.invalid-root-packet.v1".to_owned();
    refusal_for_invalid_packet_for_test(&packet)
}

#[test]
fn refused_pair_emits_bound_evidence_without_a_root_capability() {
    let mut packet = repository_packet();
    packet.schema_id = "civsim.test.invalid-root-packet.v1".to_owned();
    let packet_sha256 = packet_observation_sha256(&packet);

    let refusal = refusal_for_invalid_packet_for_test(&packet);
    let receipt = &refusal.receipt;

    assert_eq!(refusal.code, RepositoryRootRefusalCode::SchemaMismatch);
    assert_eq!(receipt.refusal_code, refusal.code);
    assert!(verify_refusal(&refusal));
    assert!(verify_refusal_receipt(receipt));
    assert_eq!(receipt.schema_id, REPOSITORY_ROOT_REFUSAL_RECEIPT_SCHEMA_ID);
    assert_eq!(receipt.claim_id, REPOSITORY_ROOT_CLAIM_ID);
    assert_eq!(receipt.producer_id, REPOSITORY_ROOT_PRODUCER_ID);
    assert_eq!(receipt.watchdog_id, REPOSITORY_ROOT_WATCHDOG_ID);
    assert_eq!(receipt.producer_packet_sha256, packet_sha256);
    assert_eq!(receipt.watchdog_packet_sha256, packet_sha256);
    assert!(matches!(
        receipt.producer_outcome,
        RepositoryRootCheckerOutcome::Refused {
            code: RepositoryRootRefusalCode::SchemaMismatch
        }
    ));
    assert!(matches!(
        receipt.watchdog_outcome,
        RepositoryRootCheckerOutcome::Refused {
            code: RepositoryRootRefusalCode::SchemaMismatch
        }
    ));
    let (
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: producer_resource_sha256,
        },
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: watchdog_resource_sha256,
        },
    ) = (
        &receipt.producer_resource_contract,
        &receipt.watchdog_resource_contract,
    )
    else {
        panic!("both root checkers must bind their resource contracts")
    };
    assert_ne!(*producer_resource_sha256, [0; 32]);
    assert_eq!(producer_resource_sha256, watchdog_resource_sha256);
    assert_eq!(receipt.canary.suite_id, REPOSITORY_ROOT_CANARY_SUITE_ID);
    assert_eq!(receipt.canary.digest_sha256, None);
    assert_eq!(
        receipt.refusal_stage,
        RepositoryRootRefusalStage::CheckerAgreement
    );
    assert_eq!(receipt.decision_id, "refused");
    assert!(!receipt.membership_authority);
    assert_ne!(receipt.receipt_sha256, [0; 32]);
    assert_eq!(
        inspect_unsealed_packet_for_test(&packet),
        Err(RepositoryRootRefusalCode::SchemaMismatch)
    );

    let mut substituted_outer_decision = refusal;
    substituted_outer_decision.code = RepositoryRootRefusalCode::CheckerDisagreement;
    assert!(!verify_refusal(&substituted_outer_decision));
}

#[test]
fn structurally_consistent_synthetic_refusal_cannot_verify_as_current_production_evidence() {
    let refusal = schema_refusal();
    assert!(verify_refusal(&refusal));
    assert!(verify_refusal_receipt(&refusal.receipt));
    assert!(!verify_current_refusal(&refusal));
    assert!(matches!(
        decide_repository_roots(),
        RepositoryRootDecision::Projected(_)
    ));
}

#[test]
fn every_refusal_receipt_field_is_bound_against_substitution() {
    let receipt = schema_refusal().receipt;
    let mut mutations = Vec::new();

    let mut changed = receipt.clone();
    changed.schema_id = "civsim.planet.substituted-root-refusal-receipt.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.claim_id = "planet.substituted-root-claim";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.producer_id = "civsim.planet.substituted-root-producer.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.watchdog_id = "civsim.planet.substituted-root-watchdog.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.producer_packet_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.watchdog_packet_sha256[0] ^= 1;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.producer_outcome = RepositoryRootCheckerOutcome::Refused {
        code: RepositoryRootRefusalCode::EmptySourceSet,
    };
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.watchdog_outcome = RepositoryRootCheckerOutcome::Refused {
        code: RepositoryRootRefusalCode::EmptySourceSet,
    };
    mutations.push(changed);
    let mut changed = receipt.clone();
    if let RepositoryRootResourceContractOutcome::Bound { digest_sha256 } =
        &mut changed.producer_resource_contract
    {
        digest_sha256[0] ^= 1;
    } else {
        panic!("test refusal must carry a producer resource digest");
    }
    mutations.push(changed);
    let mut changed = receipt.clone();
    if let RepositoryRootResourceContractOutcome::Bound { digest_sha256 } =
        &mut changed.watchdog_resource_contract
    {
        digest_sha256[0] ^= 1;
    } else {
        panic!("test refusal must carry a watchdog resource digest");
    }
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.canary.suite_id = "civsim.planet.substituted-root-canaries.v1";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.canary.digest_sha256 = Some([19; 32]);
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.refusal_stage = RepositoryRootRefusalStage::Canary;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.refusal_code = RepositoryRootRefusalCode::EmptySourceSet;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.decision_id = "projected";
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.membership_authority = true;
    mutations.push(changed);
    let mut changed = receipt.clone();
    changed.receipt_sha256[0] ^= 1;
    mutations.push(changed);

    assert!(mutations
        .iter()
        .all(|mutation| !verify_refusal_receipt(mutation)));
}

#[test]
fn redigesting_a_substituted_refusal_authority_does_not_make_it_valid() {
    let receipt = schema_refusal().receipt;

    let mut substituted_claim = receipt.clone();
    substituted_claim.claim_id = "planet.substituted-root-claim";
    substituted_claim.receipt_sha256 = refusal_receipt_digest(&substituted_claim);
    assert!(!verify_refusal_receipt(&substituted_claim));

    let mut substituted_checker = receipt.clone();
    substituted_checker.producer_id = "civsim.planet.substituted-root-producer.v1";
    substituted_checker.receipt_sha256 = refusal_receipt_digest(&substituted_checker);
    assert!(!verify_refusal_receipt(&substituted_checker));

    let mut inconsistent_outcomes = receipt;
    inconsistent_outcomes.producer_outcome = RepositoryRootCheckerOutcome::Refused {
        code: RepositoryRootRefusalCode::EmptySourceSet,
    };
    inconsistent_outcomes.receipt_sha256 = refusal_receipt_digest(&inconsistent_outcomes);
    assert!(!verify_refusal_receipt(&inconsistent_outcomes));

    let mut substituted_resource = inconsistent_outcomes.clone();
    substituted_resource.producer_outcome = RepositoryRootCheckerOutcome::Refused {
        code: RepositoryRootRefusalCode::SchemaMismatch,
    };
    substituted_resource.producer_resource_contract =
        RepositoryRootResourceContractOutcome::Refused {
            code: RepositoryRootRefusalCode::SourceCapacityExceeded,
        };
    substituted_resource.receipt_sha256 = refusal_receipt_digest(&substituted_resource);
    assert!(!verify_refusal_receipt(&substituted_resource));

    let mut added_early_canary = inconsistent_outcomes.clone();
    added_early_canary.producer_outcome = RepositoryRootCheckerOutcome::Refused {
        code: RepositoryRootRefusalCode::SchemaMismatch,
    };
    added_early_canary.canary.digest_sha256 = Some([20; 32]);
    added_early_canary.receipt_sha256 = refusal_receipt_digest(&added_early_canary);
    assert!(!verify_refusal_receipt(&added_early_canary));

    let mut substituted_stage = inconsistent_outcomes;
    substituted_stage.producer_outcome = RepositoryRootCheckerOutcome::Refused {
        code: RepositoryRootRefusalCode::SchemaMismatch,
    };
    substituted_stage.refusal_stage = RepositoryRootRefusalStage::Canary;
    substituted_stage.receipt_sha256 = refusal_receipt_digest(&substituted_stage);
    assert!(!verify_refusal_receipt(&substituted_stage));
}

#[test]
fn resource_preflight_refusal_has_no_canary_and_cannot_be_relabelled() {
    let packet = repository_packet();
    let decision = decide_packet_pair(
        Ok(packet.clone()),
        Ok(packet),
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        RepositoryRootResourceContractOutcome::Refused {
            code: RepositoryRootRefusalCode::SourceCapacityExceeded,
        },
    );
    let RepositoryRootDecision::Refused(refusal) = decision else {
        panic!("a resource-contract failure must refuse");
    };

    assert!(verify_refusal(&refusal));
    assert_eq!(
        refusal.receipt.refusal_stage,
        RepositoryRootRefusalStage::ResourceContractPreflight
    );
    assert_eq!(
        refusal.receipt.refusal_code,
        RepositoryRootRefusalCode::ResourceContractDisagreement
    );
    assert_eq!(refusal.receipt.canary.digest_sha256, None);

    let mut added_canary = refusal.receipt.clone();
    added_canary.canary.digest_sha256 = Some([6; 32]);
    added_canary.receipt_sha256 = refusal_receipt_digest(&added_canary);
    assert!(!verify_refusal_receipt(&added_canary));

    let mut substituted_decision = refusal.receipt;
    substituted_decision.refusal_code = RepositoryRootRefusalCode::CheckerDisagreement;
    substituted_decision.receipt_sha256 = refusal_receipt_digest(&substituted_decision);
    assert!(!verify_refusal_receipt(&substituted_decision));
}

#[test]
fn checker_disagreement_cannot_absorb_a_resource_or_canary_failure() {
    let decision = refused_decision(
        RepositoryRootRefusalCode::CheckerDisagreement,
        RepositoryRootRefusalStage::CheckerAgreement,
        [1; 32],
        [1; 32],
        RepositoryRootCheckerOutcome::Projected {
            input_sha256: [2; 32],
            result_sha256: [3; 32],
        },
        RepositoryRootCheckerOutcome::Refused {
            code: RepositoryRootRefusalCode::DecimalInvalid,
        },
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        None,
    );
    let RepositoryRootDecision::Refused(refusal) = decision else {
        panic!("checker disagreement must refuse");
    };
    assert!(verify_refusal(&refusal));

    let mut substituted_resource = refusal.receipt.clone();
    substituted_resource.watchdog_resource_contract =
        RepositoryRootResourceContractOutcome::Refused {
            code: RepositoryRootRefusalCode::CanonicalByteLimitExceeded,
        };
    substituted_resource.receipt_sha256 = refusal_receipt_digest(&substituted_resource);
    assert!(!verify_refusal_receipt(&substituted_resource));

    let mut added_canary = refusal.receipt;
    added_canary.canary.digest_sha256 = Some([5; 32]);
    added_canary.receipt_sha256 = refusal_receipt_digest(&added_canary);
    assert!(!verify_refusal_receipt(&added_canary));
}

#[test]
fn canary_stage_refusal_rejects_a_redigested_canary_graft() {
    let checker_outcome = RepositoryRootCheckerOutcome::Projected {
        input_sha256: [2; 32],
        result_sha256: [3; 32],
    };
    let decision = refused_decision(
        RepositoryRootRefusalCode::CanaryFailure,
        RepositoryRootRefusalStage::Canary,
        [1; 32],
        [1; 32],
        checker_outcome.clone(),
        checker_outcome,
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        None,
    );
    let RepositoryRootDecision::Refused(refusal) = decision else {
        panic!("a canary failure must refuse");
    };

    assert!(verify_refusal(&refusal));
    assert_eq!(refusal.receipt.canary.digest_sha256, None);

    let mut grafted_canary = refusal.receipt;
    grafted_canary.canary.digest_sha256 = Some([6; 32]);
    grafted_canary.receipt_sha256 = refusal_receipt_digest(&grafted_canary);
    assert!(!verify_refusal_receipt(&grafted_canary));
}

#[test]
fn post_canary_refusal_requires_the_available_canary_digest() {
    let checker_outcome = RepositoryRootCheckerOutcome::Projected {
        input_sha256: [2; 32],
        result_sha256: [3; 32],
    };
    let decision = refused_decision(
        RepositoryRootRefusalCode::ReceiptInvalid,
        RepositoryRootRefusalStage::ProjectionReceipt,
        [1; 32],
        [1; 32],
        checker_outcome.clone(),
        checker_outcome,
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        Some([6; 32]),
    );
    let RepositoryRootDecision::Refused(refusal) = decision else {
        panic!("an invalid acceptance receipt must refuse");
    };

    assert!(verify_refusal(&refusal));
    assert_eq!(
        refusal.receipt.canary.suite_id,
        REPOSITORY_ROOT_CANARY_SUITE_ID
    );
    assert_eq!(refusal.receipt.canary.digest_sha256, Some([6; 32]));
    assert_ne!(refusal.receipt.receipt_sha256, [0; 32]);

    let mut removed_canary = refusal.receipt.clone();
    removed_canary.canary.digest_sha256 = None;
    removed_canary.receipt_sha256 = refusal_receipt_digest(&removed_canary);
    assert!(!verify_refusal_receipt(&removed_canary));

    let mut substituted_resource = refusal.receipt;
    substituted_resource.watchdog_resource_contract =
        RepositoryRootResourceContractOutcome::Refused {
            code: RepositoryRootRefusalCode::CanonicalByteLimitExceeded,
        };
    substituted_resource.receipt_sha256 = refusal_receipt_digest(&substituted_resource);
    assert!(!verify_refusal_receipt(&substituted_resource));
}

#[test]
fn post_projection_canary_failure_is_stage_consistent() {
    let checker_outcome = RepositoryRootCheckerOutcome::Projected {
        input_sha256: [2; 32],
        result_sha256: [3; 32],
    };
    let decision = refused_decision(
        RepositoryRootRefusalCode::CanaryFailure,
        RepositoryRootRefusalStage::ProjectionReceipt,
        [1; 32],
        [1; 32],
        checker_outcome.clone(),
        checker_outcome,
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        RepositoryRootResourceContractOutcome::Bound {
            digest_sha256: [4; 32],
        },
        Some([6; 32]),
    );
    let RepositoryRootDecision::Refused(refusal) = decision else {
        panic!("a post-projection canary failure must refuse");
    };
    assert!(verify_refusal(&refusal));

    let mut missing_preprojection_canary = refusal.receipt.clone();
    missing_preprojection_canary.canary.digest_sha256 = None;
    missing_preprojection_canary.receipt_sha256 =
        refusal_receipt_digest(&missing_preprojection_canary);
    assert!(!verify_refusal_receipt(&missing_preprojection_canary));

    let mut wrong_stage = refusal.receipt.clone();
    wrong_stage.refusal_stage = RepositoryRootRefusalStage::Canary;
    wrong_stage.receipt_sha256 = refusal_receipt_digest(&wrong_stage);
    assert!(!verify_refusal_receipt(&wrong_stage));

    let mut disagreeing_pair = refusal.receipt.clone();
    disagreeing_pair.watchdog_outcome = RepositoryRootCheckerOutcome::Projected {
        input_sha256: [2; 32],
        result_sha256: [7; 32],
    };
    disagreeing_pair.receipt_sha256 = refusal_receipt_digest(&disagreeing_pair);
    assert!(!verify_refusal_receipt(&disagreeing_pair));

    let mut wrong_code = refusal.receipt;
    wrong_code.refusal_code = RepositoryRootRefusalCode::DecimalInvalid;
    wrong_code.receipt_sha256 = refusal_receipt_digest(&wrong_code);
    assert!(!verify_refusal_receipt(&wrong_code));
}

#[test]
fn finalization_errors_use_the_production_projection_receipt_mapping() {
    assert_eq!(
        projection_finalization_refusal(RepositoryRootRefusalCode::CanaryFailure),
        (
            RepositoryRootRefusalCode::CanaryFailure,
            RepositoryRootRefusalStage::ProjectionReceipt,
        )
    );
    assert_eq!(
        projection_finalization_refusal(RepositoryRootRefusalCode::ReceiptInvalid),
        (
            RepositoryRootRefusalCode::ReceiptInvalid,
            RepositoryRootRefusalStage::ProjectionReceipt,
        )
    );
}

#[test]
fn independent_projectors_agree_over_current_floor_shape() {
    let packet = producer::sealed_floor_packet().expect("producer extracts the sealed floor");
    let produced = producer::project(&packet).expect("producer accepts the typed floor packet");
    let watched = watchdog::project(&packet).expect("watchdog accepts the typed floor packet");

    assert_eq!(produced, watched);
    assert_eq!(produced.scalar_coordinate_count, 3);
    assert_eq!(produced.mass_projection_count, 1);
    assert_eq!(produced.candidates.len(), 4);
    assert_ne!(produced.input_sha256, [0; 32]);
    assert!(!produced.canonical_bytes.is_empty());
}

#[test]
fn current_floor_decimals_match_independently_pinned_exact_rationals() {
    let packet = watchdog::sealed_floor_packet().expect("watchdog extracts the sealed floor");
    let produced = producer::project(&packet).expect("producer accepts the typed floor packet");
    let watched = watchdog::project(&packet).expect("watchdog accepts the typed floor packet");
    let expected = [
        (
            "fundamental.alpha",
            &[0x10, 0xfd, 0x8f, 0xa2, 0xbd][..],
            &[0x09, 0x18, 0x4e, 0x72, 0xa0, 0x00][..],
        ),
        (
            "fundamental.G",
            &[0x01, 0x04, 0xb7][..],
            &[0x03, 0x8d, 0x7e, 0xa4, 0xc6, 0x80, 0x00][..],
        ),
        (
            "fundamental.m_e",
            &[0x04, 0x3d, 0xec, 0x54, 0x2b][..],
            &[
                0x3a, 0xc6, 0x53, 0xe3, 0x86, 0xb9, 0x49, 0x7f, 0x57, 0x73, 0xea, 0xc2, 0x00, 0x00,
                0x00, 0x00, 0x00,
            ][..],
        ),
    ];

    for output in [&produced, &watched] {
        for (entry_id, numerator_be, denominator_be) in expected {
            let rational = output
                .candidates
                .iter()
                .find_map(|candidate| {
                    if candidate.source_entry_id != entry_id {
                        return None;
                    }
                    match &candidate.payload {
                        ArtifactPayload::ScalarCoordinate(coordinate) => {
                            Some(&coordinate.exact_value)
                        }
                        _ => None,
                    }
                })
                .expect("every sealed floor leaf has one scalar coordinate");
            assert!(!rational.negative);
            assert_eq!(rational.numerator_be, numerator_be);
            assert_eq!(rational.denominator_be, denominator_be);
        }
    }
}

#[test]
fn paired_projection_is_derived_and_membership_neutral() {
    let projection = project_repository_roots().expect("the independent sealed pair agrees");

    assert!(!projection.membership_authority);
    assert!(!projection.receipt.membership_authority);
    assert_eq!(projection.receipt.scalar_coordinate_count, 3);
    assert_eq!(projection.receipt.mass_projection_count, 1);
    assert_eq!(projection.admitted_artifacts.len(), 4);
    assert_ne!(
        projection.receipt.producer_id,
        projection.receipt.watchdog_id
    );

    let mut identities = BTreeSet::new();
    for artifact in &projection.admitted_artifacts {
        assert!(identities.insert(artifact.claimed_identity));
        assert_eq!(artifact.admission.tier, LedgerTier::Universal);
        assert_eq!(artifact.admission.provenance, ProvenanceMark::Derived);
        let AdmissionRoute::Derived(admission) = &artifact.admission.route else {
            panic!("repository coordinate roots must use the derived route");
        };
        let receipts = [
            &admission.ancestry_receipt,
            &admission.semantic_checker_receipt,
            &admission.independent_watchdog_receipt,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(receipts.len(), 3);
        match &artifact.payload {
            ArtifactPayload::ScalarCoordinate(_) => {}
            ArtifactPayload::MassProjection(projection) => {
                assert_eq!(projection.scope, MassProjectionScope::MembershipNeutral);
                assert_eq!(projection.expression.nodes.len(), 1);
                assert_eq!(projection.expression.output_node, 0);
                assert!(matches!(
                    projection.expression.nodes[0],
                    ExactExpressionNode::Coordinate(_)
                ));
            }
            payload => panic!("unexpected member-forming payload: {payload:?}"),
        }
    }
}

#[test]
fn root_identities_match_the_physical_registry_encoder() {
    let projection = project_repository_roots().expect("the independent sealed pair agrees");
    for artifact in projection.admitted_artifacts {
        let expected = super::super::producer::derive_artifact_identity_for_test(&artifact.payload)
            .expect("physical registry can encode the projected root");
        assert_eq!(artifact.claimed_identity, expected);
    }
}

#[test]
fn sealed_projection_canonicalizes_packet_source_reordering() {
    let packet = producer::sealed_floor_packet().expect("producer extracts the sealed floor");
    let mut reversed = packet.clone();
    reversed.sources.reverse();

    let producer = producer::project(&packet).expect("producer projects the sealed packet");
    let producer_reordered =
        producer::project(&reversed).expect("producer canonicalizes source arrival order");
    let watchdog = watchdog::project(&packet).expect("watchdog projects the sealed packet");
    let watchdog_reordered =
        watchdog::project(&reversed).expect("watchdog canonicalizes source arrival order");
    assert_eq!(producer, producer_reordered);
    assert_eq!(watchdog, watchdog_reordered);

    let expected = inspect_unsealed_packet_for_test(&packet).expect("ordered test packet projects");
    let observed =
        inspect_unsealed_packet_for_test(&reversed).expect("generic test projector keeps order");
    assert_eq!(expected, observed);
}

#[test]
fn presentation_equivalent_decimals_preserve_semantic_projection_identity() {
    let packet = repository_packet();
    let mut alternate = packet.clone();
    alternate.sources[0].central_decimal = "0.0072973525693".to_owned();
    alternate.sources[0].uncertainty_decimal = "0.0000000000011".to_owned();

    let expected = inspect_unsealed_packet_for_test(&packet).expect("base packet projects");
    let observed =
        inspect_unsealed_packet_for_test(&alternate).expect("equivalent decimal packet projects");

    assert_eq!(expected.input_sha256, observed.input_sha256);
    assert_eq!(expected.canonical_bytes, observed.canonical_bytes);
    assert_eq!(
        expected
            .candidates
            .iter()
            .map(|candidate| candidate.identity)
            .collect::<Vec<_>>(),
        observed
            .candidates
            .iter()
            .map(|candidate| candidate.identity)
            .collect::<Vec<_>>()
    );

    let baseline = project_repository_roots().expect("the sealed pair agrees");
    let receipt = |output: &RootCheckerOutput| {
        let result_sha256 = sha256(&output.canonical_bytes);
        let producer_ancestry_manifest_sha256 =
            producer::ancestry_manifest_sha256(&output.candidates)
                .expect("producer ancestry manifest");
        let watchdog_ancestry_manifest_sha256 =
            watchdog::ancestry_manifest_sha256(&output.candidates)
                .expect("watchdog ancestry manifest");
        let mut receipt = RepositoryRootProjectionReceipt {
            input_sha256: output.input_sha256,
            result_sha256,
            producer_result_sha256: result_sha256,
            watchdog_result_sha256: result_sha256,
            producer_ancestry_manifest_sha256,
            watchdog_ancestry_manifest_sha256,
            scalar_coordinate_count: output.scalar_coordinate_count,
            mass_projection_count: output.mass_projection_count,
            receipt_sha256: [0; 32],
            ..baseline.receipt.clone()
        };
        receipt.receipt_sha256 = pair_receipt_digest(&receipt);
        receipt
    };
    assert_ne!(receipt(&expected), receipt(&observed));
}

#[test]
fn authority_receipt_changes_do_not_change_semantic_projection_bytes() {
    let packet = repository_packet();
    let mut alternate_leaf = packet.clone();
    alternate_leaf.sources[0].exhaustion_binding.digest_sha256 = [91; 32];
    let mut alternate_floor = packet.clone();
    alternate_floor.floor_authority.digest_sha256 = [92; 32];

    let expected = inspect_unsealed_packet_for_test(&packet).expect("base packet projects");
    let observed_leaf = inspect_unsealed_packet_for_test(&alternate_leaf)
        .expect("alternate leaf-evidence packet projects");
    let observed_floor = inspect_unsealed_packet_for_test(&alternate_floor)
        .expect("alternate floor-authority packet projects");

    assert_eq!(expected.input_sha256, observed_leaf.input_sha256);
    assert_eq!(expected.canonical_bytes, observed_leaf.canonical_bytes);
    assert_eq!(expected.input_sha256, observed_floor.input_sha256);
    assert_eq!(expected.canonical_bytes, observed_floor.canonical_bytes);
    let expected_alpha = expected
        .candidates
        .iter()
        .find(|candidate| candidate.source_entry_id == "fundamental.alpha")
        .expect("the alpha coordinate is present");
    let observed_leaf_alpha = observed_leaf
        .candidates
        .iter()
        .find(|candidate| candidate.source_entry_id == "fundamental.alpha")
        .expect("the alpha coordinate is present");
    let observed_floor_alpha = observed_floor
        .candidates
        .iter()
        .find(|candidate| candidate.source_entry_id == "fundamental.alpha")
        .expect("the alpha coordinate is present");
    assert_ne!(
        expected_alpha.ancestry_digest_sha256,
        observed_leaf_alpha.ancestry_digest_sha256
    );
    assert_ne!(
        expected_alpha.ancestry_digest_sha256,
        observed_floor_alpha.ancestry_digest_sha256
    );
}

#[test]
fn coordinated_identity_rename_with_old_receipt_refuses() {
    let mut packet = producer::sealed_floor_packet().expect("producer extracts the sealed floor");
    packet.sources[0].entry_id = "fundamental.unfamiliar_alpha".to_owned();
    packet.sources[0].symbol = "unfamiliar_alpha".to_owned();

    assert_eq!(
        producer::project(&packet),
        Err(RepositoryRootRefusalCode::SealedSourceBindingMismatch)
    );
    assert_eq!(
        watchdog::project(&packet),
        Err(RepositoryRootRefusalCode::SealedSourceBindingMismatch)
    );
}

#[test]
fn unfamiliar_positive_mass_coordinate_gets_no_species_identity_shortcut() {
    let packet = RepositoryRootPacket::new(
        binding("civsim.test.floor-authority.v1", 1),
        vec![source(
            "alien_mass_coordinate",
            "3.125e2",
            "2.5e-3",
            MASS_DIMENSION,
            21,
        )],
    );
    let projection =
        inspect_unsealed_packet_for_test(&packet).expect("unfamiliar floor coordinate is generic");

    assert_eq!(projection.scalar_coordinate_count, 1);
    assert_eq!(projection.mass_projection_count, 1);
    assert!(projection.candidates.iter().all(|candidate| matches!(
        candidate.payload,
        ArtifactPayload::ScalarCoordinate(_) | ArtifactPayload::MassProjection(_)
    )));
}

#[test]
fn nonpositive_or_nonmass_coordinates_do_not_get_mass_projections() {
    let packet = RepositoryRootPacket::new(
        binding("civsim.test.floor-authority.v1", 1),
        vec![
            source("negative_mass", "-3.0", "1e-2", MASS_DIMENSION, 31),
            source(
                "positive_time",
                "4.0",
                "1e-2",
                si_dimension([0, 0, 1, 0, 0, 0, 0]),
                32,
            ),
        ],
    );
    let projection =
        inspect_unsealed_packet_for_test(&packet).expect("both values remain scalar coordinates");

    assert_eq!(projection.scalar_coordinate_count, 2);
    assert_eq!(projection.mass_projection_count, 0);
}

#[test]
fn value_uncertainty_and_dimension_are_semantic_identity_inputs() {
    let baseline =
        inspect_unsealed_packet_for_test(&repository_packet()).expect("baseline projects");
    let baseline_digest = sha256(&baseline.canonical_bytes);

    let mut mutations = Vec::new();
    let mut value = repository_packet();
    value.sources[0].central_decimal = "7.2973525694e-3".to_owned();
    mutations.push(value);

    let mut uncertainty = repository_packet();
    uncertainty.sources[0].uncertainty_decimal = "1.2e-12".to_owned();
    mutations.push(uncertainty);

    let mut dimension = repository_packet();
    dimension.sources[0].dimension = si_dimension([1, 0, 0, 0, 0, 0, 0]);
    mutations.push(dimension);

    for mutation in mutations {
        let observed =
            inspect_unsealed_packet_for_test(&mutation).expect("valid mutation still projects");
        assert_ne!(sha256(&observed.canonical_bytes), baseline_digest);
    }
}

fn assert_paired_refusal(packet: &RepositoryRootPacket, expected: RepositoryRootRefusalCode) {
    assert_eq!(producer::project(packet), Err(expected));
    assert_eq!(watchdog::project(packet), Err(expected));
    assert_eq!(inspect_unsealed_packet_for_test(packet), Err(expected));
}

#[test]
fn representation_definition_provenance_cannot_enter_the_root_packet() {
    let mut packet = RepositoryRootPacket::new(
        binding("civsim.test.floor-authority.v1", 1),
        vec![source(
            "h",
            "6.62607015e-34",
            "0",
            si_dimension([2, 1, -1, 0, 0, 0, 0]),
            41,
        )],
    );
    packet.sources[0].source_provenance = ProvenanceMark::Derived;

    assert_paired_refusal(
        &packet,
        RepositoryRootRefusalCode::SourceProvenanceNotMeasured,
    );
}

#[test]
fn entry_identity_must_be_the_bound_fundamental_symbol() {
    let mut packet = repository_packet();
    packet.sources[0].entry_id = "fundamental.other".to_owned();

    assert_paired_refusal(&packet, RepositoryRootRefusalCode::CanonicalTextInvalid);
}

#[test]
fn membership_authority_is_refused_by_both_paths() {
    let mut packet = repository_packet();
    packet.membership_authority = true;

    assert_paired_refusal(
        &packet,
        RepositoryRootRefusalCode::MembershipAuthorityPresent,
    );
}

#[test]
fn leaf_fingerprints_must_be_nonzero_and_distinct() {
    let mut duplicate = repository_packet();
    duplicate.sources[1].exhaustion_binding.digest_sha256 =
        duplicate.sources[0].exhaustion_binding.digest_sha256;
    assert_paired_refusal(
        &duplicate,
        RepositoryRootRefusalCode::DuplicateLeafFingerprint,
    );

    let mut zero = repository_packet();
    zero.sources[0].exhaustion_binding.digest_sha256 = [0; 32];
    assert_paired_refusal(&zero, RepositoryRootRefusalCode::MissingBindingDigest);
}

#[test]
fn negative_uncertainty_is_not_a_coordinate_root() {
    let mut packet = repository_packet();
    packet.sources[0].uncertainty_decimal = "-1.1e-12".to_owned();

    assert_paired_refusal(&packet, RepositoryRootRefusalCode::NegativeUncertainty);
}
