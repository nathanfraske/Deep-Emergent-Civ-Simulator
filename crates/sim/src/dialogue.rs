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

//! Modelled dialogue: the data-defined move and force substrates and the content gate
//! (design Part 9.5, the resolved R-CONVERSE work, record 62.14).
//!
//! Modelled dialogue is the promoted-tier refinement of the gossip loop (9.5): at the
//! significant tier the engine records the communicative acts that produce a belief
//! change as first-class canonical events, where the one-pass loop only transmits the
//! outcome. This module is the additive foundation under that resolution: the two data
//! registries (the dialogue-move registry and the etic force floor), the felicity
//! conditions that gate whether a move lands, and the content gate that refuses any data
//! which would smuggle a graded persuasion or fidelity weight into the substrate. It
//! carries no tick integration; the speak-as-intent selection and the per-move response
//! loop build on it.
//!
//! Three substrate siblings make this data-driven (Principle 11), the same hardening
//! applied to the access-channel registry ([`crate::tom`]), the trace-kind registry, the
//! institution-function substrate, the value substrate, and the semantic substrate:
//!
//! - The etic force floor ([`ForceFloor`]) is the small menu of primitive effects a move
//!   can fire. Its membership is data (a starting menu, not a fixed fact), and each entry
//!   names a [`ForceKind`], the engine affordance it realises. [`ForceKind`] is a fixed
//!   mechanism enum exactly as [`crate::tom::EvidenceOrder`] is: it is the affordance
//!   discriminator, not a catalogue of world content, because each variant is a call into
//!   a mechanism the engine already has (a told-evidence facet 9.5, an inquiry goal 9.13,
//!   a naming-game form proposal 33.9, an uptake, a conditional intent 37, a contact). A
//!   move adds no new authored behaviour; it composes affordances the engine already owns.
//! - The dialogue-move registry ([`MoveRegistry`]) is the recognised move kinds. A
//!   [`MoveKindDef`] is a recurring bundle of force effects a community comes to recognise
//!   and respond to as one move (an assertion, a question, a promise), with the responses
//!   it conventionally expects, whether its sincerity is judged, the felicity conditions
//!   under which it lands, and a deterministic gloss handle. Which compositions a culture
//!   conventionalises is the emic repertoire and can emerge; the starter repertoire is
//!   data so a world is legible from the first tick.
//! - Felicity conditions ([`FelicityCond`]) decide whether a move's force lands at all,
//!   never how hard it lands (the Austin felicity condition made structural). A condition
//!   names a state dimension the world already carries (a role in an institution 36, a
//!   trust band 37, a value-distance band 21, a channel capability 33.3) and a reserved
//!   band over it: it reads that dimension and returns pass or fail, with no magnitude.
//!
//! No graded weight, by construction. The magnitude a move delivers (how much a told
//! assertion moves a belief, how much a persuasion moves a value) is the consumer's
//! computation under the unmodified inference engine (9.10), opinion kernel (Part 28),
//! and deception frame (Part 37), keyed off the existing reserved calibrations
//! (`gossip.told_weight` and the tom and inquiry scales). The substrate carries no field
//! in which such a magnitude could sit: force is a set of affordance ids that fire or do
//! not, and a felicity condition is two names (a dimension and a reserved band) resolving
//! to a boolean. The reserved felicity band bounds live in the calibration manifest and
//! fail loud until the owner sets them, never fabricated here. The content gate
//! ([`MoveRegistry::content_gate`]) enforces that structural guarantee together with
//! referential integrity and well-formed data at load.

use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::evidence::AttrKindId;
use crate::language::ConceptId;
use civsim_core::{EventId, Fixed, StableId};
use serde::{Deserialize, Serialize};

