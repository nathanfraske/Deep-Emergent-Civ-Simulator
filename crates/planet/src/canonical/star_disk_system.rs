use super::{
    floor_magnitudes::AuditedFloorView,
    stellar_birth_measure::{self, StellarBirthMeasureCapability, StellarBirthMeasureRefusal},
};

/// Require the typed joint stellar-birth measure for Stage 1.
///
/// This cannot be satisfied by inserting an identity with the right spelling.
/// The opaque capability belongs to the repository-owned measure contract and
/// remains unavailable until both of its physical obligations have closed.
pub(crate) fn require_birth_measure(
    floor: &AuditedFloorView<'_>,
) -> Result<StellarBirthMeasureCapability, StellarBirthMeasureRefusal> {
    stellar_birth_measure::require_birth_measure(floor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::sealed_absolute_physics_floor;
    use crate::canonical::stellar_birth_measure::STELLAR_BIRTH_REALIZATION_MEASURE;

    #[test]
    fn the_current_floor_cannot_generate_contingency_without_stellar_birth_physics() {
        let floor = sealed_absolute_physics_floor().expect("the physical catalog is admissible");

        let floor_view =
            AuditedFloorView::from_floor(&floor).expect("the audited floor has typed magnitudes");
        let refusal = require_birth_measure(&floor_view)
            .expect_err("the universal-only floor cannot close a birth measure");
        assert_eq!(refusal.requirement_id(), STELLAR_BIRTH_REALIZATION_MEASURE);
        assert_eq!(
            refusal.to_string(),
            "derived or admitted absolute-floor measure 'stellar_birth.realization_measure'"
        );
    }
}
