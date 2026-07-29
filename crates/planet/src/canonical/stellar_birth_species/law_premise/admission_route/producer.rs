//! Forward fixed-point derive-first route producer.

use super::{
    valid_key, BuckinghamPiDisposition, ChaosDisposition, DerivationCoverageCapability,
    DerivationWitness, IrreducibleProtocolCapability, OpenDerivationFrontier,
    PremiseAdmissionCheckerOutput, PremiseAdmissionDecision, PremiseAdmissionEvaluation,
    PremiseAdmissionRefusal, PremiseAdmissionRouteInput, PremiseKey, VerifiedPremiseDerivationRule,
    MAX_CANONICAL_BYTES, MAX_RULE_COUNT, MAX_RULE_PREMISES, MAX_SEED_COUNT,
    MAX_TOTAL_RULE_PREMISES, MAX_WORK_UNITS, RESULT_SCHEMA_ID,
};
use crate::canonical::stellar_birth_species::law_premise::CandidateProof;
use civsim_units::digest::sha256;
use std::collections::{BTreeMap, BTreeSet};

const RULE_CAPABILITY_DOMAIN: &[u8] =
    b"civsim.planet.law-premise-route.derivation-rule-capability.v1";
const COVERAGE_CAPABILITY_DOMAIN: &[u8] =
    b"civsim.planet.law-premise-route.derivation-coverage-capability.v1";
const IRREDUCIBLE_CAPABILITY_DOMAIN: &[u8] =
    b"civsim.planet.law-premise-route.irreducible-protocol-capability.v1";
const CATALOG_DOMAIN: &[u8] = b"civsim.planet.law-premise-route.catalog.v1";

pub(super) fn route(
    input: &PremiseAdmissionRouteInput,
) -> Result<PremiseAdmissionCheckerOutput, PremiseAdmissionRefusal> {
    validate_envelope(input)?;
    let all_seeds = validate_seeds(input)?;
    let all_rules = validate_rules(input)?;
    let (seeds, rules) = target_scoped_catalog(input.target, &all_seeds, &all_rules);
    validate_scoped_work(
        rules.len(),
        rules.iter().map(|rule| rule.premises.len()).sum(),
    )?;
    let catalog_sha256 = catalog_digest(input, &seeds, &rules)?;
    validate_optional_capabilities(input, catalog_sha256)?;

    let levels = forward_derivation_levels(&seeds, &rules)?;
    let reachable = levels.keys().copied().collect::<BTreeSet<_>>();

    let decision = if reachable.contains(&input.target) {
        if input.irreducible_protocol.is_some() {
            return Err(PremiseAdmissionRefusal::IrreducibleProtocolCapabilityInvalid);
        }
        PremiseAdmissionDecision::Derived(build_witness(input.target, &seeds, &rules, &levels)?)
    } else {
        match input.derivation_coverage {
            None => PremiseAdmissionDecision::OpenDerivationFrontier(OpenDerivationFrontier {
                target: input.target,
                reachable: reachable.iter().copied().collect(),
                unresolved_dependencies: unresolved_dependencies(&rules, &reachable),
                derivation_catalog_sha256: catalog_sha256,
            }),
            Some(coverage) => match input.irreducible_protocol {
                None => PremiseAdmissionDecision::IrreducibleProtocolRequired {
                    target: input.target,
                    derivation_coverage_capability_sha256: coverage.capability_sha256,
                },
                Some(protocol) => PremiseAdmissionDecision::IrreducibleProtocolStructurallyBound {
                    target: input.target,
                    derivation_coverage_capability_sha256: coverage.capability_sha256,
                    irreducible_protocol_capability_sha256: protocol.capability_sha256,
                },
            },
        }
    };

    let evaluation = PremiseAdmissionEvaluation {
        claim_identity: input.claim_identity,
        target: input.target,
        derivation_catalog_sha256: catalog_sha256,
        seed_count: u32::try_from(seeds.len())
            .map_err(|_| PremiseAdmissionRefusal::SeedCapacityExceeded)?,
        rule_count: u32::try_from(rules.len())
            .map_err(|_| PremiseAdmissionRefusal::RuleCapacityExceeded)?,
        decision,
    };
    let canonical_bytes = encode_evaluation(&evaluation)?;
    Ok(PremiseAdmissionCheckerOutput {
        evaluation,
        canonical_bytes,
    })
}

