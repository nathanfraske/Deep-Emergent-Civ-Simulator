//! Planet-facing access to the independently sealed absolute physical floor.
//!
//! The authority lives in `civsim-units`, below both the canonical runner and
//! the active physics substrate. This module is intentionally only the planet
//! boundary, so runner code cannot author a catalog and then verify it against
//! the same constructor.

pub use civsim_units::physics_floor::{
    audited_substrate_ledger, sealed_absolute_physics_floor, AuditedCatalogError, SealedFloorError,
};

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_ledger::{Provenance, Tier};
    use civsim_units::fundamentals::{COMPOSITES, REPRESENTATION_DEFINITIONS};
    use civsim_units::physics_floor::verify_absolute_physics_floor;

    #[test]
    fn catalog_is_the_three_measured_physical_coordinates() {
        let ledger = audited_substrate_ledger().expect("the sealed catalog is a valid DAG");
        assert_eq!(ledger.len(), 3);
        assert_eq!(
            ledger
                .inventory()
                .count(Tier::Universal, Provenance::Measured),
            3
        );
        assert_eq!(
            ledger
                .entries()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            ["fundamental.alpha", "fundamental.G", "fundamental.m_e"]
        );
    }

    #[test]
    fn representation_definitions_and_execution_relations_are_not_floor_entries() {
        let ledger = audited_substrate_ledger().expect("the sealed catalog is a valid DAG");
        assert_eq!(REPRESENTATION_DEFINITIONS.len(), 7);
        assert_eq!(COMPOSITES.len(), 4);
        assert_eq!(ledger.len(), 3);
        for definition in REPRESENTATION_DEFINITIONS {
            assert!(ledger
                .get(&format!("fundamental.{}", definition.symbol))
                .is_none());
        }
        for relation in COMPOSITES {
            assert!(ledger
                .get(&format!("fundamental.{}", relation.symbol))
                .is_none());
        }
    }

    #[test]
    fn sealed_floor_carries_a_pinned_receipt_for_every_leaf() {
        let floor = sealed_absolute_physics_floor().expect("the physical floor is sealed");
        verify_absolute_physics_floor(&floor).expect("the independent seal replays");
        for id in ["fundamental.alpha", "fundamental.G", "fundamental.m_e"] {
            assert!(floor.receipt(id).is_some());
        }
    }
}
