//! Ordered-set profile protocol producer.

use super::{
    CatalogRule, CatalogSeed, ProfileProtocolAssessment, ProfileProtocolCheckerOutput,
    ProfileProtocolCheckerReceipts, ProfileProtocolInput,
};
use civsim_units::digest::sha256;
use std::collections::BTreeSet;

const ASSESSMENT_SCHEMA: &[u8] = b"civsim.planet.theory-profile-protocol-assessment.v1";
const CATALOG_DOMAIN: &[u8] = b"civsim.planet.theory-profile-catalog.v1";
const SLOT_DOMAIN: &[u8] = b"civsim.planet.theory-profile-slot-inventory.v1";
const RECEIPT_DOMAINS: [&[u8]; 7] = [
    b"civsim.planet.theory-profile-coverage.producer.v2",
    b"civsim.planet.theory-profile-buckingham-pi.producer.v2",
    b"civsim.planet.theory-profile-gap-law.producer.v2",
    b"civsim.planet.theory-profile-chaos.producer.v2",
    b"civsim.planet.theory-profile-residual.producer.v2",
    b"civsim.planet.theory-profile-slot.producer.v2",
    b"civsim.planet.theory-profile-owner.producer.v2",
];

pub(super) fn inspect(
    input: &ProfileProtocolInput,
) -> Result<ProfileProtocolCheckerOutput, &'static str> {
    validate_fixed(input)?;
    let seeds = input
        .repository_seeds
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let rules = input
        .repository_rules
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if seeds.len() != input.repository_seeds.len()
        || rules.len() != input.repository_rules.len()
        || seeds.iter().any(invalid_seed)
        || rules.iter().any(invalid_rule)
    {
        return Err("invalid_repository_derivation_catalog");
    }
    let target_scoped_seed_count = u32::try_from(
        seeds
            .iter()
            .filter(|seed| {
                seed.claim_identity == input.claim_identity
                    && seed.role_identity == input.role_identity
                    && seed.content_identity == input.content_identity
            })
            .count(),
    )
    .map_err(|_| "catalog_count_overflow")?;
    let target_scoped_rule_count = u32::try_from(
        rules
            .iter()
            .filter(|rule| {
                rule.claim_identity == input.claim_identity
                    && rule.conclusion_role_identity == input.role_identity
                    && rule.conclusion_content_identity == input.content_identity
            })
            .count(),
    )
    .map_err(|_| "catalog_count_overflow")?;
    let repository_catalog_sha256 = catalog_digest(&seeds, &rules);

    let slots = input
        .occupied_residual_slots
        .iter()
        .map(|slot| slot.trim().to_owned())
        .collect::<BTreeSet<_>>();
    if slots.len() != input.occupied_residual_slots.len()
        || slots.iter().any(|slot| !valid_text(slot))
    {
        return Err("invalid_occupied_slot_inventory");
    }
    let residual_slot = input.residual_slot_id.trim();
    let occupied_slot_inventory_sha256 = slot_inventory_digest(&slots);
    let assessment = ProfileProtocolAssessment {
        repository_catalog_sha256,
        repository_seed_count: u32::try_from(seeds.len()).map_err(|_| "catalog_count_overflow")?,
        repository_rule_count: u32::try_from(rules.len()).map_err(|_| "catalog_count_overflow")?,
        target_scoped_seed_count,
        target_scoped_rule_count,
        numeric_basis_count: 0,
        trajectory_coordinate_count: 0,
        occupied_slot_count: u32::try_from(slots.len()).map_err(|_| "slot_count_overflow")?,
        occupied_slot_inventory_sha256,
        residual_slot_collision: slots.contains(residual_slot),
    };
    let canonical_bytes = encode_assessment(&assessment);
    let receipts = build_receipts(input, &canonical_bytes);
    Ok(ProfileProtocolCheckerOutput {
        assessment,
        canonical_bytes,
        receipts,
    })
}

fn validate_fixed(input: &ProfileProtocolInput) -> Result<(), &'static str> {
    let fixed = [
        input.claim_identity,
        input.role_identity,
        input.content_identity,
        input.profile_input_sha256,
        input.source_custody_sha256,
        input.applicability_receipt_sha256,
        input.validity_receipt_sha256,
        input.derivation_catalog_sha256,
    ];
    if fixed.contains(&[0; 32])
        || fixed
            .iter()
            .enumerate()
            .any(|(index, digest)| fixed[index + 1..].contains(digest))
        || !valid_text(input.residual_slot_id.trim())
        || !valid_text(input.owner_admission_record.trim())
    {
        return Err("invalid_profile_protocol_input");
    }
    Ok(())
}