fn validate_envelope(input: &PremiseAdmissionRouteInput) -> Result<(), PremiseAdmissionRefusal> {
    if input.claim_identity.0 == [0; 32] {
        return Err(PremiseAdmissionRefusal::InvalidClaimIdentity);
    }
    if !valid_key(input.target) {
        return Err(PremiseAdmissionRefusal::InvalidTarget);
    }
    if input.admitted_seeds.len() > MAX_SEED_COUNT {
        return Err(PremiseAdmissionRefusal::SeedCapacityExceeded);
    }
    if input.admitted_rules.len() > MAX_RULE_COUNT {
        return Err(PremiseAdmissionRefusal::RuleCapacityExceeded);
    }
    if input.irreducible_protocol.is_some() && input.derivation_coverage.is_none() {
        return Err(PremiseAdmissionRefusal::IrreducibleProtocolWithoutCoverage);
    }
    Ok(())
}

fn validate_seeds(
    input: &PremiseAdmissionRouteInput,
) -> Result<BTreeMap<PremiseKey, [u8; 32]>, PremiseAdmissionRefusal> {
    let mut seeds = BTreeMap::new();
    for seed in &input.admitted_seeds {
        if seed.claim_identity != input.claim_identity
            || !valid_key(seed.key)
            || seed.capability_sha256 == [0; 32]
            || super::super::producer::capability_digest(seed) != seed.capability_sha256
            || !seed_receipts_are_distinct(seed)
        {
            return Err(PremiseAdmissionRefusal::InvalidSeed);
        }
        if seeds.insert(seed.key, seed.capability_sha256).is_some() {
            return Err(PremiseAdmissionRefusal::DuplicateSeed);
        }
    }
    Ok(seeds)
}

fn seed_receipts_are_distinct(seed: &super::super::ClaimScopedPremiseCapability) -> bool {
    let mut receipts = vec![
        seed.upstream_capability_sha256,
        seed.semantic_producer_receipt_sha256,
        seed.semantic_watchdog_receipt_sha256,
        seed.applicability_receipt_sha256,
        seed.validity_receipt_sha256,
    ];
    if let CandidateProof::ExactZero(proof) = seed.proof {
        receipts.push(proof.exclusion_receipt_sha256);
    }
    receipts.iter().all(|receipt| *receipt != [0; 32])
        && !receipts
            .iter()
            .enumerate()
            .any(|(index, receipt)| receipts[index + 1..].contains(receipt))
}

fn validate_rules(
    input: &PremiseAdmissionRouteInput,
) -> Result<Vec<VerifiedPremiseDerivationRule>, PremiseAdmissionRefusal> {
    let total_premises = input
        .admitted_rules
        .iter()
        .try_fold(0_usize, |total, rule| {
            if rule.premises.len() > MAX_RULE_PREMISES {
                return Err(PremiseAdmissionRefusal::RulePremiseCapacityExceeded);
            }
            total
                .checked_add(rule.premises.len())
                .ok_or(PremiseAdmissionRefusal::TotalRulePremiseCapacityExceeded)
        })?;
    if total_premises > MAX_TOTAL_RULE_PREMISES {
        return Err(PremiseAdmissionRefusal::TotalRulePremiseCapacityExceeded);
    }
    let mut seen_rules = BTreeSet::new();
    let mut rules = Vec::with_capacity(input.admitted_rules.len());
    for source in &input.admitted_rules {
        let mut rule = source.clone();
        rule.premises.sort_unstable();
        if rule.claim_identity != input.claim_identity
            || rule.rule_identity.0 == [0; 32]
            || rule.premises.is_empty()
            || !valid_key(rule.conclusion)
            || rule.premises.iter().any(|premise| !valid_key(*premise))
            || rule.premises.windows(2).any(|pair| pair[0] == pair[1])
            || !rule_receipts_are_distinct(&rule)
        {
            return Err(PremiseAdmissionRefusal::InvalidRule);
        }
        if rule_capability_digest(&rule) != rule.capability_sha256 {
            return Err(PremiseAdmissionRefusal::RuleCapabilityDigestMismatch);
        }
        if !seen_rules.insert(rule.rule_identity) {
            return Err(PremiseAdmissionRefusal::DuplicateRule);
        }
        rules.push(rule);
    }
    rules
        .sort_unstable_by_key(|rule| (rule.conclusion, rule.rule_identity, rule.capability_sha256));
    Ok(rules)
}