/// The etic floor of primitive effects: the engine affordances a dialogue move's force
/// can fire. A fixed mechanism enum, not a catalogue of world content, on the same
/// footing as [`crate::tom::EvidenceOrder`]: every variant is a call into a mechanism the
/// engine already has, so a move composes affordances rather than authoring behaviour.
/// The floor grows when the engine resolves a new mechanism, never from world data; what
/// world data composes from this floor (the move kinds) is the emergent part.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForceKind {
    /// Tell a told-evidence facet into the hearer's belief (the assertion primitive,
    /// design 9.5). The magnitude is the gossip told-weight, a reserved consumer value.
    TellEvidence,
    /// Raise an inquiry goal in the hearer (the question primitive, design 9.13). The
    /// motive salience is the reserved inquiry scale.
    RaiseInquiry,
    /// Propose a concept-and-form pairing (the naming-game coordination move, 33.9).
    ProposeForm,
    /// Register an uptake of the effect's sign (acceptance is positive, refusal the same
    /// with the opposite sign). The step size is the reserved reinforcement rate of 9.3.
    RegisterUptake,
    /// Write a conditional intent into the hearer's model of the speaker (the promise or
    /// threat primitive, Part 37), for the hearer to reason over.
    WriteIntent,
    /// Open a channel of contact toward the addressee (the greeting primitive). The trust
    /// touch is the reserved contact value of Part 37.
    OpenContact,
}

impl ForceKind {
    /// Whether this kind is an affordance (an engine mechanism the move realises) rather
    /// than an outcome (a graded persuasion or fidelity setter). Every floor primitive is
    /// an affordance by construction, which is the structural half of the content gate: an
    /// outcome cannot be expressed because no variant carries a magnitude. The content
    /// gate calls this so the classification the design names is explicit in the code.
    pub fn is_affordance(&self) -> bool {
        match self {
            ForceKind::TellEvidence
            | ForceKind::RaiseInquiry
            | ForceKind::ProposeForm
            | ForceKind::RegisterUptake
            | ForceKind::WriteIntent
            | ForceKind::OpenContact => true,
        }
    }
}

/// The sign of a primitive effect: a direction, never a magnitude. An uptake leans
/// positive (acceptance) or negative (refusal); most effects are unsigned. A direction is
/// mechanism, not a graded weight, so it is admissible under the content gate.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum EffectSign {
    /// No sign (the default for assertion, inquiry, form proposal, intent, contact).
    #[default]
    Neutral,
    /// A positive lean (acceptance, agreement).
    Positive,
    /// A negative lean (refusal, doubt).
    Negative,
}

/// A data-defined identifier for a force-floor primitive.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct ForceEffectId(pub u32);

/// One primitive in the etic force floor: an id, the engine affordance it realises, and
/// its sign. Membership only; the magnitude each affordance delivers is the consumer's
/// reserved calibration, not carried here.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ForceEffectDef {
    /// Stable identifier within the floor.
    pub id: ForceEffectId,
    /// The engine affordance this primitive fires.
    pub kind: ForceKind,
    /// The effect's direction (relevant to uptake; neutral otherwise).
    #[serde(default)]
    pub sign: EffectSign,
    /// A short description of what the primitive does.
    #[serde(default)]
    pub name: String,
}

/// The data-driven etic force floor: the menu of primitive effects available in a world.
/// Open membership (a world may omit or reorder primitives), but every entry must name a
/// recognised [`ForceKind`], so the floor cannot define an affordance the engine has no
/// mechanism for.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct ForceFloor {
    /// The primitives, in file order.
    #[serde(default)]
    pub effects: Vec<ForceEffectDef>,
}

impl ForceFloor {
    /// Parse a floor from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }

    /// Serialize a floor to TOML text.
    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|e| e.to_string())
    }

    /// Load a floor from a file path.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_toml_str(&text)
    }

    /// The primitive with this id, if any.
    pub fn effect(&self, id: ForceEffectId) -> Option<&ForceEffectDef> {
        self.effects.iter().find(|e| e.id == id)
    }
}

