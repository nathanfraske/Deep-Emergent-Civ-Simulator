//! Dependency-indexed derive-first route watchdog.

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
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

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
    let levels = dependency_derivation_levels(&seeds, &rules)?;
    let reachable = levels.keys().copied().collect::<BTreeSet<_>>();

    let decision = if reachable.contains(&input.target) {
        if input.irreducible_protocol.is_some() {
            return Err(PremiseAdmissionRefusal::IrreducibleProtocolCapabilityInvalid);
        }
        PremiseAdmissionDecision::Derived(build_witness(input.target, &seeds, &rules, &levels)?)
    } else if let Some(coverage) = input.derivation_coverage {
        if let Some(protocol) = input.irreducible_protocol {
            PremiseAdmissionDecision::IrreducibleProtocolStructurallyBound {
                target: input.target,
                derivation_coverage_capability_sha256: coverage.capability_sha256,
                irreducible_protocol_capability_sha256: protocol.capability_sha256,
            }
        } else {
            PremiseAdmissionDecision::IrreducibleProtocolRequired {
                target: input.target,
                derivation_coverage_capability_sha256: coverage.capability_sha256,
            }
        }
    } else {
        PremiseAdmissionDecision::OpenDerivationFrontier(OpenDerivationFrontier {
            target: input.target,
            reachable: reachable.iter().copied().collect(),
            unresolved_dependencies: unresolved_dependencies(&rules, &reachable),
            derivation_catalog_sha256: catalog_sha256,
        })
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
    if input.claim_identity.0.iter().all(|byte| *byte == 0) {
        return Err(PremiseAdmissionRefusal::InvalidClaimIdentity);
    }
    if !valid_key(input.target) {
        return Err(PremiseAdmissionRefusal::InvalidTarget);
    }
    match input.admitted_seeds.len().cmp(&MAX_SEED_COUNT) {
        std::cmp::Ordering::Greater => return Err(PremiseAdmissionRefusal::SeedCapacityExceeded),
        std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {}
    }
    if input.admitted_rules.len() > MAX_RULE_COUNT {
        return Err(PremiseAdmissionRefusal::RuleCapacityExceeded);
    }
    if matches!(
        (&input.irreducible_protocol, &input.derivation_coverage),
        (Some(_), None)
    ) {
        return Err(PremiseAdmissionRefusal::IrreducibleProtocolWithoutCoverage);
    }
    Ok(())
}

fn validate_seeds(
    input: &PremiseAdmissionRouteInput,
) -> Result<BTreeMap<PremiseKey, [u8; 32]>, PremiseAdmissionRefusal> {
    let mut seeds = BTreeMap::new();
    for seed in input.admitted_seeds.iter().rev() {
        let valid_claim = seed.claim_identity == input.claim_identity;
        let valid_digest = seed.capability_sha256.iter().any(|byte| *byte != 0)
            && super::super::watchdog::capability_digest(seed) == seed.capability_sha256;
        if !valid_claim
            || !valid_key(seed.key)
            || !valid_digest
            || !seed_receipts_are_distinct(seed)
        {
            return Err(PremiseAdmissionRefusal::InvalidSeed);
        }
        if seeds.contains_key(&seed.key) {
            return Err(PremiseAdmissionRefusal::DuplicateSeed);
        }
        seeds.insert(seed.key, seed.capability_sha256);
    }
    Ok(seeds)
}

fn seed_receipts_are_distinct(seed: &super::super::ClaimScopedPremiseCapability) -> bool {
    let mut receipts = BTreeSet::new();
    for receipt in [
        seed.upstream_capability_sha256,
        seed.semantic_producer_receipt_sha256,
        seed.semantic_watchdog_receipt_sha256,
        seed.applicability_receipt_sha256,
        seed.validity_receipt_sha256,
    ] {
        if receipt.iter().all(|byte| *byte == 0) || !receipts.insert(receipt) {
            return false;
        }
    }
    match seed.proof {
        CandidateProof::AdmittedContent | CandidateProof::VerifiedDerivedContent => true,
        CandidateProof::ExactZero(proof) => {
            proof.exclusion_receipt_sha256.iter().any(|byte| *byte != 0)
                && receipts.insert(proof.exclusion_receipt_sha256)
        }
    }
}

