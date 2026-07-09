# AbioticField field-kind registry: grounding, bedrock target, and the storage-model fork (Agent C opener)

Status: OPENER, doc-only, no code. This is Agent C's grounding pass and bedrock analysis for the arc that
opens the closed `AbioticField` enum into a data-defined field-kind registry. Posted for the gate's
source-verification before any code, per the standing directive (loop to bedrock, name the real build target,
gate the framing blind first). Everything below is verified against source with file:line; nothing is trusted
from memory or from the roadmap's summary.

## The gap, grounded

`crates/sim/src/environ.rs:331` defines `pub enum AbioticField { Light, Water, Soil }`. It is the field
IDENTITY an evolved abiotic source binds to (`AbioticBinding::field`, environ.rs:370). A producer's niche
evolves a SET of opaque source ids; the run caps the producer's productivity to the scarcest of its sources
(the Liebig law-of-the-minimum) and draws down any depletable located stock. The id itself is already opaque
data, carried un-interpreted from genesis (`genesis.rs:237`, doc: "The id's MEANING is never interpreted
here"). The one place the id acquires physical meaning is `EnvironFields::extract_producers`
(environ.rs:871), which looks up `registry.binding(id)` and then switches on `binding.field` in exactly two
places:

- READ (environ.rs:903-914): `match binding.field { Light => self.light[i], Soil => soil.deposit(weathering)
  + soil.mass(class), Water => self.water.at(x,y) }`.
- DEPLETE (environ.rs:945-953): `match binding.field { Light => {} /* no stock */, Soil => soil.take(class),
  Water => self.water.take(x,y) }`.

Those two matches are the ONLY functional consumers of the enum's variants. Everything else
(`insert`/`insert_available`, `earth_dev`, the viewer, the examples, genesis) constructs or passes the value
without switching on it. So the closed set of three Terran-named fields is the whole defect: a world's beings
can only ever draw energy from light, water, or a soil nutrient. A chemosynthetic redox gradient
(Venus/Europa), a geothermal flux (Europa), or a mana field (Arcanum) cannot be a data row.

This is the deferred "Arc 5 data-defined field set" the code already names: environ.rs:326-329 and the ARC5
plan (`docs/working/ARC5_SOURCE_MODEL_PLAN.md:176, 188`) both call `AbioticField` "the remaining closed enum
in the otherwise-data-defined `AbioticSourceRegistry`" and state the fix is "a new field handle plus the
environ field it reads rather than a rewrite of the extract dispatch (Principle 11)." The sibling pieces (the
source-id opacity, the `AbioticAvailability` presence bands, `available_in`) are already built and merged; this
arc is the last closed set in the stack.

## Consumer set and disjointness (verified against source)

Full enumeration of every `AbioticField` reference in the tree (`grep -rn AbioticField crates/`):

- `crates/sim/src/environ.rs`: the definition (331), the `AbioticBinding.field` field (370), the two
  constructor signatures (`insert` 412, `insert_available` 423), the `earth_dev` fixture (451/458/471), the two
  functional matches in `extract_producers` (903-914, 945-953), and two tests (1672/1679/1726/1727). ALL in my
  surface.
- `crates/sim/src/decompose.rs:114`: a doc comment ONLY, citing `AbioticField` as the precedent for the
  `ConditionSource` fixed-vocabulary-plus-data-binding boundary. No code dependency.
- `crates/sim/src/runner.rs:97,2908,3314,3580`, `genesis.rs`, `worldbuild.rs:443`, the viewer, and the
  examples reference only the registry TYPE (`AbioticSourceRegistry`) or call `extract_producers` /
  `set_abiotic_sources` / `earth_dev()`. NONE switches on the `AbioticField` variants.

Cross-surface check against the other agents' files:

- A owns perception/learning/material (`runner.rs`, `percept.rs`, `learn.rs`, `discovery.rs`, `laws.rs`,
  `material.rs`, `sensorium.rs`). `runner.rs:3580` calls `env.extract_producers(emb.soil_mut(), reg)` but
  never matches `AbioticField`. `material.rs:868` defines `SoilNutrientField`, whose PUBLIC interface
  (`deposit`/`take`/`mass` by `(cell, class)`) I only CONSUME through the `&mut` argument; I do not edit it.
- B owns affordance/composition (`affordance_percept.rs`, the discovery/controller loop). No `AbioticField`
  reference at all.

Conclusion: the entire live surface of the `AbioticField` enum is `environ.rs` alone. My edit surface is
`environ.rs`. Disjoint from A and B. No shared-file edit is needed; the soil interface is consumed, not
modified. If that changes, I raise it to the gate and it sequences.

Also verified: there is NO being-side "metabolism reader" that switches on `AbioticField` (the briefing
flagged runner.rs as a possible site). Beings graze the `ResourceField`, which `regrow_supply`
(environ.rs:1014) writes. `AbioticField` is producer-side sourcing only, entirely within environ.rs. The
concern is closed.

## Loop to bedrock

- Layer 0 (surface): the closed enum `AbioticField { Light, Water, Soil }` (environ.rs:331).
- Layer 1 (consumer): the two `match binding.field` in `extract_producers`. Each arm does up to three things:
  READ a located per-cell supply, DEPLETE a located stock (draw-down), and (soil only) DEPOSIT a weathering
  bootstrap.