/// A felicity condition: the names of a state dimension the world already carries and a
/// reserved band over it. It gates (pass or fail), never weights, so it carries no
/// magnitude of its own. The dimension is read against world state at evaluation time (a
/// role in an institution 36, a trust band 37, a value-distance band 21, a channel reach
/// 33.3); the band's inclusive bounds are reserved owner calibrations, read from the
/// manifest by the band key. A move whose conditions fail misfires, landing as a bare
/// attempt with no force, the Austin condition made structural.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FelicityCond {
    /// The state dimension this condition reads (for example `trust`, `role.command`,
    /// `value_distance`, `channel.reach`).
    pub dimension: String,
    /// The reserved band key naming the inclusive bounds in the manifest. The lower and
    /// upper bounds are read from `<band>.lo` and `<band>.hi`.
    pub band: String,
}

/// The resolved inclusive bounds of a felicity band, read from the manifest. Separate from
/// [`FelicityCond`] (which carries only the band's name) because the bounds are reserved
/// owner numbers that fail loud until set, never fabricated in the move data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResolvedBand {
    /// The inclusive lower bound.
    pub lo: Fixed,
    /// The inclusive upper bound.
    pub hi: Fixed,
}

impl ResolvedBand {
    /// Read a band's bounds from the manifest by its key, failing loud while either bound
    /// is still reserved, so a build cannot gate dialogue on unset numbers.
    pub fn from_manifest(m: &CalibrationManifest, band: &str) -> Result<Self, CalibrationError> {
        Ok(ResolvedBand {
            lo: m.require_fixed(&format!("{band}.lo"))?,
            hi: m.require_fixed(&format!("{band}.hi"))?,
        })
    }

    /// Whether a reading of the dimension falls in the band. A pure gate: `lo <= value <=
    /// hi`, returning a boolean and no magnitude.
    pub fn holds(&self, value: Fixed) -> bool {
        value >= self.lo && value <= self.hi
    }

    /// Whether the band is well-formed (`lo <= hi`). An inverted band can never hold.
    pub fn is_well_formed(&self) -> bool {
        self.lo <= self.hi
    }
}

/// A data-defined identifier for a recognised dialogue-move kind.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct MoveKindId(pub u32);

/// One recognised dialogue-move kind: a bundle of force effects a community recognises and
/// responds to as a single move (an assertion, a question, a promise, a refusal). The
/// label and gloss are emic data and never enter the force computation (the
/// content-blindness invariant of Part 41); the force is the etic bundle, which is what
/// determines the move's effect.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MoveKindDef {
    /// Stable identifier within the registry.
    pub id: MoveKindId,
    /// The emic label (data, never consulted by the force computation).
    #[serde(default)]
    pub name: String,
    /// The primitive effects this move's force fires, as floor ids.
    #[serde(default)]
    pub force: Vec<ForceEffectId>,
    /// The move kinds this one conventionally expects in reply (the adjacency a response
    /// points back along), as registry ids.
    #[serde(default)]
    pub expects: Vec<MoveKindId>,
    /// Whether the deception frame judges this move's sincerity (true for an assertion,
    /// false for a greeting).
    #[serde(default)]
    pub sincerity_judged: bool,
    /// The felicity conditions under which the move's force lands.
    #[serde(default)]
    pub felicity: Vec<FelicityCond>,
    /// A deterministic gloss handle for the legibility layer (33.2). Texture only; never
    /// consulted by the force computation.
    #[serde(default)]
    pub gloss: String,
}

impl MoveKindDef {
    /// Whether every felicity condition holds, given a reading of each named dimension and
    /// a resolver for each named band. A dimension the reader cannot supply, or a band the
    /// resolver cannot supply (still reserved), fails closed (the move misfires), since an
    /// unconfirmable condition cannot license the force. Returns a boolean; the magnitude
    /// of the force that then lands is the consumer's computation, not set here.
    pub fn felicitous(
        &self,
        reading: impl Fn(&str) -> Option<Fixed>,
        bands: impl Fn(&str) -> Option<ResolvedBand>,
    ) -> bool {
        self.felicity
            .iter()
            .all(|c| match (reading(&c.dimension), bands(&c.band)) {
                (Some(v), Some(b)) => b.holds(v),
                _ => false,
            })
    }
}

