//! Sealed, read-only projection of one canonical run outcome.

use super::{PlanetSnapshot, RunReceipt};

/// An immutable observer token borrowed from one canonical run outcome.
///
/// The private outcome borrow keeps the receipt and optional snapshot paired by
/// the planet crate. An observer can inspect a refusal or a completed snapshot,
/// but it cannot construct a different pairing or promote a refusal.
#[derive(Debug, Clone, Copy)]
pub struct PlanetObservation<'a> {
    inner: ObservationKind<'a>,
}

#[derive(Debug, Clone, Copy)]
enum ObservationKind<'a> {
    Complete(&'a PlanetSnapshot),
    Refused(&'a RunReceipt),
}

impl<'a> PlanetObservation<'a> {
    pub(super) const fn from_snapshot(snapshot: &'a PlanetSnapshot) -> Self {
        Self {
            inner: ObservationKind::Complete(snapshot),
        }
    }

    pub(super) const fn from_refusal(receipt: &'a RunReceipt) -> Self {
        Self {
            inner: ObservationKind::Refused(receipt),
        }
    }

    /// Receipt emitted by the observed canonical run.
    pub fn receipt(self) -> &'a RunReceipt {
        match self.inner {
            ObservationKind::Complete(snapshot) => snapshot.receipt(),
            ObservationKind::Refused(receipt) => receipt,
        }
    }

    /// Completed immutable state, or `None` when the run refused.
    pub fn snapshot(self) -> Option<&'a PlanetSnapshot> {
        match self.inner {
            ObservationKind::Complete(snapshot) => Some(snapshot),
            ObservationKind::Refused(_) => None,
        }
    }

    /// Receipt only when this observation represents a refused run.
    pub fn refusal_receipt(self) -> Option<&'a RunReceipt> {
        match self.inner {
            ObservationKind::Complete(_) => None,
            ObservationKind::Refused(receipt) => Some(receipt),
        }
    }

    /// Whether the observed run completed and therefore carries a snapshot.
    pub const fn is_complete(self) -> bool {
        matches!(self.inner, ObservationKind::Complete(_))
    }
}

#[cfg(test)]
mod tests {
    use crate::canonical::{run_planet, sealed_absolute_physics_floor};

    #[test]
    fn a_refusal_projects_the_same_receipt_and_never_a_snapshot() {
        let floor = sealed_absolute_physics_floor().expect("the physical floor seals");
        let outcome = run_planet(&floor);
        let observation = outcome.observation();

        assert!(!observation.is_complete());
        assert!(observation.snapshot().is_none());
        assert!(std::ptr::eq(observation.receipt(), outcome.receipt()));
        assert!(std::ptr::eq(
            observation
                .refusal_receipt()
                .expect("a refused observation carries its receipt"),
            outcome.receipt()
        ));
    }
}
