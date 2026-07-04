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
//!   can fire. Its *entries* are data (which primitives a world includes, and in what
//!   order), but each entry names a [`ForceKind`], and the *kinds* are a fixed mechanism
//!   enum, exactly the two-layer shape [`crate::tom`] uses (the [`crate::tom::EvidenceOrder`]
//!   discriminator beside the data `AccessChannelRegistry`). [`ForceKind`] is the affordance
//!   discriminator, not a catalogue of world content, because each variant is a call into a
//!   mechanism the engine already has (a told-evidence facet 9.5, an inquiry goal 9.13, a
//!   naming-game form proposal 33.9, an uptake, a conditional intent 37, a contact); the
//!   kinds grow only when the engine resolves a new mechanism, never from world data. A
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
use crate::tom::AccessChannelId;
use civsim_core::{Event, EventId, EventKindId, EventLog, Fixed, StableId};
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

/// A labelled DEVELOPMENT FIXTURE modelled-dialogue substrate (base-level liveliness promotion policy):
/// the minimal assertion / acceptance / refusal move set over a three-primitive force floor
/// (tell-evidence, register-uptake positive, register-uptake negative), with no felicity conditions, so a
/// promoted mind can assert a committed belief and a promoted partner can accept or refuse it. Not owner
/// canon (a canonical dialogue substrate is data, design Part 9.5); this lets the promotion policy run
/// end to end in the dev harness so a promoted being converses rather than being silenced (skipped by
/// gossip with no dialogue substrate to converse through). Passes the content gate.
pub fn dev_substrate() -> (ForceFloor, MoveRegistry) {
    let floor = ForceFloor {
        effects: vec![
            ForceEffectDef {
                id: ForceEffectId(1),
                kind: ForceKind::TellEvidence,
                sign: EffectSign::Neutral,
                name: "assert".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(2),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Positive,
                name: "accept".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(3),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Negative,
                name: "refuse".to_string(),
            },
        ],
    };
    let registry = MoveRegistry {
        moves: vec![
            MoveKindDef {
                id: MoveKindId(1),
                name: "assertion".to_string(),
                force: vec![ForceEffectId(1)],
                expects: vec![MoveKindId(2), MoveKindId(3)],
                sincerity_judged: true,
                felicity: vec![],
                gloss: "tells".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(2),
                name: "acceptance".to_string(),
                force: vec![ForceEffectId(2)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "agrees".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(3),
                name: "refusal".to_string(),
                force: vec![ForceEffectId(3)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "doubts".to_string(),
            },
        ],
    };
    (floor, registry)
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

    /// The first move kind whose force bundle realises the given affordance (and the given
    /// sign, when one is required), in registry order. This is how the engine picks a move
    /// to carry an intent: by the affordance it realises, never by its label, which is what
    /// keeps move selection content-blind (the Part 41 invariant). Returns `None` if no
    /// move kind in the registry realises the affordance. A move kind whose force names an
    /// effect absent from the floor is skipped rather than matched, so an ungated registry
    /// still selects safely; run [`MoveRegistry::content_gate`] at load to refuse such data.
    pub fn first_realizing(
        &self,
        floor: &ForceFloor,
        kind: ForceKind,
        sign: Option<EffectSign>,
    ) -> Option<MoveKindId> {
        self.moves
            .iter()
            .find(|m| {
                m.force.iter().any(|fid| match floor.effect(*fid) {
                    Some(def) => def.kind == kind && sign.is_none_or(|s| def.sign == s),
                    None => false,
                })
            })
            .map(|m| m.id)
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

impl ContentRef {
    /// The entities this content references as subjects, so a move event can index its
    /// topic in the provenance index (a belief or inquiry is about a subject; a concept
    /// proposal and a reply reference no world subject of their own).
    pub fn topic_subjects(&self) -> Vec<StableId> {
        match self {
            ContentRef::Belief { subject, .. } | ContentRef::Inquiry { subject, .. } => {
                vec![*subject]
            }
            ContentRef::Concept { .. } | ContentRef::PriorMove { .. } => Vec::new(),
        }
    }
}

/// The registered event-schema id under which a dialogue move is logged. A single kind
/// carries every move; the move's force is a registry id in the payload, not a closed
/// `EventKind` variant, so the encoding stands under either resolution of R-EVENT
/// (design Part 9.5, Part 7.1). The value is a registered schema identifier, mechanism on
/// the same footing as the [`civsim_core::Phase`] ids, not world content, and is fixed
/// when the R-EVENT event-kind registry lands.
pub const MOVE_EVENT_KIND: EventKindId = EventKindId(0xD1A);

/// The payload schema version, so a future move-schema change is detectable rather than
/// silently misread.
const MOVE_PAYLOAD_VERSION: u8 = 1;

/// The absent-reference sentinel for an optional event id in the payload, matching the
/// degrade-sentinel convention of the draw-keying schema.
const NO_REPLY: u64 = u64::MAX;

/// A dialogue move: a communicative act recorded as a first-class canonical event (design
/// Part 9.5). It carries the force it realises (a registry id), who spoke, whom it
/// addresses, the content it refers to (a handle into existing state, never authored
/// text), the move it answers if any, the channel it travels (33.3), and its place in the
/// canonical order (tick and a per-tick ordinal). The move is the unit the conversation
/// query reassembles into a conversation; nothing here stores a conversation object.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Move {
    /// The recognised move kind this act realises (a registry id, in the payload).
    pub force: MoveKindId,
    /// Who spoke.
    pub speaker: StableId,
    /// Whom the move addresses, in stable-id order (the determinism pin of Part 9.5).
    pub addressees: Vec<StableId>,
    /// What the move is about: a handle into state the engine already holds.
    pub content: ContentRef,
    /// The move this one answers, if it is a response (the in-reply-to adjacency).
    pub in_reply_to: Option<EventId>,
    /// The access channel the move travels (witnessed-medium spoken, signed, written; 33.3).
    pub channel: AccessChannelId,
    /// The tick the move was made.
    pub tick: u64,
    /// The per-tick move ordinal that, with the speaker id, totally orders same-tick moves.
    pub ordinal: u32,
}

impl Move {
    /// Encode the move as a canonical event row. The speaker is the sole actor; the
    /// addressees and the content's topic subjects are the subjects, so the provenance
    /// index finds the move for every participant and topic. Everything needed to
    /// reconstruct the move rides in the payload as fixed-width big-endian integers, so
    /// the bytes are canonical and platform-independent (no float, no hashing).
    pub fn to_event(&self) -> Event {
        let mut subjects = self.addressees.clone();
        subjects.extend(self.content.topic_subjects());
        let mut e = Event::new(self.tick, MOVE_EVENT_KIND, vec![self.speaker], subjects);
        e.payload = self.encode_payload();
        e
    }

    fn encode_payload(&self) -> Vec<u8> {
        let mut p = Vec::new();
        p.push(MOVE_PAYLOAD_VERSION);
        p.extend_from_slice(&self.force.0.to_be_bytes());
        p.extend_from_slice(&self.channel.0.to_be_bytes());
        p.extend_from_slice(&self.ordinal.to_be_bytes());
        let reply = self.in_reply_to.map(|e| e.0).unwrap_or(NO_REPLY);
        p.extend_from_slice(&reply.to_be_bytes());
        encode_content(&self.content, &mut p);
        p.extend_from_slice(&(self.addressees.len() as u32).to_be_bytes());
        for a in &self.addressees {
            p.extend_from_slice(&a.0.to_be_bytes());
        }
        p
    }

    /// Reconstruct a move from a logged event, or `None` if the event is not a dialogue
    /// move or its payload is malformed. Total over arbitrary bytes, so a corrupt or
    /// foreign event never panics the conversation query.
    pub fn from_event(e: &Event) -> Option<Move> {
        if e.kind != MOVE_EVENT_KIND {
            return None;
        }
        let speaker = *e.actors.first()?;
        let mut c = Cursor::new(&e.payload);
        if c.u8()? != MOVE_PAYLOAD_VERSION {
            return None;
        }
        let force = MoveKindId(c.u32()?);
        let channel = AccessChannelId(c.u32()?);
        let ordinal = c.u32()?;
        let reply = c.u64()?;
        let in_reply_to = (reply != NO_REPLY).then_some(EventId(reply));
        let content = decode_content(&mut c)?;
        let n = c.u32()? as usize;
        let mut addressees = Vec::with_capacity(n);
        for _ in 0..n {
            addressees.push(StableId(c.u64()?));
        }
        Some(Move {
            force,
            speaker,
            addressees,
            content,
            in_reply_to,
            channel,
            tick: e.tick,
            ordinal,
        })
    }

    /// Every participant of the move: the speaker then the addressees.
    pub fn participants(&self) -> Vec<StableId> {
        let mut all = Vec::with_capacity(1 + self.addressees.len());
        all.push(self.speaker);
        all.extend_from_slice(&self.addressees);
        all
    }

    /// The base topic of the move, for the co-reference link of the conversation query.
    /// A belief or inquiry is about a question, a proposal about a concept; a reply has no
    /// base topic of its own (it connects through the in-reply-to link instead).
    fn base_topic(&self) -> Option<TopicKey> {
        match self.content {
            ContentRef::Belief { subject, attr } | ContentRef::Inquiry { subject, attr } => {
                Some(TopicKey::Question(subject, attr))
            }
            ContentRef::Concept { concept } => Some(TopicKey::Concept(concept)),
            ContentRef::PriorMove { .. } => None,
        }
    }
}

/// The base-topic token two moves compare on to count as co-referenced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TopicKey {
    Question(StableId, AttrKindId),
    Concept(ConceptId),
}

fn encode_content(c: &ContentRef, p: &mut Vec<u8>) {
    match c {
        ContentRef::Belief { subject, attr } => {
            p.push(1);
            p.extend_from_slice(&subject.0.to_be_bytes());
            p.extend_from_slice(&attr.0.to_be_bytes());
        }
        ContentRef::Inquiry { subject, attr } => {
            p.push(2);
            p.extend_from_slice(&subject.0.to_be_bytes());
            p.extend_from_slice(&attr.0.to_be_bytes());
        }
        ContentRef::Concept { concept } => {
            p.push(3);
            p.extend_from_slice(&concept.0.to_be_bytes());
        }
        ContentRef::PriorMove { event } => {
            p.push(4);
            p.extend_from_slice(&event.0.to_be_bytes());
        }
    }
}

fn decode_content(c: &mut Cursor) -> Option<ContentRef> {
    match c.u8()? {
        1 => Some(ContentRef::Belief {
            subject: StableId(c.u64()?),
            attr: AttrKindId(c.u32()?),
        }),
        2 => Some(ContentRef::Inquiry {
            subject: StableId(c.u64()?),
            attr: AttrKindId(c.u32()?),
        }),
        3 => Some(ContentRef::Concept {
            concept: ConceptId(c.u32()?),
        }),
        4 => Some(ContentRef::PriorMove {
            event: EventId(c.u64()?),
        }),
        _ => None,
    }
}

/// A bounds-checked big-endian reader over a payload slice. Every read returns `None`
/// past the end, so decoding foreign or truncated bytes fails cleanly rather than
/// panicking.
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Cursor { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u32(&mut self) -> Option<u32> {
        let arr: [u8; 4] = self.take(4)?.try_into().ok()?;
        Some(u32::from_be_bytes(arr))
    }

    fn u64(&mut self) -> Option<u64> {
        let arr: [u8; 8] = self.take(8)?.try_into().ok()?;
        Some(u64::from_be_bytes(arr))
    }
}

/// A conversation reassembled from the move log (design Part 9.5, Principle 5): the
/// move-event ids that belong together, with the participants gathered across them. A
/// conversation is not stored; it is this query over co-referenced move events, the way an
/// artifact's saga is a query over its tagged events.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Conversation {
    /// The move-event ids in the conversation, in ascending id order (deterministic).
    pub event_ids: Vec<EventId>,
    /// Every participant across the conversation, in stable-id order.
    pub participants: Vec<StableId>,
}

/// Reassemble the conversation containing a seed move from the event log. Two moves join
/// the same conversation when they fall within `window` ticks of each other and either one
/// answers the other (the in-reply-to link) or they share a participant and a base topic
/// (the co-reference link), which is exactly the participants, topic, in-reply-to, and
/// time-window the resolution names. The conversation is the connected component of the
/// seed under that relation, computed deterministically (moves processed in ascending
/// event-id order) and returned sorted. Returns `None` if the seed is not a move event.
pub fn conversation_of(log: &EventLog, seed: EventId, window: u64) -> Option<Conversation> {
    // Decode every move event once, in ascending id order (the log is append-ordered).
    let moves: Vec<(EventId, Move)> = log
        .iter()
        .filter_map(|e| Move::from_event(e).map(|m| (e.id, m)))
        .collect();
    if !moves.iter().any(|(id, _)| *id == seed) {
        return None;
    }
    let adjacent = |a: &Move, b: &Move, a_id: EventId, b_id: EventId| -> bool {
        if a.tick.abs_diff(b.tick) > window {
            return false;
        }
        if a.in_reply_to == Some(b_id) || b.in_reply_to == Some(a_id) {
            return true;
        }
        let shares_participant = a
            .participants()
            .iter()
            .any(|p| b.participants().contains(p));
        let shares_topic = match (a.base_topic(), b.base_topic()) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        };
        shares_participant && shares_topic
    };
    // Breadth-first closure from the seed, visiting in ascending id order for determinism.
    let mut in_set = vec![seed];
    let mut frontier = vec![seed];
    while let Some(cur) = frontier.pop() {
        let cur_move = &moves.iter().find(|(id, _)| *id == cur).unwrap().1;
        for (id, m) in &moves {
            if in_set.contains(id) {
                continue;
            }
            if adjacent(cur_move, m, cur, *id) {
                in_set.push(*id);
                frontier.push(*id);
            }
        }
    }
    in_set.sort();
    let mut participants: Vec<StableId> = Vec::new();
    for id in &in_set {
        let m = &moves.iter().find(|(mid, _)| mid == id).unwrap().1;
        for p in m.participants() {
            if !participants.contains(&p) {
                participants.push(p);
            }
        }
    }
    participants.sort();
    Some(Conversation {
        event_ids: in_set,
        participants,
    })
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

    #[test]
    fn first_realizing_selects_by_affordance_and_sign() {
        let (reg, fl) = (registry(), floor());
        // The assertion is the move that realises TellEvidence.
        assert_eq!(
            reg.first_realizing(&fl, ForceKind::TellEvidence, None),
            Some(MoveKindId(1))
        );
        // Sign discriminates acceptance (positive uptake) from refusal (negative).
        assert_eq!(
            reg.first_realizing(&fl, ForceKind::RegisterUptake, Some(EffectSign::Positive)),
            Some(MoveKindId(3))
        );
        assert_eq!(
            reg.first_realizing(&fl, ForceKind::RegisterUptake, Some(EffectSign::Negative)),
            Some(MoveKindId(4))
        );
        // No move realises a contact in this starter repertoire.
        assert_eq!(reg.first_realizing(&fl, ForceKind::OpenContact, None), None);
    }

    fn assertion(
        speaker: StableId,
        addressee: StableId,
        subject: StableId,
        tick: u64,
        ordinal: u32,
    ) -> Move {
        Move {
            force: MoveKindId(1),
            speaker,
            addressees: vec![addressee],
            content: ContentRef::Belief {
                subject,
                attr: AttrKindId(0),
            },
            in_reply_to: None,
            channel: AccessChannelId(3),
            tick,
            ordinal,
        }
    }

    #[test]
    fn a_move_round_trips_through_an_event() {
        // Every content shape survives the encode-append-decode round trip.
        let speaker = StableId(1);
        let cases = [
            ContentRef::Belief {
                subject: StableId(99),
                attr: AttrKindId(2),
            },
            ContentRef::Inquiry {
                subject: StableId(7),
                attr: AttrKindId(5),
            },
            ContentRef::Concept {
                concept: ConceptId(42),
            },
            ContentRef::PriorMove { event: EventId(3) },
        ];
        for content in cases {
            let m = Move {
                force: MoveKindId(2),
                speaker,
                addressees: vec![StableId(10), StableId(20)],
                content,
                in_reply_to: Some(EventId(8)),
                channel: AccessChannelId(3),
                tick: 17,
                ordinal: 4,
            };
            let mut log = EventLog::new();
            let id = log.append(m.to_event());
            let decoded = Move::from_event(log.get(id)).unwrap();
            assert_eq!(decoded, m, "the move survived the event round trip");
        }
    }

    #[test]
    fn a_move_with_no_reply_round_trips() {
        let m = assertion(StableId(1), StableId(2), StableId(99), 3, 0);
        let mut log = EventLog::new();
        let id = log.append(m.to_event());
        let decoded = Move::from_event(log.get(id)).unwrap();
        assert_eq!(decoded.in_reply_to, None);
        assert_eq!(decoded, m);
    }

    #[test]
    fn the_event_indexes_participants_and_topic() {
        // The move event references the speaker, the addressees, and the topic subject, so
        // the provenance index finds it for each (the query's keys survive R-EVENT).
        let m = assertion(StableId(1), StableId(2), StableId(99), 5, 0);
        let mut log = EventLog::new();
        log.append(m.to_event());
        assert_eq!(log.history_of(StableId(1)).count(), 1, "speaker indexed");
        assert_eq!(log.history_of(StableId(2)).count(), 1, "addressee indexed");
        assert_eq!(log.history_of(StableId(99)).count(), 1, "topic indexed");
        assert_eq!(
            log.history_of(StableId(404)).count(),
            0,
            "a stranger is not"
        );
    }

    #[test]
    fn a_foreign_or_corrupt_event_decodes_to_none() {
        // A non-move event is not a move.
        let other = Event::new(1, EventKindId(7), vec![StableId(1)], vec![]);
        assert_eq!(Move::from_event(&other), None);
        // A move-kinded event with a truncated payload fails cleanly, never panics.
        let mut truncated = Event::new(1, MOVE_EVENT_KIND, vec![StableId(1)], vec![]);
        truncated.payload = vec![MOVE_PAYLOAD_VERSION, 0, 0];
        assert_eq!(Move::from_event(&truncated), None);
    }

    #[test]
    fn the_payload_encoding_is_canonical_and_stable() {
        // The same move encodes to the same bytes every time (a pure function of the
        // move), which is what lets a logged conversation replay bit for bit.
        let m = assertion(StableId(1), StableId(2), StableId(99), 3, 0);
        assert_eq!(m.to_event().payload, m.to_event().payload);
    }

    #[test]
    fn a_conversation_gathers_a_reply_chain() {
        // An assertion, an answer to it, and an answer to that form one conversation,
        // linked by the in-reply-to chain even as the topic reference changes to the prior
        // move.
        let (a, b) = (StableId(1), StableId(2));
        let mut log = EventLog::new();
        let m0 = assertion(a, b, StableId(99), 1, 0);
        let id0 = log.append(m0.to_event());
        let m1 = Move {
            force: MoveKindId(3),
            speaker: b,
            addressees: vec![a],
            content: ContentRef::PriorMove { event: id0 },
            in_reply_to: Some(id0),
            channel: AccessChannelId(3),
            tick: 2,
            ordinal: 0,
        };
        let id1 = log.append(m1.to_event());
        let m2 = Move {
            force: MoveKindId(1),
            speaker: a,
            addressees: vec![b],
            content: ContentRef::PriorMove { event: id1 },
            in_reply_to: Some(id1),
            channel: AccessChannelId(3),
            tick: 3,
            ordinal: 0,
        };
        let id2 = log.append(m2.to_event());

        let conv = conversation_of(&log, id0, 10).unwrap();
        assert_eq!(
            conv.event_ids,
            vec![id0, id1, id2],
            "the whole chain is one talk"
        );
        assert_eq!(conv.participants, vec![a, b]);
    }

    #[test]
    fn a_conversation_links_co_reference_and_excludes_the_unrelated() {
        // Two moves by the same pair on the same topic join even without a reply link; a
        // move by a stranger pair on a different topic does not; a move past the window is
        // excluded though it shares a participant and topic.
        let (a, b, c) = (StableId(1), StableId(2), StableId(3));
        let topic = StableId(99);
        let other_topic = StableId(88);
        let mut log = EventLog::new();
        let here0 = log.append(assertion(a, b, topic, 1, 0).to_event());
        let here1 = log.append(assertion(b, a, topic, 2, 0).to_event()); // same pair, same topic
        let _elsewhere = log.append(assertion(a, c, other_topic, 2, 1).to_event()); // diff topic
        let _too_late = log.append(assertion(a, b, topic, 100, 0).to_event()); // past the window

        let conv = conversation_of(&log, here0, 5).unwrap();
        assert_eq!(
            conv.event_ids,
            vec![here0, here1],
            "only the co-referenced, in-window moves join"
        );
    }

    #[test]
    fn the_conversation_query_is_deterministic() {
        // The same log yields the same conversation every call (order-independent set,
        // sorted result).
        let (a, b) = (StableId(1), StableId(2));
        let topic = StableId(99);
        let mut log = EventLog::new();
        let seed = log.append(assertion(a, b, topic, 1, 0).to_event());
        log.append(assertion(b, a, topic, 2, 0).to_event());
        log.append(assertion(a, b, topic, 3, 0).to_event());
        assert_eq!(
            conversation_of(&log, seed, 10),
            conversation_of(&log, seed, 10)
        );
    }

    #[test]
    fn conversation_of_a_non_move_seed_is_none() {
        let mut log = EventLog::new();
        let other = log.append(Event::new(1, EventKindId(7), vec![StableId(1)], vec![]));
        assert_eq!(conversation_of(&log, other, 10), None);
    }
}