fn target_scoped_catalog(
    target: PremiseKey,
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
) -> (
    BTreeMap<PremiseKey, [u8; 32]>,
    Vec<VerifiedPremiseDerivationRule>,
) {
    let mut rules_by_conclusion = BTreeMap::<PremiseKey, Vec<usize>>::new();
    for (index, rule) in rules.iter().enumerate() {
        rules_by_conclusion
            .entry(rule.conclusion)
            .or_default()
            .push(index);
    }
    let mut needed = BTreeSet::new();
    let mut pending = vec![target];
    while let Some(current) = pending.pop() {
        if !needed.insert(current) {
            continue;
        }
        if let Some(indices) = rules_by_conclusion.get(&current) {
            for index in indices.iter().rev() {
                pending.extend(rules[*index].premises.iter().rev().copied());
            }
        }
    }
    let scoped_seeds = seeds
        .iter()
        .filter(|(key, _)| needed.contains(key))
        .map(|(key, digest)| (*key, *digest))
        .collect();
    let scoped_rules = rules
        .iter()
        .filter(|rule| needed.contains(&rule.conclusion))
        .cloned()
        .collect();
    (scoped_seeds, scoped_rules)
}

fn validate_scoped_work(
    rule_count: usize,
    total_premises: usize,
) -> Result<(), PremiseAdmissionRefusal> {
    if preflight_work(rule_count, total_premises)? > MAX_WORK_UNITS {
        return Err(PremiseAdmissionRefusal::WorkLimitExceeded);
    }
    Ok(())
}

pub(super) fn preflight_work(
    rule_count: usize,
    total_premises: usize,
) -> Result<usize, PremiseAdmissionRefusal> {
    rule_count
        .checked_add(1)
        .and_then(|passes| {
            total_premises
                .checked_add(rule_count)
                .and_then(|per_pass| passes.checked_mul(per_pass))
        })
        .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)
}

fn rule_receipts_are_distinct(rule: &VerifiedPremiseDerivationRule) -> bool {
    let receipts = [
        rule.semantic_producer_receipt_sha256,
        rule.semantic_watchdog_receipt_sha256,
        rule.applicability_receipt_sha256,
        rule.validity_receipt_sha256,
    ];
    receipts.iter().all(|receipt| *receipt != [0; 32])
        && !receipts
            .iter()
            .enumerate()
            .any(|(index, receipt)| receipts[index + 1..].contains(receipt))
}

fn validate_optional_capabilities(
    input: &PremiseAdmissionRouteInput,
    catalog_sha256: [u8; 32],
) -> Result<(), PremiseAdmissionRefusal> {
    if let Some(coverage) = input.derivation_coverage {
        if coverage.claim_identity != input.claim_identity
            || coverage.target != input.target
            || coverage.derivation_catalog_sha256 != catalog_sha256
            || coverage.producer_receipt_sha256 == [0; 32]
            || coverage.watchdog_receipt_sha256 == [0; 32]
            || coverage.producer_receipt_sha256 == coverage.watchdog_receipt_sha256
            || coverage_capability_digest(&coverage) != coverage.capability_sha256
        {
            return Err(PremiseAdmissionRefusal::DerivationCoverageCapabilityInvalid);
        }
    }
    if let Some(protocol) = input.irreducible_protocol {
        let coverage = input
            .derivation_coverage
            .ok_or(PremiseAdmissionRefusal::IrreducibleProtocolWithoutCoverage)?;
        if protocol.claim_identity != input.claim_identity
            || protocol.target != input.target
            || protocol.derivation_coverage_capability_sha256 != coverage.capability_sha256
            || protocol.residual_slot_identity == [0; 32]
            || !valid_buckingham_pi(protocol.buckingham_pi)
            || !valid_chaos(protocol.chaos)
            || protocol_capability_digest(&protocol) != protocol.capability_sha256
        {
            return Err(PremiseAdmissionRefusal::IrreducibleProtocolCapabilityInvalid);
        }
        let receipts = protocol_receipts(protocol);
        if receipts.contains(&[0; 32])
            || receipts
                .iter()
                .enumerate()
                .any(|(index, receipt)| receipts[index + 1..].contains(receipt))
            || receipts.contains(&coverage.producer_receipt_sha256)
            || receipts.contains(&coverage.watchdog_receipt_sha256)
        {
            return Err(PremiseAdmissionRefusal::DuplicateProtocolReceipt);
        }
    }
    Ok(())
}

