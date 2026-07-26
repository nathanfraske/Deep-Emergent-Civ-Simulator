use super::super::{
    model::{
        AdmissionRoute, AdmittedArtifact, ArtifactPayload, CanonicalArtifact,
        ConstraintLawArtifact, DerivedAdmission, DimensionVector, ExactRationalWire, LedgerTier,
        ProvenanceMark, ReceiptBinding, RequirementSet, RootAdmission, ScalarCoordinateArtifact,
    },
    producer as registry_producer,
    repository_roots::{decide_repository_roots, verify_projection, RepositoryRootDecision},
};
use super::{
    derive_and_agree, derive_binding, producer, producer_accepts_binding, verify_agreement,
    watchdog, watchdog_accepts_binding, VocabularyRefusalCode, VocabularyResourceContract,
};

fn receipt(label: &str, tag: u8) -> ReceiptBinding {
    ReceiptBinding {
        schema_id: format!("synthetic.{label}.v1"),
        digest_sha256: [tag.max(1); 32],
    }
}

fn admission(tag: u8) -> RootAdmission {
    RootAdmission {
        tier: LedgerTier::Universal,
        provenance: ProvenanceMark::Derived,
        route: AdmissionRoute::Derived(DerivedAdmission {
            ancestry_receipt: receipt("ancestry", tag),
            semantic_checker_receipt: receipt("semantic", tag.wrapping_add(1)),
            independent_watchdog_receipt: receipt("watchdog", tag.wrapping_add(2)),
        }),
    }
}

fn content(label: &str, tag: u8) -> CanonicalArtifact {
    CanonicalArtifact {
        schema_id: format!("synthetic.unfamiliar.{label}.v1"),
        canonical_bytes: vec![tag, tag.rotate_left(1), tag.wrapping_mul(19)],
    }
}

fn admitted(payload: ArtifactPayload, tag: u8) -> AdmittedArtifact {
    let identity = registry_producer::derive_artifact_identity_for_test(&payload)
        .expect("synthetic root identity derives");
    AdmittedArtifact::from_exact_test_recomputation(identity, admission(tag), payload)
}

fn alien_roots() -> Vec<AdmittedArtifact> {
    vec![
        admitted(
            ArtifactPayload::PhysicalDescriptor(content("anisotropic-mode", 31)),
            31,
        ),
        admitted(
            ArtifactPayload::ConstraintLaw(ConstraintLawArtifact {
                requirements: RequirementSet {
                    artifact_relations: Vec::new(),
                    species_dependencies: Vec::new(),
                },
            }),
            41,
        ),
        admitted(
            ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
                coordinate: content("phase-coordinate", 53),
                exact_value: ExactRationalWire {
                    negative: false,
                    numerator_be: vec![1],
                    denominator_be: vec![1],
                },
                dimension: DimensionVector::dimensionless(),
            })),
            53,
        ),
    ]
}

#[test]
fn physical_vocabulary_current_repository_is_zero_four_zero_without_authority() {
    let projection = match decide_repository_roots() {
        RepositoryRootDecision::Projected(projection) => projection,
        RepositoryRootDecision::Refused(refusal) => {
            panic!(
                "current repository root projection refused: {:?}",
                refusal.code
            )
        }
    };
    assert!(verify_projection(&projection));
    let agreement =
        derive_and_agree(&projection.admitted_artifacts).expect("current roots classify");
    assert_eq!(agreement.receipt.descriptor_role_count, 0);
    assert_eq!(agreement.receipt.relation_target_count, 4);
    assert_eq!(agreement.receipt.constraint_law_count, 0);
    assert!(agreement.receipt.current_input_partition_complete);
    assert!(!agreement.receipt.global_physical_vocabulary_coverage);
    assert!(!agreement.receipt.membership_authority);
    assert!(verify_agreement(&agreement));

    let binding = derive_binding(&projection.admitted_artifacts).expect("current roots bind");
    assert!(producer_accepts_binding(
        &projection.admitted_artifacts,
        &binding
    ));
    assert!(watchdog_accepts_binding(
        &projection.admitted_artifacts,
        &binding
    ));

    let mut mutation = binding;
    mutation.receipt_sha256[0] ^= 1;
    assert!(!producer_accepts_binding(
        &projection.admitted_artifacts,
        &mutation
    ));
    assert!(!watchdog_accepts_binding(
        &projection.admitted_artifacts,
        &mutation
    ));
}