/// The data-driven dialogue-move registry (design Part 9.5, sibling to the access-channel
/// and institution-function substrates). Open membership: a culture's starter and
/// emergent repertoire are entries here, not engine variants.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct MoveRegistry {
    /// The move kinds, in file order.
    #[serde(default)]
    pub moves: Vec<MoveKindDef>,
}

/// What the content gate refused, naming the offending entry so a bad data load fails
/// loud rather than running on a malformed substrate.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ContentGateError {
    /// Two move kinds share an id.
    DuplicateMoveId(MoveKindId),
    /// Two force-floor primitives share an id.
    DuplicateEffectId(ForceEffectId),
    /// A move's force references a primitive not in the floor.
    DanglingForce {
        /// The move kind whose force list dangles.
        move_kind: MoveKindId,
        /// The unresolved force-effect id.
        effect: ForceEffectId,
    },
    /// A move expects a reply kind that is not in the registry.
    DanglingExpect {
        /// The move kind whose adjacency dangles.
        move_kind: MoveKindId,
        /// The unresolved expected move kind.
        expected: MoveKindId,
    },
    /// A move carries a felicity condition missing its dimension or band name, a gate with
    /// nothing to read.
    MalformedFelicity {
        /// The move kind that carries the malformed condition.
        move_kind: MoveKindId,
        /// What is wrong with the condition.
        reason: &'static str,
    },
    /// A move's force references a primitive whose kind is not an affordance. Structurally
    /// unreachable while [`ForceKind`] is closed to affordances, but checked so the
    /// classification the design names is enforced rather than assumed.
    NonAffordance {
        /// The move kind whose force fires a non-affordance.
        move_kind: MoveKindId,
        /// The offending force-effect id.
        effect: ForceEffectId,
    },
}

impl std::fmt::Display for ContentGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentGateError::DuplicateMoveId(id) => write!(f, "duplicate move kind id {id:?}"),
            ContentGateError::DuplicateEffectId(id) => {
                write!(f, "duplicate force effect id {id:?}")
            }
            ContentGateError::DanglingForce { move_kind, effect } => write!(
                f,
                "move {move_kind:?} fires force effect {effect:?}, which is not in the floor"
            ),
            ContentGateError::DanglingExpect {
                move_kind,
                expected,
            } => write!(
                f,
                "move {move_kind:?} expects reply {expected:?}, which is not in the registry"
            ),
            ContentGateError::MalformedFelicity { move_kind, reason } => write!(
                f,
                "move {move_kind:?} has a malformed felicity condition: {reason}"
            ),
            ContentGateError::NonAffordance { move_kind, effect } => write!(
                f,
                "move {move_kind:?} fires force effect {effect:?}, which is not an affordance"
            ),
        }
    }
}

impl std::error::Error for ContentGateError {}

impl MoveRegistry {
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

    /// The move kind with this id, if any.
    pub fn move_kind(&self, id: MoveKindId) -> Option<&MoveKindDef> {
        self.moves.iter().find(|m| m.id == id)
    }