fn invalid_seed(seed: &CatalogSeed) -> bool {
    [
        seed.claim_identity,
        seed.role_identity,
        seed.content_identity,
        seed.capability_sha256,
    ]
    .contains(&[0; 32])
}

fn invalid_rule(rule: &CatalogRule) -> bool {
    [
        rule.claim_identity,
        rule.conclusion_role_identity,
        rule.conclusion_content_identity,
        rule.capability_sha256,
    ]
    .contains(&[0; 32])
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 192 && value.is_ascii()
}

fn catalog_digest(seeds: &BTreeSet<CatalogSeed>, rules: &BTreeSet<CatalogRule>) -> [u8; 32] {
    let mut bytes = CATALOG_DOMAIN.to_vec();
    push(&mut bytes, 1, &usize_be(seeds.len()));
    for seed in seeds {
        for (tag, value) in [
            (2, seed.claim_identity),
            (3, seed.role_identity),
            (4, seed.content_identity),
            (5, seed.capability_sha256),
        ] {
            push(&mut bytes, tag, &value);
        }
    }
    push(&mut bytes, 6, &usize_be(rules.len()));
    for rule in rules {
        for (tag, value) in [
            (7, rule.claim_identity),
            (8, rule.conclusion_role_identity),
            (9, rule.conclusion_content_identity),
            (10, rule.capability_sha256),
        ] {
            push(&mut bytes, tag, &value);
        }
    }
    sha256(&bytes)
}

fn slot_inventory_digest(slots: &BTreeSet<String>) -> [u8; 32] {
    let mut bytes = SLOT_DOMAIN.to_vec();
    push(&mut bytes, 1, &usize_be(slots.len()));
    for slot in slots {
        push(&mut bytes, 2, slot.as_bytes());
    }
    sha256(&bytes)
}

fn encode_assessment(assessment: &ProfileProtocolAssessment) -> Vec<u8> {
    let mut bytes = ASSESSMENT_SCHEMA.to_vec();
    for (tag, value) in [
        (1, assessment.repository_catalog_sha256),
        (2, assessment.occupied_slot_inventory_sha256),
    ] {
        push(&mut bytes, tag, &value);
    }
    for (tag, value) in [
        (3, assessment.repository_seed_count),
        (4, assessment.repository_rule_count),
        (5, assessment.target_scoped_seed_count),
        (6, assessment.target_scoped_rule_count),
        (7, assessment.numeric_basis_count),
        (8, assessment.trajectory_coordinate_count),
        (9, assessment.occupied_slot_count),
    ] {
        push(&mut bytes, tag, &value.to_be_bytes());
    }
    push(
        &mut bytes,
        10,
        &[u8::from(assessment.residual_slot_collision)],
    );
    bytes
}

fn build_receipts(
    input: &ProfileProtocolInput,
    canonical_bytes: &[u8],
) -> ProfileProtocolCheckerReceipts {
    let values = RECEIPT_DOMAINS.map(|domain| {
        let mut bytes = domain.to_vec();
        push(&mut bytes, 1, canonical_bytes);
        push(&mut bytes, 2, &input.claim_identity);
        push(&mut bytes, 3, &input.profile_input_sha256);
        push(&mut bytes, 4, input.residual_slot_id.trim().as_bytes());
        push(
            &mut bytes,
            5,
            input.owner_admission_record.trim().as_bytes(),
        );
        push(&mut bytes, 6, &input.source_custody_sha256);
        push(&mut bytes, 7, &input.applicability_receipt_sha256);
        push(&mut bytes, 8, &input.validity_receipt_sha256);
        push(&mut bytes, 9, &input.derivation_catalog_sha256);
        sha256(&bytes)
    });
    ProfileProtocolCheckerReceipts {
        coverage_sha256: values[0],
        buckingham_pi_sha256: values[1],
        gap_law_sha256: values[2],
        chaos_protocol_sha256: values[3],
        residual_law_sha256: values[4],
        residual_slot_sha256: values[5],
        owner_admission_sha256: values[6],
    }
}

fn push(bytes: &mut Vec<u8>, tag: u16, value: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(value.len())
            .expect("bounded profile protocol field")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(value);
}

fn usize_be(value: usize) -> [u8; 8] {
    u64::try_from(value)
        .expect("bounded profile protocol inventory")
        .to_be_bytes()
}