fn valid_buckingham_pi(disposition: BuckinghamPiDisposition) -> bool {
    match disposition {
        BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
            residual_group_count,
        } => {
            variable_basis_receipt_sha256 != [0; 32]
                && producer_receipt_sha256 != [0; 32]
                && watchdog_receipt_sha256 != [0; 32]
                && producer_receipt_sha256 != watchdog_receipt_sha256
                && residual_group_count > 0
        }
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => {
            producer_receipt_sha256 != [0; 32]
                && watchdog_receipt_sha256 != [0; 32]
                && producer_receipt_sha256 != watchdog_receipt_sha256
        }
    }
}

fn valid_chaos(disposition: ChaosDisposition) -> bool {
    match disposition {
        ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        } => {
            producer_inapplicability_receipt_sha256 != [0; 32]
                && watchdog_inapplicability_receipt_sha256 != [0; 32]
                && producer_inapplicability_receipt_sha256
                    != watchdog_inapplicability_receipt_sha256
        }
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => {
            regime_partition_producer_receipt_sha256 != [0; 32]
                && regime_partition_watchdog_receipt_sha256 != [0; 32]
                && transition_law_receipt_sha256 != [0; 32]
                && regime_partition_producer_receipt_sha256
                    != regime_partition_watchdog_receipt_sha256
        }
    }
}

fn protocol_receipts(protocol: IrreducibleProtocolCapability) -> Vec<[u8; 32]> {
    let mut receipts = Vec::with_capacity(16);
    match protocol.buckingham_pi {
        BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
            ..
        } => receipts.extend([
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        ]),
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => receipts.extend([producer_receipt_sha256, watchdog_receipt_sha256]),
    }
    receipts.push(protocol.gap_law_receipt_sha256);
    match protocol.chaos {
        ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        } => receipts.extend([
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        ]),
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => receipts.extend([
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        ]),
    }
    receipts.extend([
        protocol.residual_law_producer_receipt_sha256,
        protocol.residual_law_watchdog_receipt_sha256,
        protocol.unique_slot_producer_receipt_sha256,
        protocol.unique_slot_watchdog_receipt_sha256,
        protocol.owner_admission_receipt_sha256,
    ]);
    receipts
}

fn unresolved_dependencies(
    rules: &[VerifiedPremiseDerivationRule],
    reachable: &BTreeSet<PremiseKey>,
) -> Vec<PremiseKey> {
    rules
        .iter()
        .filter(|rule| !reachable.contains(&rule.conclusion))
        .flat_map(|rule| rule.premises.iter().copied())
        .filter(|premise| !reachable.contains(premise))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn forward_derivation_levels(
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
) -> Result<BTreeMap<PremiseKey, u32>, PremiseAdmissionRefusal> {
    let mut levels = seeds
        .keys()
        .copied()
        .map(|key| (key, 0_u32))
        .collect::<BTreeMap<_, _>>();
    let mut work = 0_usize;
    loop {
        let mut additions = BTreeMap::new();
        for rule in rules {
            work = work
                .checked_add(rule.premises.len().saturating_add(1))
                .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)?;
            if work > MAX_WORK_UNITS {
                return Err(PremiseAdmissionRefusal::WorkLimitExceeded);
            }
            if levels.contains_key(&rule.conclusion) {
                continue;
            }
            let Some(maximum_premise_level) = rule
                .premises
                .iter()
                .map(|premise| levels.get(premise).copied())
                .collect::<Option<Vec<_>>>()
                .and_then(|premise_levels| premise_levels.into_iter().max())
            else {
                continue;
            };
            let conclusion_level = maximum_premise_level
                .checked_add(1)
                .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)?;
            additions
                .entry(rule.conclusion)
                .and_modify(|level: &mut u32| *level = (*level).min(conclusion_level))
                .or_insert(conclusion_level);
        }
        if additions.is_empty() {
            break;
        }
        levels.extend(additions);
    }
    Ok(levels)
}