    /// The content gate (design Part 9.5, Part 41). Classifies every move kind and
    /// felicity condition as an affordance or an outcome and refuses any that would set a
    /// graded persuasion or fidelity weight; because the substrate provides no field for a
    /// magnitude (force is a set of affordance ids and a felicity condition is two names),
    /// that half of the gate is structural, and this function enforces it together with
    /// the integrity a data load can actually violate: unique ids, force and adjacency
    /// references that resolve, every fired primitive an affordance, and every felicity
    /// condition naming both a dimension and a band. Returns the first violation in
    /// deterministic order (effects then moves, in file order) so a load fails the same
    /// way every time. The reserved band bounds are validated separately on the fail-loud
    /// manifest path ([`ResolvedBand::from_manifest`]).
    pub fn content_gate(&self, floor: &ForceFloor) -> Result<(), ContentGateError> {
        // Unique force-effect ids.
        for (i, e) in floor.effects.iter().enumerate() {
            if floor.effects[..i].iter().any(|p| p.id == e.id) {
                return Err(ContentGateError::DuplicateEffectId(e.id));
            }
        }
        // Unique move kind ids.
        for (i, m) in self.moves.iter().enumerate() {
            if self.moves[..i].iter().any(|p| p.id == m.id) {
                return Err(ContentGateError::DuplicateMoveId(m.id));
            }
        }
        for m in &self.moves {
            // Every fired primitive resolves in the floor and is an affordance.
            for &fid in &m.force {
                match floor.effect(fid) {
                    None => {
                        return Err(ContentGateError::DanglingForce {
                            move_kind: m.id,
                            effect: fid,
                        })
                    }
                    Some(def) if !def.kind.is_affordance() => {
                        return Err(ContentGateError::NonAffordance {
                            move_kind: m.id,
                            effect: fid,
                        })
                    }
                    Some(_) => {}
                }
            }
            // Every expected reply resolves in the registry.
            for &exp in &m.expects {
                if self.move_kind(exp).is_none() {
                    return Err(ContentGateError::DanglingExpect {
                        move_kind: m.id,
                        expected: exp,
                    });
                }
            }
            // Every felicity condition names both a dimension and a band.
            for c in &m.felicity {
                if c.dimension.is_empty() {
                    return Err(ContentGateError::MalformedFelicity {
                        move_kind: m.id,
                        reason: "empty dimension",
                    });
                }
                if c.band.is_empty() {
                    return Err(ContentGateError::MalformedFelicity {
                        move_kind: m.id,
                        reason: "empty band key",
                    });
                }
            }
        }
        Ok(())
    }
}

