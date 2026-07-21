//! Repository-owned proof artifacts for the Stage 1 stellar-birth contract.
//!
//! These types are deliberately opaque. A matching identity, provenance tag,
//! citation, or caller value cannot construct either proof. Future constructors
//! must live at this boundary and may return a joint-measure proof only from a
//! complete derivation or from an admitted `[M]` or `[E]` floor artifact with
//! the full exhaustion, Buckingham-Pi, Gap Law, Chaos Protocol, Residual Law,
//! and unique-slot receipts. The coordinate-law proof has a separate authority
//! because an admitted measure cannot choose its own realization.

use super::floor_magnitudes::AuditedFloorView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VerifiedJointPhysicalMeasure {
    _seal: ArtifactSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VerifiedRealizationCoordinateLaw {
    _seal: ArtifactSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArtifactSeal;

/// The exact typed artifacts currently available from repository authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RepositoryStellarBirthArtifacts {
    pub(super) joint_measure: Option<VerifiedJointPhysicalMeasure>,
    pub(super) coordinate_law: Option<VerifiedRealizationCoordinateLaw>,
}

/// Resolve Stage 1 proof artifacts after the absolute floor has been audited.
///
/// Both proofs are absent today. Returning typed absence keeps the evaluator
/// executable without laundering closure through strings or booleans.
pub(super) const fn resolve_repository_artifacts(
    _floor: &AuditedFloorView<'_>,
) -> RepositoryStellarBirthArtifacts {
    RepositoryStellarBirthArtifacts {
        joint_measure: None,
        coordinate_law: None,
    }
}

#[cfg(test)]
impl RepositoryStellarBirthArtifacts {
    pub(super) const fn test_fixture(joint_measure: bool, coordinate_law: bool) -> Self {
        Self {
            joint_measure: if joint_measure {
                Some(VerifiedJointPhysicalMeasure {
                    _seal: ArtifactSeal,
                })
            } else {
                None
            },
            coordinate_law: if coordinate_law {
                Some(VerifiedRealizationCoordinateLaw {
                    _seal: ArtifactSeal,
                })
            } else {
                None
            },
        }
    }
}