fn build_witness(
    target: PremiseKey,
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
    levels: &BTreeMap<PremiseKey, u32>,
) -> Result<DerivationWitness, PremiseAdmissionRefusal> {
    let mut seed_capabilities = BTreeSet::new();
    let mut rule_capabilities = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    trace_witness(
        target,
        seeds,
        rules,
        levels,
        &mut visiting,
        &mut seed_capabilities,
        &mut rule_capabilities,
    )?;
    Ok(DerivationWitness {
        target,
        seed_capability_sha256: seed_capabilities.into_iter().collect(),
        rule_capability_sha256: rule_capabilities.into_iter().collect(),
    })
}

fn trace_witness(
    target: PremiseKey,
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
    levels: &BTreeMap<PremiseKey, u32>,
    visiting: &mut BTreeSet<PremiseKey>,
    seed_capabilities: &mut BTreeSet<[u8; 32]>,
    rule_capabilities: &mut BTreeSet<[u8; 32]>,
) -> Result<(), PremiseAdmissionRefusal> {
    if let Some(capability) = seeds.get(&target) {
        seed_capabilities.insert(*capability);
        return Ok(());
    }
    if !visiting.insert(target) {
        return Err(PremiseAdmissionRefusal::CheckerDisagreement);
    }
    let target_level = levels
        .get(&target)
        .copied()
        .ok_or(PremiseAdmissionRefusal::CheckerDisagreement)?;
    let rule = rules
        .iter()
        .find(|rule| {
            rule.conclusion == target
                && rule
                    .premises
                    .iter()
                    .map(|premise| levels.get(premise).copied())
                    .collect::<Option<Vec<_>>>()
                    .and_then(|premise_levels| premise_levels.into_iter().max())
                    .and_then(|maximum| maximum.checked_add(1))
                    == Some(target_level)
        })
        .ok_or(PremiseAdmissionRefusal::CheckerDisagreement)?;
    rule_capabilities.insert(rule.capability_sha256);
    for premise in &rule.premises {
        trace_witness(
            *premise,
            seeds,
            rules,
            levels,
            visiting,
            seed_capabilities,
            rule_capabilities,
        )?;
    }
    visiting.remove(&target);
    Ok(())
}

pub(super) fn rule_capability_digest(rule: &VerifiedPremiseDerivationRule) -> [u8; 32] {
    let mut bytes = RULE_CAPABILITY_DOMAIN.to_vec();
    field(&mut bytes, 1, &rule.claim_identity.0);
    field(&mut bytes, 2, &rule.rule_identity.0);
    let mut premises = rule.premises.clone();
    premises.sort_unstable();
    field(&mut bytes, 3, &encode_keys_unbounded(&premises));
    field(&mut bytes, 4, &rule.conclusion.role.0);
    field(&mut bytes, 5, &rule.conclusion.content.0);
    field(&mut bytes, 6, &rule.semantic_producer_receipt_sha256);
    field(&mut bytes, 7, &rule.semantic_watchdog_receipt_sha256);
    field(&mut bytes, 8, &rule.applicability_receipt_sha256);
    field(&mut bytes, 9, &rule.validity_receipt_sha256);
    sha256(&bytes)
}