fn validate_rules(
    input: &PremiseAdmissionRouteInput,
) -> Result<Vec<VerifiedPremiseDerivationRule>, PremiseAdmissionRefusal> {
    let mut total_premises = 0_usize;
    for rule in &input.admitted_rules {
        if rule.premises.len() > MAX_RULE_PREMISES {
            return Err(PremiseAdmissionRefusal::RulePremiseCapacityExceeded);
        }
        total_premises = total_premises
            .checked_add(rule.premises.len())
            .ok_or(PremiseAdmissionRefusal::TotalRulePremiseCapacityExceeded)?;
        if total_premises > MAX_TOTAL_RULE_PREMISES {
            return Err(PremiseAdmissionRefusal::TotalRulePremiseCapacityExceeded);
        }
    }
    let mut identities = BTreeSet::new();
    let mut rules = Vec::with_capacity(input.admitted_rules.len());
    for source in input.admitted_rules.iter().rev() {
        let mut rule = source.clone();
        let premise_set = rule.premises.iter().copied().collect::<BTreeSet<_>>();
        rule.premises = premise_set.iter().copied().collect();
        let receipts = [
            rule.semantic_producer_receipt_sha256,
            rule.semantic_watchdog_receipt_sha256,
            rule.applicability_receipt_sha256,
            rule.validity_receipt_sha256,
        ];
        let receipt_set = receipts.into_iter().collect::<BTreeSet<_>>();
        if rule.claim_identity != input.claim_identity
            || rule.rule_identity.0.iter().all(|byte| *byte == 0)
            || rule.premises.is_empty()
            || rule.premises.len() != source.premises.len()
            || !valid_key(rule.conclusion)
            || rule.premises.iter().any(|premise| !valid_key(*premise))
            || receipt_set.len() != receipts.len()
            || receipt_set
                .iter()
                .any(|receipt| receipt.iter().all(|byte| *byte == 0))
        {
            return Err(PremiseAdmissionRefusal::InvalidRule);
        }
        if rule_capability_digest(&rule) != rule.capability_sha256 {
            return Err(PremiseAdmissionRefusal::RuleCapabilityDigestMismatch);
        }
        if !identities.insert(rule.rule_identity) {
            return Err(PremiseAdmissionRefusal::DuplicateRule);
        }
        rules.push(rule);
    }
    rules.sort_unstable_by(|left, right| {
        left.conclusion
            .cmp(&right.conclusion)
            .then(left.rule_identity.cmp(&right.rule_identity))
            .then(left.capability_sha256.cmp(&right.capability_sha256))
    });
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
    let ordered_conclusions = rules
        .iter()
        .enumerate()
        .map(|(index, rule)| (rule.conclusion, index))
        .collect::<Vec<_>>();
    let mut needed = BTreeSet::new();
    let mut pending = std::collections::VecDeque::from([target]);
    while let Some(current) = pending.pop_front() {
        if !needed.insert(current) {
            continue;
        }
        let first = ordered_conclusions.partition_point(|(conclusion, _)| *conclusion < current);
        let last = ordered_conclusions.partition_point(|(conclusion, _)| *conclusion <= current);
        for (_, index) in &ordered_conclusions[first..last] {
            for premise in &rules[*index].premises {
                pending.push_back(*premise);
            }
        }
    }
    let scoped_seeds = seeds
        .range(..)
        .filter(|(key, _)| needed.contains(key))
        .map(|(key, digest)| (*key, *digest))
        .collect();
    let scoped_rules = rules
        .iter()
        .filter_map(|rule| needed.contains(&rule.conclusion).then_some(rule.clone()))
        .collect();
    (scoped_seeds, scoped_rules)
}

fn validate_scoped_work(
    rule_count: usize,
    total_premises: usize,
) -> Result<(), PremiseAdmissionRefusal> {
    match preflight_work(rule_count, total_premises)? {
        work if work > MAX_WORK_UNITS => Err(PremiseAdmissionRefusal::WorkLimitExceeded),
        _ => Ok(()),
    }
}

pub(super) fn preflight_work(
    rule_count: usize,
    total_premises: usize,
) -> Result<usize, PremiseAdmissionRefusal> {
    let passes = rule_count
        .checked_add(1)
        .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)?;
    let per_pass = total_premises
        .checked_add(rule_count)
        .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)?;
    passes
        .checked_mul(per_pass)
        .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)
}

