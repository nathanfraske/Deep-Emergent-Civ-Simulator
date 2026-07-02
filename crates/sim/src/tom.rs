// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Recursive theory of mind: the nested-model update (design Part 37, the resolved
//! R-TOM-UPDATE work, record 62.11).
//!
//! A mind models what another mind believes by running the resolved evidence engine
//! (Part 9.10, [`crate::evidence`]) recursively, with the subject re-pointed to the
//! target and the question changed from whether a thing is true to whether the target
//! believes it. The meta-frame is the same [`InferenceFrame`]: the same integer
//! log-odds accumulator with clamp-at-read, so the nested belief inherits the
//! order-independence and bit-identity of the first-order engine unchanged.
//!
//! What makes the nested model diverge from projection is a typed, audited
//! admissibility rule. Every piece of evidence carries an [`EvidenceOrder`]: world
//! evidence is about the world (the marble is in the box), and access evidence is
//! about a particular mind's epistemic access (that mind witnessed, was told, said, or
//! could reach a thing). A nested frame admits only access evidence about its own
//! target; the modeller's own world belief, and access evidence about a different
//! mind, are refused at the write path ([`NestedFrame::admit`] returns
//! [`ProjectionRejected`]). Because the evidence sets are disjoint, two minds with
//! different histories of a target's access reach provably different conclusions about
//! what the target believes, which is the Perner-Wimmer property that a false belief
//! is unreachable by any first-order shortcut.
//!
//! The membership of access-relation kinds is data, an [`AccessChannelRegistry`]
//! sibling to the trace-kind registry (design 9.9): a new sense or medium of access is
//! a data entry, so the engine enumerates no closed set of second-order evidence. The
//! per-channel weights and the meta thresholds are the owner's reserved calibrations,
//! read from the manifest and failing loud until set. The only fixed enum here is
//! [`EvidenceOrder`], the recursion-level discriminator, parameterised by the stable
//! id of whose access is meant: it is mechanism, not a catalogue of world content.

use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::evidence::{AttrKindId, InferenceFrame, InferenceParams, ValueId};
use civsim_core::{Fixed, StableId};
use serde::{Deserialize, Serialize};

/// What a piece of evidence is about: the world, or a specific mind's access to the
/// world. The recursion-level discriminator, fixed mechanism rather than a closed
/// catalogue of evidence kinds; the `Access` variant carries the stable id of whose
/// access is meant, so it is open over every mind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EvidenceOrder {
    /// Evidence about the world (the first-order corpus). Inadmissible to a nested
    /// frame: feeding it would be projection.
    World,
    /// Evidence about a named mind's epistemic access to the world. Admissible to that
    /// mind's nested frame and to no other.
    Access {
        /// Whose access this evidence is about.
        of: StableId,
    },
}

/// A data-defined identifier for an access-relation kind (witnessed, told, said,
/// reachable, absence, denied, or any a world adds). Not a closed enum.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct AccessChannelId(pub u32);

/// One access-relation kind. Membership only: the weight-of-evidence each kind carries
/// is a reserved calibration ([`AccessWeights`]), not baked into the data, so the
/// registry says which channels exist while the owner sets how much each weighs.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccessChannelDef {
    /// Stable identifier within the registry.
    pub id: AccessChannelId,
    /// The channel's name, the key under which its reserved weight is read
    /// (`tom.access_weight.<name>`).
    pub name: String,
}

/// The data-driven set of access-relation kinds (design Part 40, sibling to the
/// trace-kind registry). The set is open: a new sense or medium is an entry here, with
/// no engine change.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct AccessChannelRegistry {
    /// The channels, in file order.
    #[serde(default)]
    pub channels: Vec<AccessChannelDef>,
}

impl AccessChannelRegistry {
    /// Parse a registry from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }

    /// Serialize a registry to TOML text.
    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|e| e.to_string())
    }

    /// Load a registry from a file path.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_toml_str(&text)
    }

    /// The channel with this id, if any.
    pub fn by_id(&self, id: AccessChannelId) -> Option<&AccessChannelDef> {
        self.channels.iter().find(|c| c.id == id)
    }

    /// The channel with this name, if any.
    pub fn by_name(&self, name: &str) -> Option<&AccessChannelDef> {
        self.channels.iter().find(|c| c.name == name)
    }
}