pub(super) fn coverage_capability_digest(coverage: &DerivationCoverageCapability) -> [u8; 32] {
    let mut bytes = COVERAGE_CAPABILITY_DOMAIN.to_vec();
    field(&mut bytes, 1, &coverage.claim_identity.0);
    field(&mut bytes, 2, &coverage.target.role.0);
    field(&mut bytes, 3, &coverage.target.content.0);
    field(&mut bytes, 4, &coverage.derivation_catalog_sha256);
    field(&mut bytes, 5, &coverage.producer_receipt_sha256);
    field(&mut bytes, 6, &coverage.watchdog_receipt_sha256);
    sha256(&bytes)
}

pub(super) fn protocol_capability_digest(protocol: &IrreducibleProtocolCapability) -> [u8; 32] {
    let mut bytes = IRREDUCIBLE_CAPABILITY_DOMAIN.to_vec();
    field(&mut bytes, 1, &protocol.claim_identity.0);
    field(&mut bytes, 2, &protocol.target.role.0);
    field(&mut bytes, 3, &protocol.target.content.0);
    field(
        &mut bytes,
        4,
        &protocol.derivation_coverage_capability_sha256,
    );
    encode_buckingham_pi(&mut bytes, protocol.buckingham_pi);
    field(&mut bytes, 10, &protocol.gap_law_receipt_sha256);
    encode_chaos(&mut bytes, protocol.chaos);
    field(
        &mut bytes,
        15,
        &protocol.residual_law_producer_receipt_sha256,
    );
    field(
        &mut bytes,
        16,
        &protocol.residual_law_watchdog_receipt_sha256,
    );
    field(&mut bytes, 17, &protocol.residual_slot_identity);
    field(
        &mut bytes,
        18,
        &protocol.unique_slot_producer_receipt_sha256,
    );
    field(
        &mut bytes,
        19,
        &protocol.unique_slot_watchdog_receipt_sha256,
    );
    field(&mut bytes, 20, &protocol.owner_admission_receipt_sha256);
    sha256(&bytes)
}

fn catalog_digest(
    input: &PremiseAdmissionRouteInput,
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
) -> Result<[u8; 32], PremiseAdmissionRefusal> {
    let mut bytes = CATALOG_DOMAIN.to_vec();
    field(&mut bytes, 1, &input.claim_identity.0);
    field(&mut bytes, 2, &input.target.role.0);
    field(&mut bytes, 3, &input.target.content.0);
    let seed_digests = seeds.values().copied().collect::<Vec<_>>();
    field(&mut bytes, 4, &encode_digests_unbounded(&seed_digests));
    let rule_digests = rules
        .iter()
        .map(|rule| rule.capability_sha256)
        .collect::<Vec<_>>();
    field(&mut bytes, 5, &encode_digests_unbounded(&rule_digests));
    if bytes.len() > MAX_CANONICAL_BYTES {
        return Err(PremiseAdmissionRefusal::CanonicalByteCapacityExceeded);
    }
    Ok(sha256(&bytes))
}