fn validate_optional_capabilities(
    input: &PremiseAdmissionRouteInput,
    catalog_sha256: [u8; 32],
) -> Result<(), PremiseAdmissionRefusal> {
    if let Some(coverage) = input.derivation_coverage {
        let receipts_are_distinct = coverage
            .producer_receipt_sha256
            .iter()
            .any(|byte| *byte != 0)
            && coverage
                .watchdog_receipt_sha256
                .iter()
                .any(|byte| *byte != 0)
            && coverage.producer_receipt_sha256 != coverage.watchdog_receipt_sha256;
        if coverage.claim_identity != input.claim_identity
            || coverage.target != input.target
            || coverage.derivation_catalog_sha256 != catalog_sha256
            || !receipts_are_distinct
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
            || protocol
                .residual_slot_identity
                .iter()
                .all(|byte| *byte == 0)
            || !valid_buckingham_pi(protocol.buckingham_pi)
            || !valid_chaos(protocol.chaos)
            || protocol_capability_digest(&protocol) != protocol.capability_sha256
        {
            return Err(PremiseAdmissionRefusal::IrreducibleProtocolCapabilityInvalid);
        }
        let receipts = protocol_receipts(protocol);
        let receipt_set = receipts.iter().copied().collect::<BTreeSet<_>>();
        if receipts
            .iter()
            .any(|receipt| receipt.iter().all(|byte| *byte == 0))
            || receipt_set.len() != receipts.len()
            || receipt_set.contains(&coverage.producer_receipt_sha256)
            || receipt_set.contains(&coverage.watchdog_receipt_sha256)
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
            [
                variable_basis_receipt_sha256,
                producer_receipt_sha256,
                watchdog_receipt_sha256,
            ]
            .iter()
            .all(|receipt| receipt.iter().any(|byte| *byte != 0))
                && producer_receipt_sha256 != watchdog_receipt_sha256
                && residual_group_count != 0
        }
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => {
            producer_receipt_sha256.iter().any(|byte| *byte != 0)
                && watchdog_receipt_sha256.iter().any(|byte| *byte != 0)
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
            producer_inapplicability_receipt_sha256
                .iter()
                .any(|byte| *byte != 0)
                && watchdog_inapplicability_receipt_sha256
                    .iter()
                    .any(|byte| *byte != 0)
                && producer_inapplicability_receipt_sha256
                    != watchdog_inapplicability_receipt_sha256
        }
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => {
            regime_partition_producer_receipt_sha256
                .iter()
                .any(|byte| *byte != 0)
                && regime_partition_watchdog_receipt_sha256
                    .iter()
                    .any(|byte| *byte != 0)
                && transition_law_receipt_sha256.iter().any(|byte| *byte != 0)
                && regime_partition_producer_receipt_sha256
                    != regime_partition_watchdog_receipt_sha256
        }
    }
}

fn protocol_receipts(protocol: IrreducibleProtocolCapability) -> Vec<[u8; 32]> {
    let mut receipts = Vec::new();
    match protocol.buckingham_pi {
        BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
            ..
        } => {
            receipts.push(variable_basis_receipt_sha256);
            receipts.push(producer_receipt_sha256);
            receipts.push(watchdog_receipt_sha256);
        }
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => {
            receipts.push(producer_receipt_sha256);
            receipts.push(watchdog_receipt_sha256);
        }
    }
    receipts.push(protocol.gap_law_receipt_sha256);
    match protocol.chaos {
        ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        } => {
            receipts.push(producer_inapplicability_receipt_sha256);
            receipts.push(watchdog_inapplicability_receipt_sha256);
        }
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => {
            receipts.push(regime_partition_producer_receipt_sha256);
            receipts.push(regime_partition_watchdog_receipt_sha256);
            receipts.push(transition_law_receipt_sha256);
        }
    }
    receipts.push(protocol.residual_law_producer_receipt_sha256);
    receipts.push(protocol.residual_law_watchdog_receipt_sha256);
    receipts.push(protocol.unique_slot_producer_receipt_sha256);
    receipts.push(protocol.unique_slot_watchdog_receipt_sha256);
    receipts.push(protocol.owner_admission_receipt_sha256);
    receipts
}