/// The reserved per-channel weights (the TOM-W-ACCESS calibration). Read from the
/// manifest keyed by `tom.access_weight.<channel name>`; until the owner sets them,
/// reading fails loud rather than running on a fabricated default. Stored as a small
/// id-keyed list read in registry order, so a lookup is deterministic.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AccessWeights {
    weights: Vec<(AccessChannelId, Fixed)>,
}

impl AccessWeights {
    /// Read every channel's weight from the manifest, in registry order. Returns the
    /// fail-loud `Reserved` error while any channel's weight is still reserved, so a
    /// build cannot run tuned theory of mind on unset numbers.
    pub fn from_manifest(
        reg: &AccessChannelRegistry,
        m: &CalibrationManifest,
    ) -> Result<Self, CalibrationError> {
        let mut weights = Vec::with_capacity(reg.channels.len());
        for ch in &reg.channels {
            let key = format!("tom.access_weight.{}", ch.name);
            weights.push((ch.id, m.require_fixed(&key)?));
        }
        Ok(AccessWeights { weights })
    }

    /// Build a weight table directly. The sanctioned production path is
    /// [`AccessWeights::from_manifest`]; this exists for tests and tools, which supply
    /// clearly-labelled fixtures rather than the owner's reserved numbers.
    pub fn from_pairs(it: impl IntoIterator<Item = (AccessChannelId, Fixed)>) -> Self {
        AccessWeights {
            weights: it.into_iter().collect(),
        }
    }

    /// The weight for a channel, or `None` if the channel is not in the table.
    pub fn get(&self, id: AccessChannelId) -> Option<Fixed> {
        self.weights.iter().find(|(i, _)| *i == id).map(|(_, w)| *w)
    }
}

/// Read the meta-frame inference parameters (the TOM-COMMIT, TOM-CLAMP calibrations)
/// from the manifest. Distinct from the first-order `evidence.*` values, because
/// second-order evidence is noisier and the design reserves a possibly wider margin.
pub fn meta_params_from_manifest(
    m: &CalibrationManifest,
) -> Result<InferenceParams, CalibrationError> {
    Ok(InferenceParams {
        clamp: m.require_fixed("tom.meta_log_odds_clamp")?,
        commit_threshold: m.require_fixed("tom.meta_commit_threshold")?,
        margin: m.require_fixed("tom.meta_runner_up_margin")?,
    })
}

/// Returned when a nested frame is offered evidence it must not accept: the modeller's
/// own world belief, or access evidence about a different mind. Refusing these is the
/// typed anti-projection guarantee, so the rejection is an error rather than a silent
/// drop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProjectionRejected;

/// One nested belief: a model of one target mind's belief on one question. Reuses the
/// first-order [`InferenceFrame`] with its subject set to the target, and restricts
/// the write path to access evidence about that target.
#[derive(Clone, Debug)]
pub struct NestedFrame {
    of: StableId,
    depth: u8,
    frame: InferenceFrame,
}

impl NestedFrame {
    /// A fresh nested frame modelling `of`'s belief over `attr`, at nesting `depth`
    /// (the modeller's own belief is depth 0, the first nested store depth 1, the nest
    /// of a nest depth 2). The hypotheses are the target's possible belief-values; the
    /// engine's explicit unknown is the absence of a commit.
    pub fn new(
        of: StableId,
        depth: u8,
        attr: AttrKindId,
        hyps: impl IntoIterator<Item = ValueId>,
    ) -> Self {
        NestedFrame {
            of,
            depth,
            frame: InferenceFrame::new(of, attr, hyps),
        }
    }

    /// Union additional candidate hypotheses into the nested frame (see
    /// [`InferenceFrame::merge_hyps`]): the modelled belief's hypothesis space is the union of
    /// every candidate set the modeller has seen asserted about the target, so it does not depend
    /// on which access evidence arrived first.
    pub fn merge_hyps(&mut self, hyps: impl IntoIterator<Item = ValueId>) {
        self.frame.merge_hyps(hyps);
    }

    /// Whose mind this models.
    pub fn of(&self) -> StableId {
        self.of
    }

    /// The nesting depth of this frame.
    pub fn depth(&self) -> u8 {
        self.depth
    }