fn encode_evaluation(
    evaluation: &PremiseAdmissionEvaluation,
) -> Result<Vec<u8>, PremiseAdmissionRefusal> {
    let mut bytes = Vec::new();
    bounded_field(&mut bytes, 1, RESULT_SCHEMA_ID.as_bytes())?;
    bounded_field(&mut bytes, 2, &evaluation.claim_identity.0)?;
    bounded_field(&mut bytes, 3, &evaluation.target.role.0)?;
    bounded_field(&mut bytes, 4, &evaluation.target.content.0)?;
    bounded_field(&mut bytes, 5, &evaluation.derivation_catalog_sha256)?;
    bounded_field(&mut bytes, 6, &evaluation.seed_count.to_be_bytes())?;
    bounded_field(&mut bytes, 7, &evaluation.rule_count.to_be_bytes())?;
    bounded_field(&mut bytes, 8, evaluation.decision.id().as_bytes())?;
    match &evaluation.decision {
        PremiseAdmissionDecision::Derived(witness) => {
            bounded_field(
                &mut bytes,
                9,
                &encode_digests_unbounded(&witness.seed_capability_sha256),
            )?;
            bounded_field(
                &mut bytes,
                10,
                &encode_digests_unbounded(&witness.rule_capability_sha256),
            )?;
            bounded_field(&mut bytes, 11, &[])?;
            bounded_field(&mut bytes, 12, &[])?;
        }
        PremiseAdmissionDecision::OpenDerivationFrontier(frontier) => {
            bounded_field(&mut bytes, 9, &[])?;
            bounded_field(&mut bytes, 10, &[])?;
            bounded_field(&mut bytes, 11, &encode_keys_unbounded(&frontier.reachable))?;
            bounded_field(
                &mut bytes,
                12,
                &encode_keys_unbounded(&frontier.unresolved_dependencies),
            )?;
        }
        PremiseAdmissionDecision::IrreducibleProtocolRequired {
            derivation_coverage_capability_sha256,
            ..
        } => {
            bounded_field(&mut bytes, 9, derivation_coverage_capability_sha256)?;
            bounded_field(&mut bytes, 10, &[])?;
            bounded_field(&mut bytes, 11, &[])?;
            bounded_field(&mut bytes, 12, &[])?;
        }
        PremiseAdmissionDecision::IrreducibleProtocolStructurallyBound {
            derivation_coverage_capability_sha256,
            irreducible_protocol_capability_sha256,
            ..
        } => {
            bounded_field(&mut bytes, 9, derivation_coverage_capability_sha256)?;
            bounded_field(&mut bytes, 10, irreducible_protocol_capability_sha256)?;
            bounded_field(&mut bytes, 11, &[])?;
            bounded_field(&mut bytes, 12, &[])?;
        }
    }
    Ok(bytes)
}

fn encode_buckingham_pi(bytes: &mut Vec<u8>, disposition: BuckinghamPiDisposition) {
    match disposition {
        BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
            residual_group_count,
        } => {
            field(bytes, 5, &[0]);
            field(bytes, 6, &variable_basis_receipt_sha256);
            field(bytes, 7, &producer_receipt_sha256);
            field(bytes, 8, &watchdog_receipt_sha256);
            field(bytes, 9, &residual_group_count.to_be_bytes());
        }
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => {
            field(bytes, 5, &[1]);
            field(bytes, 6, &[]);
            field(bytes, 7, &producer_receipt_sha256);
            field(bytes, 8, &watchdog_receipt_sha256);
            field(bytes, 9, &0_u32.to_be_bytes());
        }
    }
}

fn encode_chaos(bytes: &mut Vec<u8>, disposition: ChaosDisposition) {
    match disposition {
        ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        } => {
            field(bytes, 11, &[0]);
            field(bytes, 12, &producer_inapplicability_receipt_sha256);
            field(bytes, 13, &watchdog_inapplicability_receipt_sha256);
            field(bytes, 14, &[]);
        }
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => {
            field(bytes, 11, &[1]);
            field(bytes, 12, &regime_partition_producer_receipt_sha256);
            field(bytes, 13, &regime_partition_watchdog_receipt_sha256);
            field(bytes, 14, &transition_law_receipt_sha256);
        }
    }
}

fn encode_keys_unbounded(keys: &[PremiseKey]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + keys.len().saturating_mul(64));
    bytes.extend_from_slice(&u64::try_from(keys.len()).unwrap_or(u64::MAX).to_be_bytes());
    for key in keys {
        bytes.extend_from_slice(&key.role.0);
        bytes.extend_from_slice(&key.content.0);
    }
    bytes
}

fn encode_digests_unbounded(digests: &[[u8; 32]]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + digests.len().saturating_mul(32));
    bytes.extend_from_slice(
        &u64::try_from(digests.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for digest in digests {
        bytes.extend_from_slice(digest);
    }
    bytes
}

fn field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}

fn bounded_field(
    bytes: &mut Vec<u8>,
    tag: u16,
    payload: &[u8],
) -> Result<(), PremiseAdmissionRefusal> {
    let next = bytes
        .len()
        .checked_add(10)
        .and_then(|length| length.checked_add(payload.len()))
        .ok_or(PremiseAdmissionRefusal::CanonicalByteCapacityExceeded)?;
    if next > MAX_CANONICAL_BYTES {
        return Err(PremiseAdmissionRefusal::CanonicalByteCapacityExceeded);
    }
    field(bytes, tag, payload);
    Ok(())
}