fn dependency_derivation_levels(
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
) -> Result<BTreeMap<PremiseKey, u32>, PremiseAdmissionRefusal> {
    let mut levels = BTreeMap::new();
    let mut dependents: BTreeMap<PremiseKey, Vec<usize>> = BTreeMap::new();
    let mut unresolved = rules
        .iter()
        .map(|rule| rule.premises.len())
        .collect::<Vec<_>>();
    let mut maximum_premise_level = vec![0_u32; rules.len()];
    for (index, rule) in rules.iter().enumerate() {
        for premise in &rule.premises {
            dependents.entry(*premise).or_default().push(index);
        }
    }

    let mut ready = seeds
        .keys()
        .copied()
        .map(|key| Reverse((0_u32, key)))
        .collect::<BinaryHeap<_>>();
    let mut work = rules.iter().try_fold(0_usize, |total, rule| {
        total
            .checked_add(rule.premises.len().saturating_add(1))
            .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)
    })?;
    if work > MAX_WORK_UNITS {
        return Err(PremiseAdmissionRefusal::WorkLimitExceeded);
    }
    while let Some(Reverse((level, key))) = ready.pop() {
        work = work
            .checked_add(1)
            .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)?;
        if work > MAX_WORK_UNITS {
            return Err(PremiseAdmissionRefusal::WorkLimitExceeded);
        }
        if levels.contains_key(&key) {
            continue;
        }
        levels.insert(key, level);
        if let Some(waiting) = dependents.get(&key) {
            for dependent in waiting {
                work = work
                    .checked_add(1)
                    .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)?;
                if work > MAX_WORK_UNITS {
                    return Err(PremiseAdmissionRefusal::WorkLimitExceeded);
                }
                if unresolved[*dependent] > 0 {
                    unresolved[*dependent] -= 1;
                    maximum_premise_level[*dependent] =
                        maximum_premise_level[*dependent].max(level);
                    if unresolved[*dependent] == 0 {
                        let conclusion_level = maximum_premise_level[*dependent]
                            .checked_add(1)
                            .ok_or(PremiseAdmissionRefusal::WorkLimitExceeded)?;
                        ready.push(Reverse((conclusion_level, rules[*dependent].conclusion)));
                    }
                }
            }
        }
    }
    Ok(levels)
}

fn unresolved_dependencies(
    rules: &[VerifiedPremiseDerivationRule],
    reachable: &BTreeSet<PremiseKey>,
) -> Vec<PremiseKey> {
    let mut unresolved = BTreeSet::new();
    for rule in rules {
        if reachable.contains(&rule.conclusion) {
            continue;
        }
        for premise in &rule.premises {
            if !reachable.contains(premise) {
                unresolved.insert(*premise);
            }
        }
    }
    unresolved.into_iter().collect()
}

fn build_witness(
    target: PremiseKey,
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
    levels: &BTreeMap<PremiseKey, u32>,
) -> Result<DerivationWitness, PremiseAdmissionRefusal> {
    let mut pending = vec![target];
    let mut visited = BTreeSet::new();
    let mut seed_capabilities = BTreeSet::new();
    let mut rule_capabilities = BTreeSet::new();
    while let Some(current) = pending.pop() {
        if !visited.insert(current) {
            continue;
        }
        if let Some(capability) = seeds.get(&current) {
            seed_capabilities.insert(*capability);
            continue;
        }
        let current_level = levels
            .get(&current)
            .copied()
            .ok_or(PremiseAdmissionRefusal::CheckerDisagreement)?;
        let rule = rules
            .iter()
            .find(|rule| {
                rule.conclusion == current
                    && rule
                        .premises
                        .iter()
                        .map(|premise| levels.get(premise).copied())
                        .collect::<Option<Vec<_>>>()
                        .and_then(|premise_levels| premise_levels.into_iter().max())
                        .and_then(|maximum| maximum.checked_add(1))
                        == Some(current_level)
            })
            .ok_or(PremiseAdmissionRefusal::CheckerDisagreement)?;
        rule_capabilities.insert(rule.capability_sha256);
        pending.extend(rule.premises.iter().rev().copied());
    }
    Ok(DerivationWitness {
        target,
        seed_capability_sha256: seed_capabilities.into_iter().collect(),
        rule_capability_sha256: rule_capabilities.into_iter().collect(),
    })
}