    /// Offer evidence to the frame under an explicit order. Admitted only when the
    /// order is access about this frame's own target; world evidence and access
    /// evidence about another mind are refused, which is the typed anti-projection
    /// guarantee. A harness can assert that no world evidence is ever admitted by
    /// checking this returns `Err` for [`EvidenceOrder::World`].
    pub fn admit(
        &mut self,
        order: EvidenceOrder,
        toward: ValueId,
        weight: Fixed,
        acuity: Fixed,
        from: StableId,
    ) -> Result<(), ProjectionRejected> {
        match order {
            EvidenceOrder::Access { of } if of == self.of => {
                self.frame.add_evidence(toward, weight, acuity, from);
                Ok(())
            }
            _ => Err(ProjectionRejected),
        }
    }

    /// Apply a believed access relation about the target through a data registry
    /// channel, toward the target-belief hypothesis the witnessed access points at.
    /// The order is access about this target by construction, so this is the sanctioned
    /// way to feed a nested frame. The witnessed value is supplied at observation time
    /// (a witness saw a specific thing), while the channel's weight-of-evidence is the
    /// reserved per-channel datum.
    pub fn observe_access(
        &mut self,
        weights: &AccessWeights,
        channel: AccessChannelId,
        toward: ValueId,
        acuity: Fixed,
        from: StableId,
    ) -> Result<(), ProjectionRejected> {
        let weight = weights.get(channel).ok_or(ProjectionRejected)?;
        self.admit(
            EvidenceOrder::Access { of: self.of },
            toward,
            weight,
            acuity,
            from,
        )
    }

    /// Commit the target's believed value if the leader clears the meta threshold and
    /// beats the runner-up by the meta margin, else the explicit unknown.
    pub fn commit(&self, params: &InferenceParams) -> Option<ValueId> {
        self.frame.commit(params)
    }

    /// The clamped meta log-odds total for a hypothesis, for inspection and the
    /// sincerity test.
    pub fn clamped_total(&self, value: ValueId, params: &InferenceParams) -> Option<Fixed> {
        self.frame.clamped_total(value, params)
    }

    /// The candidate hypotheses the target's belief ranges over, in their fixed order.
    pub fn hyps(&self) -> &[ValueId] {
        self.frame.hyps()
    }
}

