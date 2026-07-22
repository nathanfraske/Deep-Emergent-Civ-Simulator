//! Immutable scene projections over the planet-owned observation boundary.

mod frontier;
mod transcript;

pub use frontier::{
    AnalysisScene, DimensionalCensusScene, OpenRequirementScene, RefusalReasonScene, RefusalScene,
    SpeciesAttemptScene, SpeciesDerivationScene, StageScene,
};
pub use transcript::{
    FloorEventScene, FloorScene, ProvenanceScene, RepresentationValueScene, TranscriptEventScene,
    TranscriptScene, ValuePayloadVisibility,
};

use crate::{ObservationView, SnapshotView};

/// Which sealed canonical outcome an observer is looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationSceneStatus {
    /// A completed immutable snapshot exists.
    Complete,
    /// The canonical run stopped with an exact refusal receipt.
    Refused,
}

/// Root observer projection over one sealed canonical outcome.
///
/// The private view keeps this projection tied to the planet-owned observation
/// token. This type has no physical input, mutation, or completion surface.
#[derive(Debug, Clone, Copy)]
pub struct ObservationScene<'a> {
    view: ObservationView<'a>,
}

impl<'a> ObservationScene<'a> {
    pub(crate) const fn new(view: ObservationView<'a>) -> Self {
        Self { view }
    }

    /// Report whether the sealed observation completed or refused.
    pub const fn status(self) -> ObservationSceneStatus {
        if self.view.is_complete() {
            ObservationSceneStatus::Complete
        } else {
            ObservationSceneStatus::Refused
        }
    }

    /// Borrow the completed snapshot projection when physical closure exists.
    pub fn snapshot(self) -> Option<SnapshotView<'a>> {
        self.view.snapshot()
    }

    /// Project the exact refusal receipt when the run stopped.
    pub fn refusal(self) -> Option<RefusalScene<'a>> {
        self.view.refusal().map(RefusalScene::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_entrypoint_requires_the_sealed_observation_adapter() {
        fn project<'a>(view: ObservationView<'a>) -> ObservationScene<'a> {
            ObservationScene::new(view)
        }

        let projector: for<'a> fn(ObservationView<'a>) -> ObservationScene<'a> = project;
        let _ = projector;
    }

    #[test]
    fn refusal_scene_exposes_the_typed_frontier_without_text_parsing() {
        fn inspect(view: crate::RefusalView<'_>) -> usize {
            RefusalScene::new(view)
                .refusals()
                .map(|reason| {
                    reason
                        .open_requirements()
                        .iter()
                        .map(|requirement| {
                            requirement.obligations().len() + requirement.analyses().len()
                        })
                        .sum::<usize>()
                })
                .sum()
        }

        let inspector: for<'a> fn(crate::RefusalView<'a>) -> usize = inspect;
        let _ = inspector;
    }
}