pub(super) fn rule_capability_digest(rule: &VerifiedPremiseDerivationRule) -> [u8; 32] {
    let mut fields = BTreeMap::new();
    fields.insert(1_u16, rule.claim_identity.0.to_vec());
    fields.insert(2, rule.rule_identity.0.to_vec());
    let premises = rule.premises.iter().copied().collect::<BTreeSet<_>>();
    fields.insert(3, encode_keys(&premises.into_iter().collect::<Vec<_>>()));
    fields.insert(4, rule.conclusion.role.0.to_vec());
    fields.insert(5, rule.conclusion.content.0.to_vec());
    fields.insert(6, rule.semantic_producer_receipt_sha256.to_vec());
    fields.insert(7, rule.semantic_watchdog_receipt_sha256.to_vec());
    fields.insert(8, rule.applicability_receipt_sha256.to_vec());
    fields.insert(9, rule.validity_receipt_sha256.to_vec());
    sha256(&encode_fields(RULE_CAPABILITY_DOMAIN, fields))
}

pub(super) fn coverage_capability_digest(coverage: &DerivationCoverageCapability) -> [u8; 32] {
    let fields = BTreeMap::from([
        (1_u16, coverage.claim_identity.0.to_vec()),
        (2, coverage.target.role.0.to_vec()),
        (3, coverage.target.content.0.to_vec()),
        (4, coverage.derivation_catalog_sha256.to_vec()),
        (5, coverage.producer_receipt_sha256.to_vec()),
        (6, coverage.watchdog_receipt_sha256.to_vec()),
    ]);
    sha256(&encode_fields(COVERAGE_CAPABILITY_DOMAIN, fields))
}

pub(super) fn protocol_capability_digest(protocol: &IrreducibleProtocolCapability) -> [u8; 32] {
    let mut fields = BTreeMap::from([
        (1_u16, protocol.claim_identity.0.to_vec()),
        (2, protocol.target.role.0.to_vec()),
        (3, protocol.target.content.0.to_vec()),
        (4, protocol.derivation_coverage_capability_sha256.to_vec()),
        (10, protocol.gap_law_receipt_sha256.to_vec()),
        (15, protocol.residual_law_producer_receipt_sha256.to_vec()),
        (16, protocol.residual_law_watchdog_receipt_sha256.to_vec()),
        (17, protocol.residual_slot_identity.to_vec()),
        (18, protocol.unique_slot_producer_receipt_sha256.to_vec()),
        (19, protocol.unique_slot_watchdog_receipt_sha256.to_vec()),
        (20, protocol.owner_admission_receipt_sha256.to_vec()),
    ]);
    insert_buckingham_pi(&mut fields, protocol.buckingham_pi);
    insert_chaos(&mut fields, protocol.chaos);
    sha256(&encode_fields(IRREDUCIBLE_CAPABILITY_DOMAIN, fields))
}

fn catalog_digest(
    input: &PremiseAdmissionRouteInput,
    seeds: &BTreeMap<PremiseKey, [u8; 32]>,
    rules: &[VerifiedPremiseDerivationRule],
) -> Result<[u8; 32], PremiseAdmissionRefusal> {
    let fields = BTreeMap::from([
        (1_u16, input.claim_identity.0.to_vec()),
        (2, input.target.role.0.to_vec()),
        (3, input.target.content.0.to_vec()),
        (
            4,
            encode_digests(&seeds.values().copied().collect::<Vec<_>>()),
        ),
        (
            5,
            encode_digests(
                &rules
                    .iter()
                    .map(|rule| rule.capability_sha256)
                    .collect::<Vec<_>>(),
            ),
        ),
    ]);
    let bytes = encode_fields(CATALOG_DOMAIN, fields);
    if bytes.len() > MAX_CANONICAL_BYTES {
        return Err(PremiseAdmissionRefusal::CanonicalByteCapacityExceeded);
    }
    Ok(sha256(&bytes))
}

