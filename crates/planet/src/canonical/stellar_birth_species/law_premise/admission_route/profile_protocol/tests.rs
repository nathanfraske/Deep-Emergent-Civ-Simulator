use super::*;

fn id(value: u8) -> [u8; 32] {
    [value; 32]
}

fn input() -> ProfileProtocolInput {
    ProfileProtocolInput {
        claim_identity: id(1),
        role_identity: id(2),
        content_identity: id(3),
        profile_input_sha256: id(4),
        source_custody_sha256: id(5),
        applicability_receipt_sha256: id(6),
        validity_receipt_sha256: id(7),
        derivation_catalog_sha256: id(8),
        repository_seeds: vec![CatalogSeed {
            claim_identity: id(20),
            role_identity: id(21),
            content_identity: id(22),
            capability_sha256: id(23),
        }],
        repository_rules: Vec::new(),
        residual_slot_id: "planet.test.profile-slot".to_owned(),
        occupied_residual_slots: vec![
            "floor.alpha".to_owned(),
            "floor.big-g".to_owned(),
            "floor.electron-mass".to_owned(),
        ],
        owner_admission_record: "owner-reviewed-test-profile".to_owned(),
    }
}

#[test]
fn independent_pair_scans_catalog_and_slot_inventory() {
    let evidence = inspect(&input()).expect("profile protocol evidence");
    assert_eq!(evidence.assessment.repository_seed_count, 1);
    assert_eq!(evidence.assessment.repository_rule_count, 0);
    assert_eq!(evidence.assessment.target_scoped_seed_count, 0);
    assert_eq!(evidence.assessment.target_scoped_rule_count, 0);
    assert_eq!(evidence.assessment.numeric_basis_count, 0);
    assert_eq!(evidence.assessment.trajectory_coordinate_count, 0);
    assert_eq!(evidence.assessment.occupied_slot_count, 3);
    assert!(!evidence.assessment.residual_slot_collision);
    assert_eq!(
        evidence.producer_result_sha256,
        evidence.watchdog_result_sha256
    );
    assert_ne!(evidence.producer_receipts, evidence.watchdog_receipts);
}

#[test]
fn catalog_target_or_slot_collision_refuses_pair_evidence() {
    let mut target_seed = input();
    target_seed.repository_seeds.push(CatalogSeed {
        claim_identity: target_seed.claim_identity,
        role_identity: target_seed.role_identity,
        content_identity: target_seed.content_identity,
        capability_sha256: id(24),
    });
    assert_eq!(
        inspect(&target_seed),
        Err("profile_protocol_checker_disagreement")
    );

    let mut collision = input();
    collision
        .occupied_residual_slots
        .push(collision.residual_slot_id.clone());
    assert_eq!(
        inspect(&collision),
        Err("profile_protocol_checker_disagreement")
    );
}

#[test]
fn alien_identity_renaming_preserves_structure_but_changes_receipts() {
    let baseline = inspect(&input()).unwrap();
    let mut alien = input();
    alien.claim_identity = id(101);
    alien.role_identity = id(102);
    alien.content_identity = id(103);
    let renamed = inspect(&alien).unwrap();
    assert_eq!(baseline.assessment, renamed.assessment);
    assert_ne!(
        baseline.producer_receipts.coverage_sha256,
        renamed.producer_receipts.coverage_sha256
    );
}
