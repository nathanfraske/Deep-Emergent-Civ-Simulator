use super::*;

#[test]
fn sealed_eps0_relation_mints_one_narrow_current_artifact() {
    let artifact = resolve_repository_derived_relation_premise()
        .expect("the sealed derived relation must pass both checkers");
    assert_eq!(
        artifact.schema_id(),
        "civsim.planet.derived-law-premise-execution-relation.v2"
    );
    assert!(!artifact.canonical_relation_bytes().is_empty());
    assert!(artifact.output_bits() > 0);
    assert!(artifact.output_scale_bits() > 0);
    assert_ne!(artifact.output_projection_receipt_sha256(), [0; 32]);
    assert_ne!(artifact.pair_receipt_sha256(), [0; 32]);
    assert_ne!(artifact.producer_canary_sha256(), [0; 32]);
    assert_ne!(
        artifact.producer_canary_sha256(),
        artifact.watchdog_canary_sha256()
    );
    assert!(!artifact.species_membership_authority());
    assert!(!artifact.global_physical_premise_coverage());
    assert!(verify_current_artifact(&artifact));
    assert_eq!(
        receipt_schema_id(),
        "civsim.planet.derived-law-premise-eps0-pair-receipt.v2"
    );
    assert_eq!(
        canary_suite_id(),
        "civsim.planet.derived-law-premise-eps0-canaries.v2"
    );
}

#[test]
fn producer_and_watchdog_packets_outputs_and_live_canaries_agree() {
    let producer_packet = producer::sealed_packet().unwrap();
    let watchdog_packet = watchdog::sealed_packet().unwrap();
    assert_eq!(producer_packet, watchdog_packet);
    assert_eq!(
        producer::inspect(&producer_packet),
        watchdog::inspect(&watchdog_packet)
    );
    let producer_canary = producer::canary_evidence(&producer_packet).unwrap();
    let watchdog_canary = watchdog::canary_evidence(&watchdog_packet).unwrap();
    assert_ne!(producer_canary.transcript_id, watchdog_canary.transcript_id);
    assert_ne!(
        producer_canary.transcript_sha256,
        watchdog_canary.transcript_sha256
    );
    assert_eq!(producer_canary.case_count, watchdog_canary.case_count);
}

#[test]
fn changed_value_formula_role_and_route_do_not_preserve_the_claim() {
    let packet = producer::sealed_packet().unwrap();
    let baseline = producer::inspect(&packet).unwrap();
    let mutations = [
        {
            let mut changed = packet.clone();
            changed.output.bits += 1;
            changed
        },
        {
            let mut changed = packet.clone();
            changed.relation.formula = "e^2 * (2 * alpha * h * c)".to_owned();
            changed
        },
        {
            let mut changed = packet.clone();
            changed.inputs[1].role = model::InputRole::RepresentationDefinition;
            changed
        },
        {
            let mut changed = packet.clone();
            changed.alpha_admission.owner_admission_receipt[0] ^= 1;
            changed
        },
    ];
    for mutation in mutations {
        let produced = producer::inspect(&mutation);
        let watched = watchdog::inspect(&mutation);
        assert_eq!(produced, watched);
        assert!(match produced.as_ref() {
            Ok(output) => output != &baseline,
            Err(_) => true,
        });
    }
}

#[test]
fn changed_output_scale_and_output_dimension_are_live_refusals() {
    let packet = producer::sealed_packet().unwrap();

    let mut changed_scale = packet.clone();
    changed_scale.output.scale_bits += 1;
    assert_eq!(
        producer::inspect(&changed_scale),
        Err(model::DerivedRelationRefusal::RepresentationScaleMismatch)
    );
    assert_eq!(
        watchdog::inspect(&changed_scale),
        Err(model::DerivedRelationRefusal::RepresentationScaleMismatch)
    );

    let mut changed_dimension = packet;
    changed_dimension.output.dimension[0] += 1;
    assert_eq!(
        producer::inspect(&changed_dimension),
        Err(model::DerivedRelationRefusal::OutputBindingInvalid)
    );
    assert_eq!(
        watchdog::inspect(&changed_dimension),
        Err(model::DerivedRelationRefusal::OutputBindingInvalid)
    );
}

#[test]
fn artifact_mutation_fails_fresh_sealed_replay() {
    let artifact = resolve_repository_derived_relation_premise().unwrap();

    let mut changed = artifact.clone();
    changed.output_bits += 1;
    assert!(!verify_current_artifact(&changed));

    let mut changed = artifact.clone();
    changed.pair_receipt_sha256[0] ^= 1;
    assert!(!verify_current_artifact(&changed));

    let mut changed = artifact.clone();
    changed.capability.capability_sha256[0] ^= 1;
    assert!(!verify_current_artifact(&changed));

    let mut changed = artifact;
    changed.ancestry_receipt_sha256[0] ^= 1;
    assert!(!verify_current_artifact(&changed));
}