/// Seeing through a lie: the deception verdict of the third sincerity frame in its
/// reduced form. Given the access-built model of the speaker's own belief and what the
/// speaker asserted, the lie is detected when the modelled true belief commits to a
/// value that differs from the assertion. The modeller's own world belief is not
/// consulted; only the access-built meta-belief and the assertion, so a witnessed
/// access that out-ranks the assertion (the reserved weight constraint) is what exposes
/// the lie.
pub fn detects_deception(
    speaker_model: &NestedFrame,
    asserted: ValueId,
    params: &InferenceParams,
) -> bool {
    matches!(speaker_model.commit(params), Some(v) if v != asserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WITNESSED: AccessChannelId = AccessChannelId(1);
    const TOLD: AccessChannelId = AccessChannelId(2);
    const SAID: AccessChannelId = AccessChannelId(3);
    const ONE: Fixed = Fixed::ONE;

    fn registry() -> AccessChannelRegistry {
        // Membership is data; these are the canonical starting channels, with no
        // numbers attached.
        AccessChannelRegistry {
            channels: vec![
                AccessChannelDef {
                    id: WITNESSED,
                    name: "witnessed".to_string(),
                },
                AccessChannelDef {
                    id: TOLD,
                    name: "told".to_string(),
                },
                AccessChannelDef {
                    id: SAID,
                    name: "said".to_string(),
                },
            ],
        }
    }

    fn weights() -> AccessWeights {
        // Fixture weights, not the owner's reserved manifest. The hard constraint is
        // honoured: witnessed (4) strictly exceeds told and said (3, 2), so a witnessed
        // access out-ranks a contrary assertion and lies are seen through.
        AccessWeights::from_pairs([
            (WITNESSED, Fixed::from_int(4)),
            (TOLD, Fixed::from_int(3)),
            (SAID, Fixed::from_int(2)),
        ])
    }

    fn params() -> InferenceParams {
        InferenceParams {
            clamp: Fixed::from_int(10),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    #[test]
    fn registry_round_trips_through_toml() {
        let original = registry();
        let text = original.to_toml_string().unwrap();
        let reloaded = AccessChannelRegistry::from_toml_str(&text).unwrap();
        assert_eq!(original, reloaded);
        assert_eq!(reloaded.by_name("witnessed").unwrap().id, WITNESSED);
        assert_eq!(reloaded.by_id(TOLD).unwrap().name, "told");
    }

    #[test]
    fn a_nested_frame_refuses_world_evidence() {
        // The typed anti-projection guarantee: the modeller's own world belief cannot
        // feed a nested model.
        let target = StableId(7);
        let mut m = NestedFrame::new(target, 1, AttrKindId(0), [10u32, 20]);
        assert_eq!(
            m.admit(
                EvidenceOrder::World,
                20,
                Fixed::from_int(9),
                ONE,
                StableId(1)
            ),
            Err(ProjectionRejected)
        );
        // Access evidence about a different mind is refused too.
        assert_eq!(
            m.admit(
                EvidenceOrder::Access { of: StableId(8) },
                20,
                Fixed::from_int(9),
                ONE,
                StableId(1)
            ),
            Err(ProjectionRejected)
        );
        // Access evidence about the frame's own target is admitted.
        assert!(m
            .observe_access(&weights(), WITNESSED, 10, ONE, StableId(1))
            .is_ok());
        assert_eq!(m.commit(&params()), Some(10));
    }

    #[test]
    fn sally_anne_classic_transfer() {
        // Sally saw the marble in the basket; while she is away it moves to the box.
        // The modeller's own belief is "box"; Sally's modelled belief is "basket".
        let (basket, the_box) = (10u32, 20u32);
        let sally = StableId(2);
        let modeller = StableId(1);
        let marble = StableId(99);

        let mut sally_model = NestedFrame::new(sally, 1, AttrKindId(0), [basket, the_box]);
        sally_model
            .observe_access(&weights(), WITNESSED, basket, ONE, modeller)
            .unwrap();
        // The modeller saw the move. That is world evidence and must not feed Sally's
        // model.
        assert!(sally_model
            .admit(
                EvidenceOrder::World,
                the_box,
                Fixed::from_int(5),
                ONE,
                modeller
            )
            .is_err());

        // The modeller's own first-order belief (the plain engine) tracks the truth.
        let mut own = InferenceFrame::new(marble, AttrKindId(0), [basket, the_box]);
        own.add_evidence(basket, Fixed::from_int(4), ONE, modeller);
        own.add_evidence(the_box, Fixed::from_int(5), ONE, modeller);

        assert_eq!(
            own.commit(&params()),
            Some(the_box),
            "modeller knows the truth"
        );
        assert_eq!(
            sally_model.commit(&params()),
            Some(basket),
            "Sally holds the false belief"
        );
        assert_ne!(
            sally_model.commit(&params()),
            own.commit(&params()),
            "the model diverges from projection"
        );
    }

    #[test]
    fn true_belief_control_does_not_over_correct() {
        // When Sally also has access to the truth, her model equals the truth, so the
        // rule diverges from projection only when it should. This guards against a rule
        // that reflexively negates the modeller's own belief.
        let (basket, the_box) = (10u32, 20u32);
        let sally = StableId(2);
        let modeller = StableId(1);

        let mut sally_model = NestedFrame::new(sally, 1, AttrKindId(0), [basket, the_box]);
        sally_model
            .observe_access(&weights(), WITNESSED, the_box, ONE, modeller)
            .unwrap();
        assert_eq!(
            sally_model.commit(&params()),
            Some(the_box),
            "with access to the truth the model matches it"
        );
    }

    #[test]
    fn unexpected_contents() {
        // A target sees a labelled box (the label says pencils) but not the candy
        // inside. The target's modelled belief is the label value, not the truth.
        let (pencils, candy) = (30u32, 40u32);
        let target = StableId(3);
        let modeller = StableId(1);

        let mut model = NestedFrame::new(target, 1, AttrKindId(1), [pencils, candy]);
        model
            .observe_access(&weights(), WITNESSED, pencils, ONE, modeller)
            .unwrap();
        // The world truth (candy) is inadmissible.
        assert!(model
            .admit(
                EvidenceOrder::World,
                candy,
                Fixed::from_int(9),
                ONE,
                modeller
            )
            .is_err());
        assert_eq!(model.commit(&params()), Some(pencils));
    }

    #[test]
    fn a_lie_is_believed_without_counter_access() {
        // Told a false value, with no witnessed access to contradict it, the target
        // adopts the lie.
        let (basket, the_box) = (10u32, 20u32);
        let target = StableId(4);
        let liar = StableId(5);

        let mut model = NestedFrame::new(target, 1, AttrKindId(0), [basket, the_box]);
        model
            .observe_access(&weights(), TOLD, the_box, ONE, liar)
            .unwrap();
        assert_eq!(
            model.commit(&params()),
            Some(the_box),
            "the lie is believed"
        );
    }

    #[test]
    fn a_lie_is_seen_through_by_witnessed_access() {
        // The speaker witnessed the truth (basket) but asserts the box. The modeller's
        // access-built model of the speaker's own belief commits basket, because a
        // witnessed access out-ranks a contrary assertion (the reserved weight
        // constraint), so the assertion is exposed as a lie.
        let (basket, the_box) = (10u32, 20u32);
        let speaker = StableId(6);
        let modeller = StableId(1);

        let mut speaker_model = NestedFrame::new(speaker, 1, AttrKindId(0), [basket, the_box]);
        speaker_model
            .observe_access(&weights(), WITNESSED, basket, ONE, modeller)
            .unwrap();
        assert_eq!(speaker_model.commit(&params()), Some(basket));
        assert!(
            detects_deception(&speaker_model, the_box, &params()),
            "the speaker believes basket but asserts box, so the lie is seen through"
        );
        // A sincere assertion that matches the modelled belief raises no verdict.
        assert!(!detects_deception(&speaker_model, basket, &params()));
    }

    #[test]
    fn second_order_false_belief_is_representable() {
        // Depth 2: the modeller models A's belief about B's belief. The modeller
        // believes A watched B head to the park (access about A's access to B), so the
        // depth-2 frame commits "A believes B believes park", which differs from the
        // modeller's own knowledge that B went to the church.
        let (park, church) = (50u32, 60u32);
        let a = StableId(11);
        let modeller = StableId(1);

        let mut a_about_b = NestedFrame::new(a, 2, AttrKindId(2), [park, church]);
        a_about_b
            .observe_access(&weights(), WITNESSED, park, ONE, modeller)
            .unwrap();
        assert_eq!(a_about_b.depth(), 2);
        assert_eq!(
            a_about_b.commit(&params()),
            Some(park),
            "the depth-2 belief is reachable only through two access levels"
        );
    }

    #[test]
    fn the_meta_frame_is_order_independent() {
        // The nested update inherits the order-independence of the first-order engine:
        // the same access evidence in any order yields the same modelled belief.
        let (basket, the_box) = (10u32, 20u32);
        let t = StableId(2);
        let m = StableId(1);

        let mut a = NestedFrame::new(t, 1, AttrKindId(0), [basket, the_box]);
        a.observe_access(&weights(), WITNESSED, basket, ONE, m)
            .unwrap();
        a.observe_access(&weights(), TOLD, the_box, ONE, m).unwrap();
        a.observe_access(&weights(), WITNESSED, basket, ONE, m)
            .unwrap();

        let mut b = NestedFrame::new(t, 1, AttrKindId(0), [basket, the_box]);
        b.observe_access(&weights(), WITNESSED, basket, ONE, m)
            .unwrap();
        b.observe_access(&weights(), WITNESSED, basket, ONE, m)
            .unwrap();
        b.observe_access(&weights(), TOLD, the_box, ONE, m).unwrap();

        assert_eq!(a.commit(&params()), b.commit(&params()));
        assert_eq!(
            a.commit(&params()),
            Some(basket),
            "two witnessed beat one told"
        );
    }

    #[test]
    fn weights_from_manifest_fail_loud_while_reserved() {
        let toml = r#"
[[reserved]]
id = "tom.access_weight.witnessed"
basis = "weight of a witnessed access"
status = "reserved"
source = "Part 37"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        let err = AccessWeights::from_manifest(&registry(), &m).unwrap_err();
        assert_eq!(
            err,
            CalibrationError::Reserved("tom.access_weight.witnessed".to_string())
        );
    }
}
