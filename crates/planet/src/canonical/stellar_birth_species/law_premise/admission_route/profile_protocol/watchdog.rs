//! Sorted-vector profile protocol watchdog.

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
    b"civsim.planet.theory-profile-coverage.watchdog.v3",
    b"civsim.planet.theory-profile-buckingham-pi.watchdog.v3",
    b"civsim.planet.theory-profile-gap-law.watchdog.v3",
    b"civsim.planet.theory-profile-chaos.watchdog.v3",
    b"civsim.planet.theory-profile-residual.watchdog.v3",
    b"civsim.planet.theory-profile-slot.watchdog.v3",
    b"civsim.planet.theory-profile-owner.watchdog.v3",
];

pub(super) fn inspect(
    input: &ProfileProtocolInput,
) -> Result<ProfileProtocolCheckerOutput, &'static str> {
    validate_fixed_reverse(input)?;
    let mut seeds = input.repository_seeds.clone();
    seeds.sort_unstable();
    let mut rules = input.repository_rules.clone();
    rules.sort_unstable();
    if seeds.windows(2).any(|pair| pair[0] == pair[1])
        || rules.windows(2).any(|pair| pair[0] == pair[1])
        || seeds.iter().rev().any(invalid_seed)
        || rules.iter().rev().any(invalid_rule)
    {
        return Err("invalid_repository_derivation_catalog");
    }
    let target_scoped_seed_count = seeds.iter().try_fold(0_u32, |count, seed| {
        if seed.claim_identity == input.claim_identity
            && seed.role_identity == input.role_identity
            && seed.content_identity == input.content_identity
        {
            count.checked_add(1).ok_or("catalog_count_overflow")
        } else {
            Ok(count)
        }
    })?;
    let target_scoped_rule_count = rules.iter().try_fold(0_u32, |count, rule| {
        if rule.claim_identity == input.claim_identity
            && rule.conclusion_role_identity == input.role_identity
            && rule.conclusion_content_identity == input.content_identity
        {
            count.checked_add(1).ok_or("catalog_count_overflow")
        } else {
            Ok(count)
        }
    })?;
    let repository_catalog_sha256 = catalog_digest(&seeds, &rules);

    if input
        .occupied_residual_slots
        .iter()
        .rev()
        .any(|slot| !valid_canonical_text(slot))
    {
        return Err("invalid_occupied_slot_inventory");
    }
    let mut slots = input.occupied_residual_slots.to_vec();
    slots.sort_unstable();
    if slots.windows(2).any(|pair| pair[0] == pair[1])
        || slots.iter().rev().any(|slot| !valid_text(slot))
    {
        return Err("invalid_occupied_slot_inventory");
    }
    let residual_slot = input.residual_slot_id.trim();
    let assessment = ProfileProtocolAssessment {
        repository_catalog_sha256,
        repository_seed_count: u32::try_from(seeds.len()).map_err(|_| "catalog_count_overflow")?,
        repository_rule_count: u32::try_from(rules.len()).map_err(|_| "catalog_count_overflow")?,
        target_scoped_seed_count,
        target_scoped_rule_count,
        numeric_basis_count: 0,
        trajectory_coordinate_count: 0,
        occupied_slot_count: u32::try_from(slots.len()).map_err(|_| "slot_count_overflow")?,
        occupied_slot_inventory_sha256: slot_inventory_digest(&slots),
        residual_slot_collision: slots
            .binary_search_by(|candidate| candidate.as_str().cmp(residual_slot))
            .is_ok(),
    };
    let canonical_bytes = encode_assessment(&assessment);
    let receipts = build_receipts(input, &canonical_bytes);
    Ok(ProfileProtocolCheckerOutput {
        assessment,
        canonical_bytes,
        receipts,
    })
}

fn validate_fixed_reverse(input: &ProfileProtocolInput) -> Result<(), &'static str> {
    let fixed = [
        input.derivation_catalog_sha256,
        input.validity_receipt_sha256,
        input.applicability_receipt_sha256,
        input.source_custody_sha256,
        input.profile_input_sha256,
        input.content_identity,
        input.role_identity,
        input.claim_identity,
    ];
    let distinct = fixed.into_iter().collect::<BTreeSet<_>>();
    if distinct.len() != fixed.len()
        || distinct
            .iter()
            .any(|digest| digest.iter().all(|byte| *byte == 0))
        || !valid_canonical_text(&input.residual_slot_id)
        || !valid_canonical_text(&input.owner_admission_record)
    {
        return Err("invalid_profile_protocol_input");
    }
    Ok(())
}

fn invalid_seed(seed: &CatalogSeed) -> bool {
    [
        seed.capability_sha256,
        seed.content_identity,
        seed.role_identity,
        seed.claim_identity,
    ]
    .iter()
    .any(|digest| digest.iter().all(|byte| *byte == 0))
}

