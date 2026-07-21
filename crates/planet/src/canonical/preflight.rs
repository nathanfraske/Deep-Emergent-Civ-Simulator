use super::Refusal;
use civsim_ledger::AbsolutePhysicsFloor;
use civsim_units::physics_floor::verify_absolute_physics_floor;

/// Validate the canonical front door's sole value-bearing input.
///
/// The absolute floor has already passed derivation-first structural admission. Preflight also requires
/// its complete ordered graph to equal the repository-owned audited catalog. Realization identity, written
/// state, and contingency are generated inside the run only after the corresponding physical measures
/// exist; no caller specification exists at this boundary.
pub fn preflight(floor: &AbsolutePhysicsFloor) -> Vec<Refusal> {
    floor_catalog_mismatch(floor)
        .map(Refusal::floor_catalog_mismatch)
        .into_iter()
        .collect()
}

pub(crate) fn floor_catalog_mismatch(floor: &AbsolutePhysicsFloor) -> Option<String> {
    verify_absolute_physics_floor(floor)
        .err()
        .map(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{sealed_absolute_physics_floor, RefusalCode};
    use civsim_ledger::{
        DerivationExhaustionReceipt, Entry, GapLawReceipt, Ledger, Provenance, ResidualLawReceipt,
        Tier,
    };

    fn physical_floor() -> AbsolutePhysicsFloor {
        sealed_absolute_physics_floor().expect("the physical catalog is admissible")
    }

    #[test]
    fn the_audited_floor_is_the_only_preflight_input() {
        assert!(preflight(&physical_floor()).is_empty());
    }

    #[test]
    fn arbitrary_universal_measurement_is_not_authorized_by_its_tags() {
        let ledger = Ledger::build([Entry {
            id: "fundamental.cited_knob".into(),
            tier: Tier::Universal,
            provenance: Provenance::Measured,
            inputs: vec![],
        }])
        .unwrap();
        let receipt = DerivationExhaustionReceipt {
            entry_id: "fundamental.cited_knob".into(),
            phenomenon: "fixture".into(),
            derivation_attempts: vec!["derive-first fixture attempt".into()],
            residual_slot: "fixture.slot".into(),
            buckingham_pi_groups: 1,
            gap_law: GapLawReceipt {
                reference_validity: "fixture evidence".into(),
                gap_dispatch: "fixture evidence".into(),
                smooth_systematics: "fixture evidence".into(),
                scale_free_limit: "fixture evidence".into(),
            },
            residual_law: ResidualLawReceipt {
                conservation: "fixture evidence".into(),
                disequilibrium: "fixture evidence".into(),
                fluctuation_dissipation: "fixture evidence".into(),
                dimensional_analysis: "fixture evidence".into(),
            },
        };
        let floor = AbsolutePhysicsFloor::admit(ledger, [receipt])
            .expect("structural admission does not claim catalog authority");
        let refusals = preflight(&floor);
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].code(), RefusalCode::FloorCatalogMismatch);
        assert!(refusals[0].detail().contains("fundamental.cited_knob"));
    }

    #[test]
    fn the_exact_catalog_with_caller_receipt_prose_is_still_rejected() {
        let sealed = physical_floor();
        let mut receipts: Vec<_> = sealed
            .entries()
            .map(|entry| sealed.receipt(&entry.id).unwrap().clone())
            .collect();
        receipts[0].derivation_attempts = vec!["caller says derivation failed".into()];
        let floor = AbsolutePhysicsFloor::admit(
            crate::canonical::audited_substrate_ledger().expect("the physical catalog is valid"),
            receipts,
        )
        .expect("generic structural admission cannot know the repository receipt fingerprint");

        let refusals = preflight(&floor);
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].code(), RefusalCode::FloorCatalogMismatch);
        assert!(refusals[0]
            .detail()
            .contains("receipt 'fundamental.alpha' has fingerprint"));
    }
}