#[test]
fn physical_vocabulary_alien_payloads_are_data_classified_and_order_invariant() {
    let roots = alien_roots();
    let agreement = derive_and_agree(&roots).expect("unfamiliar roots classify");
    assert_eq!(agreement.receipt.descriptor_role_count, 1);
    assert_eq!(agreement.receipt.relation_target_count, 3);
    assert_eq!(agreement.receipt.constraint_law_count, 1);
    assert!(agreement
        .partitions
        .descriptor_role_identities
        .iter()
        .all(|identity| agreement
            .partitions
            .relation_target_identities
            .binary_search(identity)
            .is_ok()));

    let mut reversed = roots;
    reversed.reverse();
    let reordered = derive_and_agree(&reversed).expect("arrival order is nonphysical");
    assert_eq!(agreement, reordered);
}

#[test]
fn physical_vocabulary_receipt_mutations_fail_closed() {
    let agreement = derive_and_agree(&alien_roots()).expect("fixture classifies");

    let mut mutation = agreement.clone();
    mutation.partitions.descriptor_role_identities[0].0[0] ^= 1;
    assert!(!verify_agreement(&mutation));

    let mut mutation = agreement.clone();
    mutation.receipt.membership_authority = true;
    assert!(!verify_agreement(&mutation));

    let mut mutation = agreement.clone();
    mutation.receipt.global_physical_vocabulary_coverage = true;
    assert!(!verify_agreement(&mutation));

    let mut mutation = agreement.clone();
    mutation.receipt.relation_target_count -= 1;
    assert!(!verify_agreement(&mutation));

    let mut mutation = agreement.clone();
    mutation.receipt.producer_result_sha256[0] ^= 1;
    assert!(!verify_agreement(&mutation));

    let mut mutation = agreement.clone();
    mutation.canonical_bytes.push(0);
    assert!(!verify_agreement(&mutation));

    let mut mutation = agreement;
    mutation.receipt.producer_resource_contract_sha256[0] ^= 1;
    assert!(!verify_agreement(&mutation));
}

#[test]
fn physical_vocabulary_resource_domains_refuse_under_both_algorithms() {
    let roots = alien_roots();
    let mut resources = VocabularyResourceContract::PRODUCTION;
    resources.max_root_count = 2;
    assert_eq!(
        producer::classify(&roots, resources),
        Err(VocabularyRefusalCode::RootCountExceeded)
    );
    assert_eq!(
        watchdog::classify(&roots, resources),
        Err(VocabularyRefusalCode::RootCountExceeded)
    );

    let mut resources = VocabularyResourceContract::PRODUCTION;
    resources.max_partition_identity_count = 4;
    assert_eq!(
        producer::classify(&roots, resources),
        Err(VocabularyRefusalCode::PartitionIdentityCountExceeded)
    );
    assert_eq!(
        watchdog::classify(&roots, resources),
        Err(VocabularyRefusalCode::PartitionIdentityCountExceeded)
    );

    let mut resources = VocabularyResourceContract::PRODUCTION;
    resources.max_work_units = 1;
    assert_eq!(
        producer::classify(&roots, resources),
        Err(VocabularyRefusalCode::WorkLimitExceeded)
    );
    assert_eq!(
        watchdog::classify(&roots, resources),
        Err(VocabularyRefusalCode::WorkLimitExceeded)
    );

    let mut resources = VocabularyResourceContract::PRODUCTION;
    resources.max_canonical_bytes = 1;
    assert_eq!(
        producer::classify(&roots, resources),
        Err(VocabularyRefusalCode::CanonicalBytesExceeded)
    );
    assert_eq!(
        watchdog::classify(&roots, resources),
        Err(VocabularyRefusalCode::CanonicalBytesExceeded)
    );
}

#[test]
fn physical_vocabulary_duplicate_and_nonadmitted_roots_refuse() {
    let mut duplicate = alien_roots();
    duplicate.push(duplicate[0].clone());
    assert_eq!(
        producer::classify(&duplicate, VocabularyResourceContract::PRODUCTION),
        Err(VocabularyRefusalCode::DuplicateRootIdentity)
    );
    assert_eq!(
        watchdog::classify(&duplicate, VocabularyResourceContract::PRODUCTION),
        Err(VocabularyRefusalCode::DuplicateRootIdentity)
    );

    let mut custody_only = alien_roots();
    custody_only[0].admission.route = AdmissionRoute::EvidenceCustodyOnly {
        source_receipt: receipt("source", 71),
    };
    custody_only[0].refresh_exact_test_capability();
    assert_eq!(
        producer::classify(&custody_only, VocabularyResourceContract::PRODUCTION),
        Err(VocabularyRefusalCode::EvidenceCustodyOnly)
    );
    assert_eq!(
        watchdog::classify(&custody_only, VocabularyResourceContract::PRODUCTION),
        Err(VocabularyRefusalCode::EvidenceCustodyOnly)
    );
}