fn invalid_rule(rule: &CatalogRule) -> bool {
    [
        rule.capability_sha256,
        rule.conclusion_content_identity,
        rule.conclusion_role_identity,
        rule.claim_identity,
    ]
    .iter()
    .any(|digest| digest.iter().all(|byte| *byte == 0))
}

fn valid_text(value: &str) -> bool {
    (1..=192).contains(&value.len()) && value.is_ascii()
}

fn catalog_digest(seeds: &[CatalogSeed], rules: &[CatalogRule]) -> [u8; 32] {
    let mut bytes = CATALOG_DOMAIN.to_vec();
    field(
        &mut bytes,
        1,
        &u64::try_from(seeds.len())
            .expect("bounded seed inventory")
            .to_be_bytes(),
    );
    for seed in seeds {
        for (tag, value) in [
            (2, seed.claim_identity),
            (3, seed.role_identity),
            (4, seed.content_identity),
            (5, seed.capability_sha256),
        ] {
            field(&mut bytes, tag, &value);
        }
    }
    field(
        &mut bytes,
        6,
        &u64::try_from(rules.len())
            .expect("bounded rule inventory")
            .to_be_bytes(),
    );
    for rule in rules {
        for (tag, value) in [
            (7, rule.claim_identity),
            (8, rule.conclusion_role_identity),
            (9, rule.conclusion_content_identity),
            (10, rule.capability_sha256),
        ] {
            field(&mut bytes, tag, &value);
        }
    }
    sha256(&bytes)
}

fn slot_inventory_digest(slots: &[String]) -> [u8; 32] {
    let mut bytes = SLOT_DOMAIN.to_vec();
    field(
        &mut bytes,
        1,
        &u64::try_from(slots.len())
            .expect("bounded slot inventory")
            .to_be_bytes(),
    );
    for slot in slots {
        field(&mut bytes, 2, slot.as_bytes());
    }
    sha256(&bytes)
}

fn encode_assessment(assessment: &ProfileProtocolAssessment) -> Vec<u8> {
    let mut fields = vec![
        (1, assessment.repository_catalog_sha256.to_vec()),
        (2, assessment.occupied_slot_inventory_sha256.to_vec()),
        (3, assessment.repository_seed_count.to_be_bytes().to_vec()),
        (4, assessment.repository_rule_count.to_be_bytes().to_vec()),
        (
            5,
            assessment.target_scoped_seed_count.to_be_bytes().to_vec(),
        ),
        (
            6,
            assessment.target_scoped_rule_count.to_be_bytes().to_vec(),
        ),
        (7, assessment.numeric_basis_count.to_be_bytes().to_vec()),
        (
            8,
            assessment
                .trajectory_coordinate_count
                .to_be_bytes()
                .to_vec(),
        ),
        (9, assessment.occupied_slot_count.to_be_bytes().to_vec()),
        (10, vec![u8::from(assessment.residual_slot_collision)]),
    ];
    fields.sort_unstable_by_key(|(tag, _)| *tag);
    let mut bytes = ASSESSMENT_SCHEMA.to_vec();
    for (tag, value) in fields {
        field(&mut bytes, tag, &value);
    }
    bytes
}

fn build_receipts(
    input: &ProfileProtocolInput,
    canonical_bytes: &[u8],
) -> ProfileProtocolCheckerReceipts {
    let mut values = Vec::with_capacity(RECEIPT_DOMAINS.len());
    for domain in RECEIPT_DOMAINS {
        let mut fields = vec![
            (9, input.derivation_catalog_sha256.to_vec()),
            (8, input.validity_receipt_sha256.to_vec()),
            (7, input.applicability_receipt_sha256.to_vec()),
            (6, input.source_custody_sha256.to_vec()),
            (5, input.owner_admission_record.trim().as_bytes().to_vec()),
            (4, input.residual_slot_id.trim().as_bytes().to_vec()),
            (3, input.profile_input_sha256.to_vec()),
            (2, input.claim_identity.to_vec()),
            (1, canonical_bytes.to_vec()),
            (10, input.role_identity.to_vec()),
            (11, input.content_identity.to_vec()),
        ];
        fields.sort_unstable_by_key(|(tag, _)| *tag);
        let mut bytes = domain.to_vec();
        for (tag, value) in fields {
            field(&mut bytes, tag, &value);
        }
        values.push(sha256(&bytes));
    }
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

fn field(bytes: &mut Vec<u8>, tag: u16, value: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(value.len())
            .expect("bounded profile protocol field")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(value);
}

fn valid_canonical_text(value: &str) -> bool {
    valid_text(value) && value == value.trim()
}