fn encode_evaluation(
    evaluation: &PremiseAdmissionEvaluation,
) -> Result<Vec<u8>, PremiseAdmissionRefusal> {
    let mut fields = BTreeMap::from([
        (1_u16, RESULT_SCHEMA_ID.as_bytes().to_vec()),
        (2, evaluation.claim_identity.0.to_vec()),
        (3, evaluation.target.role.0.to_vec()),
        (4, evaluation.target.content.0.to_vec()),
        (5, evaluation.derivation_catalog_sha256.to_vec()),
        (6, evaluation.seed_count.to_be_bytes().to_vec()),
        (7, evaluation.rule_count.to_be_bytes().to_vec()),
        (8, evaluation.decision.id().as_bytes().to_vec()),
        (9, Vec::new()),
        (10, Vec::new()),
        (11, Vec::new()),
        (12, Vec::new()),
    ]);
    match &evaluation.decision {
        PremiseAdmissionDecision::Derived(witness) => {
            fields.insert(9, encode_digests(&witness.seed_capability_sha256));
            fields.insert(10, encode_digests(&witness.rule_capability_sha256));
        }
        PremiseAdmissionDecision::OpenDerivationFrontier(frontier) => {
            fields.insert(11, encode_keys(&frontier.reachable));
            fields.insert(12, encode_keys(&frontier.unresolved_dependencies));
        }
        PremiseAdmissionDecision::IrreducibleProtocolRequired {
            derivation_coverage_capability_sha256,
            ..
        } => {
            fields.insert(9, derivation_coverage_capability_sha256.to_vec());
        }
        PremiseAdmissionDecision::IrreducibleProtocolStructurallyBound {
            derivation_coverage_capability_sha256,
            irreducible_protocol_capability_sha256,
            ..
        } => {
            fields.insert(9, derivation_coverage_capability_sha256.to_vec());
            fields.insert(10, irreducible_protocol_capability_sha256.to_vec());
        }
    }
    let bytes = encode_fields(&[], fields);
    if bytes.len() > MAX_CANONICAL_BYTES {
        return Err(PremiseAdmissionRefusal::CanonicalByteCapacityExceeded);
    }
    Ok(bytes)
}

fn insert_buckingham_pi(fields: &mut BTreeMap<u16, Vec<u8>>, disposition: BuckinghamPiDisposition) {
    match disposition {
        BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
            residual_group_count,
        } => {
            fields.insert(5, vec![0]);
            fields.insert(6, variable_basis_receipt_sha256.to_vec());
            fields.insert(7, producer_receipt_sha256.to_vec());
            fields.insert(8, watchdog_receipt_sha256.to_vec());
            fields.insert(9, residual_group_count.to_be_bytes().to_vec());
        }
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => {
            fields.insert(5, vec![1]);
            fields.insert(6, Vec::new());
            fields.insert(7, producer_receipt_sha256.to_vec());
            fields.insert(8, watchdog_receipt_sha256.to_vec());
            fields.insert(9, 0_u32.to_be_bytes().to_vec());
        }
    }
}

fn insert_chaos(fields: &mut BTreeMap<u16, Vec<u8>>, disposition: ChaosDisposition) {
    match disposition {
        ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        } => {
            fields.insert(11, vec![0]);
            fields.insert(12, producer_inapplicability_receipt_sha256.to_vec());
            fields.insert(13, watchdog_inapplicability_receipt_sha256.to_vec());
            fields.insert(14, Vec::new());
        }
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => {
            fields.insert(11, vec![1]);
            fields.insert(12, regime_partition_producer_receipt_sha256.to_vec());
            fields.insert(13, regime_partition_watchdog_receipt_sha256.to_vec());
            fields.insert(14, transition_law_receipt_sha256.to_vec());
        }
    }
}

fn encode_keys(keys: &[PremiseKey]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&u64::try_from(keys.len()).unwrap_or(u64::MAX).to_be_bytes());
    for key in keys {
        bytes.extend_from_slice(&key.role.0);
        bytes.extend_from_slice(&key.content.0);
    }
    bytes
}

fn encode_digests(digests: &[[u8; 32]]) -> Vec<u8> {
    let mut bytes = Vec::new();
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

fn encode_fields(domain: &[u8], fields: BTreeMap<u16, Vec<u8>>) -> Vec<u8> {
    let mut bytes = domain.to_vec();
    for (tag, payload) in fields {
        bytes.extend_from_slice(&tag.to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(payload.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&payload);
    }
    bytes
}