- Layer 2 (what each arm touches): Light reads `self.light[i]` (a static `Vec<Fixed>`, no stock, no deplete).
  Water reads/draws `self.water` (a dynamic `ScalarField`, stepped by `step_hydrology`). Soil reads/draws/
  deposits an EXTERNAL `SoilNutrientField` (class-keyed, owned by the embodiment, passed `&mut`).
- Layer 3 (bedrock): the located stocks are HARDCODED, heterogeneously-typed members. Light/water/salt are
  intrinsic fields on the environment grid (a `Vec<Fixed>` or `ScalarField`), each stepped by its own FIXED
  physics stencil (`step_hydrology`, `step_salinity`, `step_productivity`) — these are floor subsystems, P9-
  legal authored physics. Soil is an external multi-class store. There is NOWHERE to put a NEW located field
  (mana, geothermal, redox), so a new energy source cannot be a data row today: it would need a new struct
  member AND a new match arm.

The real build target, named with file:line: a DATA-DEFINED collection of located fields on
`EnvironFields` (the struct at environ.rs:254), plus a field-kind descriptor the two matches at
environ.rs:903 and environ.rs:945 dispatch through uniformly instead of a three-arm switch. A field-kind is
then (a) a located field the world declares and (b) a binding pointing at it. The three high-leverage alien
targets (geothermal, redox gradient, mana) are all per-cell intrinsic SCALAR quantities, so all three become
pure data rows: a `ScalarField` in the collection plus a binding, no new struct member, no new match arm.

## The storage-model fork (the one design call, surfaced for the gate)

Each located field has a STORAGE MODEL. Two exist today, and they are genuinely different beasts:

1. INTRINSIC per-cell scalar (`ScalarField` / `Vec<Fixed>`): light, water, salt. Owned by `EnvironFields`,
   each stepped by a fixed physics stencil. Read is `field.at(cell)`; deplete is `field.take(cell, want)`.
2. EXTERNAL class-keyed store (`SoilNutrientField`, material.rs:868): soil. Owned by the embodiment, keyed
   by a nutrient-class string, part of the matter cycle. Read is `soil.mass(cell, class)`; deplete is
   `soil.take(cell, class, want)`; it also takes a weathering `deposit`.

The proposal keeps the STORAGE-MODEL DISPATCH (which storage shape backs a field) as a SMALL FIXED Rust enum
— a bounded engine-layer mechanism, sibling to `ConditionSource` in decompose.rs and to `ScalarField` being
a fixed representation — while the FIELD-KIND MEMBERSHIP (which energy fields a world runs, each one's physics,
each one's presence bands) is data. A new energy field that fits an existing storage model (another per-cell
scalar) is ZERO code. A field-kind needing a NOVEL storage model (neither a per-cell scalar nor a class-keyed
store) is a bounded Rust change, the same accepted cost the enum already carried and that the ARC5 plan
(lines 111-113) and decompose.rs:116-119 both name as legitimate deferred work ("a new physical quantity
needs a new reader").

The question I am surfacing for the gate, because it is emergence-critical and I must not decide it alone:

- Is the fixed storage-model enum a legitimate bounded engine mechanism (my read: yes, by the `ConditionSource`
  precedent and P11 — the MECHANISM is fixed Rust, the MEMBERSHIP is data), or is it a closed set re-authored
  one level down that would foreclose a plausible alien energy field?
- Is a per-cell scalar the honest floor representation of a located energy supply? A real alien source could be
  a directional flux, a between-cell gradient/difference (a redox gradient is literally a difference between
  adjacent cells' chemical potential), or a multi-class store like soil. My read: a gradient/flux reduces to a
  per-cell scalar as long as an upstream stencil WRITES the scalar (exactly as water/salt are written by their
  stencils), so the intrinsic-scalar model covers the three named targets; the honest limit is that a
  fundamentally novel storage shape is a bounded Rust change, stated plainly, not hidden.

I have run the section-11 input-bias smoke test on my own construction (fail-closed) and a section-10 blind
framing panel; the synthesized result is appended once verified against source. I will not write code until the
gate rules on this framing.

## Determinism and byte-neutrality plan

The Earth path (light id0, water id1, soil id2 via `earth_dev`) must stay byte-identical: the four `run_world`
pins (default 2b7e1035, full 1873c44e, discovery 4eea5d06, viability bae5a82, the FINAL state_hash of each
scenario). The intrinsic fields keep their exact representation and their stencils; the registry dispatch
reads them through the same accessors, so an Earth world's read/deplete sequence is unchanged. Light is not
folded into `state_hash` (only water, capacity, salt are, environ.rs:1085), so any representation choice for the
alien-scalar collection is invisible to the hash on an Earth run that declares no alien field. Each isolable
clean part is built byte-neutral and proven against all four pins; any behaviour-changing step states its
intended hash change with its reason. Section-9 five-lens audit before every push.

## Files

- Edit surface: `crates/sim/src/environ.rs` (the enum, the two matches, the `EnvironFields` struct, tests).
- Consumed, not edited: `crates/sim/src/material.rs` (`SoilNutrientField` interface, A's file).
- Docs: this file; a Part-62 record and audit-log block on consolidation, per CLAUDE.md workflow 5a.