/// A handle into state the engine already holds, the content a move refers to. Never
/// authored text: a move points at a belief question, an inference frame the speaker holds
/// open, a concept-and-form pairing, or a prior move. A reference discriminator, mechanism
/// rather than world content, on the same footing as [`ForceKind`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentRef {
    /// A belief question (a subject and an attribute): the topic of an assertion or doubt.
    Belief {
        /// The subject the belief is about.
        subject: StableId,
        /// The attribute the belief is about.
        attr: AttrKindId,
    },
    /// An open inference frame over a question: the topic of a raised inquiry (9.13).
    Inquiry {
        /// The subject the inquiry is about.
        subject: StableId,
        /// The attribute being inquired into.
        attr: AttrKindId,
    },
    /// A concept-and-form pairing: the topic of a naming-game proposal (33.1, 33.9).
    Concept {
        /// The concept whose form is proposed.
        concept: ConceptId,
    },
    /// A prior move: the topic of an uptake, doubt, or repair pointing back along the
    /// adjacency.
    PriorMove {
        /// The event id of the move being responded to.
        event: EventId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor() -> ForceFloor {
        // The canonical starter floor: one primitive per affordance. Membership is data;
        // no magnitude is attached, since the floor carries affordances only.
        ForceFloor {
            effects: vec![
                ForceEffectDef {
                    id: ForceEffectId(1),
                    kind: ForceKind::TellEvidence,
                    sign: EffectSign::Neutral,
                    name: "assert".to_string(),
                },
                ForceEffectDef {
                    id: ForceEffectId(2),
                    kind: ForceKind::RaiseInquiry,
                    sign: EffectSign::Neutral,
                    name: "ask".to_string(),
                },
                ForceEffectDef {
                    id: ForceEffectId(3),
                    kind: ForceKind::RegisterUptake,
                    sign: EffectSign::Positive,
                    name: "accept".to_string(),
                },
                ForceEffectDef {
                    id: ForceEffectId(4),
                    kind: ForceKind::RegisterUptake,
                    sign: EffectSign::Negative,
                    name: "refuse".to_string(),
                },
            ],
        }
    }

    fn registry() -> MoveRegistry {
        // A starter repertoire composed from the floor: an assertion expecting an
        // acceptance or refusal, a question expecting an assertion, and the two replies.
        MoveRegistry {
            moves: vec![
                MoveKindDef {
                    id: MoveKindId(1),
                    name: "assertion".to_string(),
                    force: vec![ForceEffectId(1)],
                    expects: vec![MoveKindId(3), MoveKindId(4)],
                    sincerity_judged: true,
                    felicity: vec![],
                    gloss: "tells that".to_string(),
                },
                MoveKindDef {
                    id: MoveKindId(2),
                    name: "question".to_string(),
                    force: vec![ForceEffectId(2)],
                    expects: vec![MoveKindId(1)],
                    sincerity_judged: false,
                    felicity: vec![],
                    gloss: "asks whether".to_string(),
                },
                MoveKindDef {
                    id: MoveKindId(3),
                    name: "acceptance".to_string(),
                    force: vec![ForceEffectId(3)],
                    expects: vec![],
                    sincerity_judged: false,
                    felicity: vec![],
                    gloss: "agrees".to_string(),
                },
                MoveKindDef {
                    id: MoveKindId(4),
                    name: "refusal".to_string(),
                    force: vec![ForceEffectId(4)],
                    expects: vec![],
                    sincerity_judged: false,
                    felicity: vec![],
                    gloss: "declines".to_string(),
                },
            ],
        }
    }

    #[test]
    fn the_floor_round_trips_through_toml() {
        let original = floor();
        let text = original.to_toml_string().unwrap();
        let reloaded = ForceFloor::from_toml_str(&text).unwrap();
        assert_eq!(
            original, reloaded,
            "the force floor survived a TOML round trip"
        );
        assert_eq!(
            reloaded.effect(ForceEffectId(3)).unwrap().sign,
            EffectSign::Positive
        );
        assert_eq!(
            reloaded.effect(ForceEffectId(1)).unwrap().kind,
            ForceKind::TellEvidence
        );
    }

    #[test]
    fn the_registry_round_trips_through_toml() {
        let original = registry();
        let text = original.to_toml_string().unwrap();
        let reloaded = MoveRegistry::from_toml_str(&text).unwrap();
        assert_eq!(
            original, reloaded,
            "the move registry survived a TOML round trip"
        );
        assert_eq!(reloaded.move_kind(MoveKindId(1)).unwrap().name, "assertion");
        assert!(reloaded.move_kind(MoveKindId(1)).unwrap().sincerity_judged);
    }

    #[test]
    fn an_unknown_force_kind_is_refused_at_parse() {
        // The structural content gate for force: only recognised affordances are
        // expressible, so a primitive naming an unknown kind fails to load.
        let toml = r#"
[[effects]]
id = 1
kind = "set_persuasion_strength"
"#;
        assert!(
            ForceFloor::from_toml_str(toml).is_err(),
            "an outcome-typed force primitive cannot be loaded"
        );
    }

    #[test]
    fn a_clean_substrate_passes_the_content_gate() {
        assert_eq!(registry().content_gate(&floor()), Ok(()));
    }

    #[test]
    fn the_content_gate_refuses_a_dangling_force_reference() {
        let mut reg = registry();
        reg.moves[0].force.push(ForceEffectId(99));
        assert_eq!(
            reg.content_gate(&floor()),
            Err(ContentGateError::DanglingForce {
                move_kind: MoveKindId(1),
                effect: ForceEffectId(99),
            })
        );
    }

    #[test]
    fn the_content_gate_refuses_a_dangling_adjacency() {
        let mut reg = registry();
        reg.moves[1].expects.push(MoveKindId(99));
        assert_eq!(
            reg.content_gate(&floor()),
            Err(ContentGateError::DanglingExpect {
                move_kind: MoveKindId(2),
                expected: MoveKindId(99),
            })
        );
    }

    #[test]
    fn the_content_gate_refuses_a_felicity_condition_with_no_band() {
        // A felicity condition gates; one missing its band names no reserved bounds to
        // read, so it is malformed and refused rather than silently misfiring forever.
        let mut reg = registry();
        reg.moves[0].felicity.push(FelicityCond {
            dimension: "role.command".to_string(),
            band: String::new(),
        });
        assert_eq!(
            reg.content_gate(&floor()),
            Err(ContentGateError::MalformedFelicity {
                move_kind: MoveKindId(1),
                reason: "empty band key",
            })
        );
    }

    #[test]
    fn the_content_gate_refuses_duplicate_ids() {
        let mut reg = registry();
        reg.moves.push(MoveKindDef {
            id: MoveKindId(1),
            name: "twin".to_string(),
            force: vec![],
            expects: vec![],
            sincerity_judged: false,
            felicity: vec![],
            gloss: String::new(),
        });
        assert_eq!(
            reg.content_gate(&floor()),
            Err(ContentGateError::DuplicateMoveId(MoveKindId(1)))
        );
    }

    #[test]
    fn felicity_gates_pass_and_fail_and_misfire() {
        // A command lands only when the speaker holds a commanding role: a band over the
        // role dimension. The gate returns a boolean and never a magnitude. The band
        // bounds here are a labelled fixture, standing in for the reserved manifest value.
        let command = MoveKindDef {
            id: MoveKindId(5),
            name: "command".to_string(),
            force: vec![ForceEffectId(1)],
            expects: vec![],
            sincerity_judged: false,
            felicity: vec![FelicityCond {
                dimension: "role.command".to_string(),
                band: "felicity.command.role".to_string(),
            }],
            gloss: "orders".to_string(),
        };
        let band = ResolvedBand {
            lo: Fixed::ONE,
            hi: Fixed::from_int(10),
        };
        let resolve = |k: &str| (k == "felicity.command.role").then_some(band);
        // Holds the role: felicitous.
        assert!(command.felicitous(
            |d| (d == "role.command").then(|| Fixed::from_int(3)),
            resolve
        ));
        // Below the band: misfires.
        assert!(!command.felicitous(|d| (d == "role.command").then_some(Fixed::ZERO), resolve));
        // Dimension unreadable: fails closed, the move misfires as a bare attempt.
        assert!(!command.felicitous(|_| None, resolve));
        // Band still reserved (unresolvable): also fails closed.
        assert!(!command.felicitous(
            |d| (d == "role.command").then(|| Fixed::from_int(3)),
            |_| None
        ));
    }

    #[test]
    fn a_move_with_no_felicity_conditions_always_lands() {
        let m = &registry().moves[0];
        assert!(
            m.felicitous(|_| None, |_| None),
            "an unconditioned move is always felicitous"
        );
    }

    #[test]
    fn a_resolved_band_reads_the_manifest_and_fails_loud_while_reserved() {
        let set = r#"
[[reserved]]
id = "felicity.command.role.lo"
basis = "the role scale of Part 36, read not weighted"
status = "set"
value = "1"
source = "Part 9.5"
[[reserved]]
id = "felicity.command.role.hi"
basis = "the role scale of Part 36, read not weighted"
status = "set"
value = "10"
source = "Part 9.5"
"#;
        let m = CalibrationManifest::from_toml_str(set).unwrap();
        let b = ResolvedBand::from_manifest(&m, "felicity.command.role").unwrap();
        assert_eq!(b.lo, Fixed::ONE);
        assert_eq!(b.hi, Fixed::from_int(10));
        assert!(b.is_well_formed());

        let reserved = r#"
[[reserved]]
id = "felicity.command.role.lo"
basis = "the role scale of Part 36"
status = "reserved"
source = "Part 9.5"
[[reserved]]
id = "felicity.command.role.hi"
basis = "the role scale of Part 36"
status = "reserved"
source = "Part 9.5"
"#;
        let m = CalibrationManifest::from_toml_str(reserved).unwrap();
        assert!(
            ResolvedBand::from_manifest(&m, "felicity.command.role").is_err(),
            "an unset felicity band fails loud rather than running on a default"
        );
    }

    #[test]
    fn every_force_kind_is_an_affordance() {
        // The etic floor carries affordances only; no variant is a graded outcome.
        for k in [
            ForceKind::TellEvidence,
            ForceKind::RaiseInquiry,
            ForceKind::ProposeForm,
            ForceKind::RegisterUptake,
            ForceKind::WriteIntent,
            ForceKind::OpenContact,
        ] {
            assert!(k.is_affordance());
        }
    }
}
