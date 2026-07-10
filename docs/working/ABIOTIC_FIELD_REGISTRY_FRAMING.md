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
  physics stencil (`step_hydrology`, `step_salinity`, `step_productivity`), these are floor subsystems, P9-
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

Each located field has a STORAGE MODEL. Two exist today, and they are materially different beasts:

1. INTRINSIC per-cell scalar (`ScalarField` / `Vec<Fixed>`): light, water, salt. Owned by `EnvironFields`,
   each stepped by a fixed physics stencil. Read is `field.at(cell)`; deplete is `field.take(cell, want)`.
2. EXTERNAL class-keyed store (`SoilNutrientField`, material.rs:868): soil. Owned by the embodiment, keyed
   by a nutrient-class string, part of the matter cycle. Read is `soil.mass(cell, class)`; deplete is
   `soil.take(cell, class, want)`; it also takes a weathering `deposit`.

The proposal keeps the STORAGE-MODEL DISPATCH (which storage shape backs a field) as a SMALL FIXED Rust enum
(a bounded engine-layer mechanism, sibling to `ConditionSource` in decompose.rs and to `ScalarField` being
a fixed representation) while the FIELD-KIND MEMBERSHIP (which energy fields a world runs, each one's physics,
each one's presence bands) is data. A new energy field that fits an existing storage model (another per-cell
scalar) is ZERO code. A field-kind needing a NOVEL storage model (neither a per-cell scalar nor a class-keyed
store) is a bounded Rust change, the same accepted cost the enum already carried and that the ARC5 plan
(lines 111-113) and decompose.rs:116-119 both name as legitimate deferred work ("a new physical quantity
needs a new reader").

The question I am surfacing for the gate, because it is emergence-critical and I must not decide it alone:

- Is the fixed storage-model enum a legitimate bounded engine mechanism (my read: yes, by the `ConditionSource`
  precedent and P11: the MECHANISM is fixed Rust, the MEMBERSHIP is data), or is it a closed set re-authored
  one level down that would foreclose a plausible alien energy field?
- Is a per-cell scalar the honest floor representation of a located energy supply? A real alien source could be
  a directional flux, a between-cell gradient/difference (a redox gradient is literally a difference between
  adjacent cells' chemical potential), or a multi-class store like soil. My read: a gradient/flux reduces to a
  per-cell scalar as long as an upstream stencil WRITES the scalar (exactly as water/salt are written by their
  stencils), so the intrinsic-scalar model covers the three named targets; the honest limit is that a
  fundamentally novel storage shape is a bounded Rust change, stated plainly, not hidden.

## Blind panel result (section-11 smoke test + section-10 panel), verified against source

I ran the section-11 input-bias smoke test on my OWN construction first (fail-closed). It surfaced exactly one
risk: "does every plausible alien energy field reduce to a per-cell located scalar?" I flagged it, then
UNDER-WEIGHTED it by asserting the three named targets all fit the intrinsic-scalar model. The section-10 blind
panel (which cannot see my rationalization) then nailed that under-weighted risk as the actual defect. The
smoke test caught the scent; the blind panel caught the seam. This is the honest account.

The section-10 panel: six diverse panelists (three agent types, three models); five returned verdicts (one
dropped on a model safeguard flag, not a substantive result). CONVERGENT: 4x significant-flaw-fixable, 1x
minor-improvement, none sound-as-is, none reframe-needed. The core (opaque field-kind id, data-defined field
collection, Liebig-min/deplete/deposit mechanism fixed in Rust, membership + physics + presence bands as data)
is unanimously sound and a real improvement. The seam is NOT the storage-model enum I invited them to attack.

THE VERIFIED FINDING: the uniform interface `read/deplete-AT-CELL` authors POINT-LOCALITY as the definition of
an energy supply. Verified against source: the read arms (environ.rs:903-914) all read a per-cell scalar or
class-keyed store at the producer's OWN cell, and the deplete arms (environ.rs:945-953) draw down only the
producer's own cell (single-location, no cross-cell coupling). My "a redox gradient is a per-cell scalar" claim
is false by the redox gradient's own definition: a chemolithotroph draws power from a POTENTIAL DIFFERENCE
between an electron donor (reduced, e.g. H2S) and an acceptor (oxidized, e.g. O2), a between-quantity (and, at
a vent, between-location) difference, not a value at one cell. Point-locality is a fixed Terran-shaped category
that a gradient-, flux-, or difference-fed being cannot be expressed against as DATA (admit-the-alien / P8).

Source-verified nuance that sharpens the finding: the CORE already unlocks part of the chemolithotroph case.
The Liebig-min over the evolved source SET (environ.rs:895-921) means a producer binding BOTH a donor
field-kind AND an acceptor field-kind is capped by the scarcer, which IS reactant co-limitation (you cannot
react faster than the scarcer reactant allows), a pure data row under the core. What the core truly CANNOT
express, and what the panel correctly exposes, is (a) thermodynamic YIELD as a pairwise DIFFERENCE (delta-E,
which is a subtraction, not a minimum), and (b) a between-cell spatial GRADIENT or directional flux.

THE CORRECTED FRAMING (my synthesis of the five, source-verified). Keep the accepted core unchanged. Replace
the single hardcoded point-read with a supply QUERY generalized off the per-cell-stock ontology: "given the
producer's location and a field-kind's OWN declared state, yield the available draw and apply the
deplete/deposit," so the Liebig-min mechanism never learns which spatial shape it consulted. A field-kind's
binding declares, as DATA, HOW that query is computed, via two bounded, orthogonal engine dimensions, each a
P9-legitimate physics-floor law operator (the same class the engine already uses to STEP fields and to
Liebig-min the source set), never a high-level or Terran fact:

- a VALUE BACKING (storage model: intrinsic per-cell scalar; external class-keyed store), documented as
  implemented-not-exhaustive (an environment-owned nodal/graph-keyed backing for leyline nodes or ore veins is a
  foreseeable near-term addition, gp-sonnet's unsampled quadrant), never asserted closed; and
- a READ-SHAPE / ARITY+COMBINATOR (point value; finite-difference/gradient over a declared neighbourhood;
  directional projection; pairwise-DIFFERENCE over an ordered list of (field, role) pairs; joint presence),
  the data-selected spatial/combinator operator that was the actual hidden foreclosure.

Then a redox chemolithotroph is a DATA ROW: [donor field, acceptor field] + a pairwise-difference combinator
(both per-cell scalars, the honest two-quantity chemistry, no fudged single "redox field", no neighbourhood
access) for the thermodynamic yield; or a redox-potential field + a finite-difference read-shape for a true
between-cell spatial gradient. The world declares which. Zero Rust; the alien is data. The point-local read is
ONE operator among the set, never THE interface.

THE ACCEPTANCE GATE (Plan-opus, adopted): the arc is not done until a deep-vent chemolithotroph fed by a redox
potential DIFFERENCE is demonstrated as a pure data row (field-kind rows + binding, zero Rust). If it still
needs a Rust change, the interface is still authoring point-locality and the enum has not earned its place.

HONEST LIMIT, kept not hidden: the value-backing set and the read-shape/combinator vocabulary are bounded
engine mechanisms; a truly novel one (a new storage topology, a new spatial operator) is a bounded, rare,
accepted Rust cost, the SAME cost `ConditionSource` carries, but the sets are documented implemented-not-
exhaustive, never asserted closed.

This is a SCOPE EXPANSION over my opener (the interface generalization plus a read-shape/combinator vocabulary,
not only the field-kind collection) and it carries a mechanism fork (read-shape spatial operator vs
arity+combinator multi-field vs both). It is still incrementally byte-neutral and keeps `extract_producers`'s
signature stable. Surfaced to the gate for the formal ruling and the scope call. I will not write code until the
gate rules.

## Determinism and byte-neutrality plan

The Earth path (light id0, water id1, soil id2 via `earth_dev`) must stay byte-identical: the four `run_world`
pins (default 2b7e1035, full 1873c44e, discovery 4eea5d06, viability bae5a82, the FINAL state_hash of each
scenario). The intrinsic fields keep their exact representation and their stencils; the registry dispatch
reads them through the same accessors, so an Earth world's read/deplete sequence is unchanged. Light is not
folded into `state_hash` (only water, capacity, salt are, environ.rs:1085), so any representation choice for the
alien-scalar collection is invisible to the hash on an Earth run that declares no alien field. Each isolable
clean part is built byte-neutral and proven against all four pins; any behaviour-changing step states its
intended hash change with its reason. Section-9 five-lens audit before every push.

## Segment 2 blind framing panel (verified): a reframe the gate must rule on

Segment 1 is signed off and merged-rebased (pins re-verified on the merged tree). Before writing segment 2, I
ran the section-10 blind framing panel on the pairwise-difference construction (the one segment 1's own panel
proposed and the gate pre-approved). Six diverse panelists; CONVERGENT (1 reframe-needed, 5
significant-flaw-fixable, none sound). The finding, verified against source: the "pairwise-difference operator on
two concentration fields" framing itself authors three things a world must decide.

1. STOICHIOMETRY authored 1:1. Draining each participating field through the same global `draw_fraction` and
   `biomass_per_stock` hardcodes a 1:1 donor:acceptor consumption. Real redox is reaction-specific and can be
   multi-participant. Fix: a per-(field, role) stoichiometric coefficient as world data on the binding.
2. The stock->biomass conversion `biomass_per_stock` is a single registry-global whose basis is the reciprocal
   of a SOIL fertility scale (environ.rs), applied to a dimensionally incommensurable redox stock (joules of
   free energy versus moles of nutrient). Fix: per-source conversion, grounded in that field's own floor units.
3. A bare subtraction of two concentration fields is neither the free-energy yield (Nernst-corrected) nor the
   total available power (co-limitation by the scarcer reactant). The strongest panelist's decisive point,
   confirmed against source: co-limitation is ALREADY the engine's Liebig-minimum over the source set, which
   segment 1 delivers (a producer binding {donor, acceptor} field-kinds is capped by the scarcer, the
   `productivity_is_the_liebig_minimum_over_the_evolved_source_set` test); the redox-specific thing is the YIELD,
   and the floor ALREADY carries `law.battery_emf` (EMF = E_cathode - E_anode over `chem.standard_potential`),
   so the yield DERIVES from the floor, not a hardcoded operator.

The verified reframing. A redox chemolithotroph decomposes into: (a) co-limitation = existing Liebig-min
(segment 1, zero new code); (b) thermodynamic yield = derived from the floor `law.battery_emf` over the
participants' `chem.standard_potential`, with the EMF-to-biomass coupling a reserved value surfaced with basis
(a thermodynamic-efficiency bound), never fabricated; (c) reaction-specific stoichiometry = per-(field, role)
coefficients as world data; (d) stock-to-biomass conversion = per-source, not a soil-derived global. The bespoke
"difference operator on concentration fields" is dropped: it reinvents co-limitation wrong and misplaces the
redox character, which belongs in the yield.

This is a design-intent fork for the gate and, where it touches the floor coupling and modeling depth, the
owner: (i) the EMF-to-biomass coupling reserved value (owner's, derived against a floor efficiency bound); (ii)
modeling depth (the standard EMF as a per-source constant versus a full Nernst concentration-dependent yield);
(iii) confirmation to drop the bespoke difference operator in favour of the Liebig-min-plus-floor-yield form.
Unblocked and buildable byte-neutral regardless of the fork (correct improvements the audit and panel both name):
the per-source stock-to-biomass conversion and the per-(field, role) stoichiometric coefficients. Surfaced to the
gate; holding at doc-only for segment 2 until it rules.

## Segment 2 built (gate signed off the reframe)

The gate confirmed the reframe, dropped the bespoke difference operator, and signed off segment 2 as the two
byte-neutral parts, holding the two owner-register items (the EMF-to-biomass coupling reserved value; the
standard-EMF versus full-Nernst modeling depth) for segment 3. Built: `AbioticBinding` gained
`biomass_per_stock: Option<Fixed>` (the per-source cap conversion) and `stock_per_biomass: Option<Fixed>` (the
per-source stoichiometric drawdown), both defaulting to the registry-global so every Terran binding is
byte-identical. In the reframe each reactant is a separate source id already Liebig-min co-limited by segment 1,
so there is no role enum: "per-(field, role)" collapses to per-binding. `extract_producers` Pass 1 caps by the
per-source-or-global conversion and Pass 2 draws each field by its own coefficient (the `None` stoichiometry
falls back to the reciprocal of the effective conversion, today's implicit 1:1). A new `set_source_conversion`
arms them and fails loud on a per-source conversion of zero (symmetric with the global's unset guard; a zero
stoichiometry stays legal, the catalyst case).

Proven: the four pins hold byte-identical, `extract_producers`'s signature is unchanged, worker-invariant, 22
environ tests (four new: per-source conversion overrides the global; a donor+acceptor pair is Liebig co-limited
by the scarcer AND drawn 2:1 by its own stoichiometry, a redox chemolithotroph as a pure data row; the Pass-2
draw uses the per-source conversion; a zero conversion fails loud), full sim suite green, fmt and clippy clean.

Section-9 five-lens audit: thirteen clean-notes confirming byte-neutrality (bit-identical under the None
defaults), the value-authoring line (no hardcoded per-source number in the content path), the reframe
correctness (co-limitation is the existing Liebig-minimum, non-1:1 drawdown per source), and no panic or overflow
(including a per-source `Some(0)` and a zero conversion). Findings hardened before push: the untested Pass-2
per-source branch got a test; the zero per-source conversion now fails loud; `draw_fraction` staying global is
named as the honest remaining limit; and the one confirmed minor, that an alien `DataScalar` omitting its
conversion silently borrows the soil-derived global (opt-out, not closed), is now surfaced as an explicit honest
limit at its site, closed when the source declares its own conversion or segment 3 derives it from the floor.

## Segment 3 core built (the gate's interim unblock)

The gate unblocked the depth-independent floor-EMF yield core (the base both standard-EMF and full-Nernst
share) while the owner decides the depth. Built: a redox source's stock-to-biomass conversion DERIVES from its
couple rather than a declared number. `RedoxEmf { donor_potential, acceptor_potential }` carries the two floor
`chem.standard_potential` values; `AbioticSourceRegistry::effective_conversion` computes the galvanic EMF through
the floor law `civsim_physics::laws::battery_emf` (`E_acceptor - E_donor`, clamped at zero so a couple whose
standard EMF is non-positive powers no life) times the RESERVED `emf_to_biomass` coupling. Precedence: the redox
derivation over the segment-2 per-source value over the registry-global; both extract passes route through the one
`effective_conversion`, so Pass 1 and Pass 2 agree.

The `emf_to_biomass` coupling is RESERVED, the owner's value, surfaced with the `dG = -n*F*EMF` basis and never
fabricated: it defaults to a fail-loud sentinel (zero) that refuses to run a redox derivation rather than silently
starving the producer. The standard potentials are floor data. So the yield is derived from the floor and the
couple, nothing authored in the content path.

Flagged not wired, the owner's depth ruling: the full-Nernst concentration adjustment (the reaction quotient Q
from the `DataScalar` field concentrations, the `RT/nF` thermal factor) around this same core. Two honest limits
of the standard-EMF core are surfaced at the mechanism site: the zero-clamp reads spontaneity at the standard
state (Nernst would re-judge it at the actual concentrations), and the reserved coupling folds the per-couple
electron count `n` into one global (a per-couple `n` is a couple-data refinement); both are the owner's
yield-model-depth call.

Proven: the four pins hold byte-identical (`redox_emf` None makes `effective_conversion` return exactly the
segment-2 expression), `extract_producers`'s signature is unchanged, worker-invariant, 26 environ tests (four new:
the yield equals `supply * (EMF * coupling)` and overrides the global; the redox derivation takes precedence over
a per-source conversion; a standard-non-spontaneous couple and the EMF-zero boundary support no biomass with a
spontaneous positive control; an unset coupling fails loud), full sim suite green, fmt and clippy clean. A
section-9 five-lens audit was run, its findings verified against source and hardened (the precedence and
EMF-zero-boundary tests, and the standard-state and folded-`n` honest limits surfaced at the site).

## Nernst extension: blind framing found four seams in the owner's spec (verified, for the gate/owner)

The owner ruled full-Nernst and gave a derive-clean spec. Before building, I framed the Nernst kernel blind
(section-11 smoke then a six-panelist section-10 panel). All six returned significant-flaw-fixable, convergent on
four seams the owner's spec still carries, each verified against source (Prime Directive 1: do not assume the
owner is right):

1. ACTIVITY, not concentration. The real Nernst quotient is in ACTIVITIES; forming `Q` from raw concentrations
   silently sets the activity coefficient gamma to 1, the ideal-dilute AQUEOUS convention of Terran textbook
   electrochemistry, an authored value outside the floor. Fix: form `Q` from activities through a floor activity
   LAW (activity = concentration / c_standard times gamma), with gamma = 1 the EXPLICITLY-selected reserved
   default (surfaced with basis), overridable by a world activity model keyed on the medium; a brine, molten, or
   mana medium with gamma far from 1 becomes a data row.
2. `n` is a per-SUBSTANCE floor fact, not a per-source knob. The electrons transferred is a fixed chemical fact
   of the couple's substances (like the standard potential the floor already carries), not a free per-organism
   trait; parking it on the source lets two sources sharing a couple disagree. Fix: a `chem.electron_count`
   per-substance floor axis (same treatment as `chem.standard_potential`), `n` derived by charge and mass balance
   over the substances; the source carries only which substances it uses.
3. The yield drops `n` from the energy MAGNITUDE. `dG = -n*F*E`, but scaling biomass by EMF in VOLTS puts `n` in
   the Nernst term and drops it from the scale, so two couples with equal EMF but different `n` give identical
   biomass. Reusing the core's `emf_to_biomass` (whose own doc says it already folds `n*F`) while ALSO putting `n`
   in the Nernst term DOUBLE-COUNTS. Fix: energy = `n * F * max(0, E_nernst)`, `n` consistent in both places, F a
   new reserved CODATA constant (the existing `gas_constant` is the SPECIFIC `R_s` of `ideal_gas_density`,
   laws.rs:1294, not the universal molar R, so Nernst needs a new molar R too).
4. Metabolic efficiency should EMERGE, not be authored. Biomass = energy times an efficiency (biomass per JOULE),
   and efficiency is a biological trait that should evolve per lineage under selection (P8), not a single
   reserved floor global. Fix: per-source/per-lineage world data, reserving only a thermodynamic-efficiency
   CEILING (with basis) that mutation and selection push against.

The corrected yield: biomass = efficiency_per_lineage times `n * F * max(0, E° - (R*T)/(n*F) * ln(Q_activities))`,
with `n` from `chem.electron_count` (floor, derived), R and F reserved CODATA floor constants, T the temperature
field, the activity law and its gamma reserved-and-overridable, and efficiency an evolvable per-lineage trait.

This substantially exceeds the owner's spec and grows the floor (a `chem.electron_count` axis, an activity law,
the molar R and F constants) and makes efficiency emergent. It is squarely an owner decision. Surfaced to the
gate with the recommendation; holding at doc-only for Nernst until the gate/owner rules. The T-access construction
detail (store the stepped temperature in `EnvironFields` so the frozen-signature `extract_producers` can read it,
byte-neutral) stands ready regardless of the ruling.

## CORRECTED-T3: blind framing sharpened the fix (verified), for the gate

The gate sequenced CORRECTED-T3 first and specified carrying the plant's "composition-weighted energy density".
I framed it blind. All six panelists returned significant-flaw-fixable (one minor), convergent on two points,
both verified against source:

1. Do NOT collapse to a plant-side scalar. Killing the simplex normalization is correct (it authors a flat
   "every plant equally nutritious" coupling). But reducing the plant to a single composition-weighted GROSS
   energy density authors digestibility = 1 for every consumer: gross kJ/g is a substance property; the energy a
   grazer BANKS is what ITS metabolism assimilates (cellulose is high-energy yet worthless to a gut without the
   pathway). A plant-side scalar relocates the flatness from the plant to the consumer, a template-case and
   admit-the-alien violation (a redox/mana/silicon feeder banks no kJ/g of biomass). Fix: carry the FULL
   de-normalized composition VECTOR (real per-axis magnitudes) into the standing food, and let usable food value
   emerge at consumption as the plant's composition folded against the CONSUMER's own metabolic data.
2. Reuse the existing energy bridge, do not mint a fresh anchor. A new kJ-to-intake anchor independent of the
   one that derives a body's reserve capacity from the same `bio.energy_density` axis would give two unreconciled
   scales for one quantity and break energy conservation across the eat step. Fix: reuse the existing bridge; add
   a pin only if a unit mismatch requires it, with basis EQUALITY to that bridge, never a free parameter.

Both are confirmed against source, and they make the fix SIMPLER than the spec: `physiology::physical_intake`
(physiology.rs:495) ALREADY folds the food's physical `content` against the CONSUMER's own `assim` (assimilation)
and `eta` (trophic efficiency) through the SAME size-scaled `bio.energy_density` reserve bridge the drain uses
(`content * assim * eta / (body_mass * body_storage_density)`), keyed on NO axis identity (a non-digester with
`assim <= 0` eats nothing; a thaumic reserve fills from a mana plant the same way). So the consumer-side
per-consumer emergence is already built and alien-clean. The ONLY defect is that `set_producer_food`
(environ.rs) divides the composition by its total (the simplex), discarding the magnitude so every plant presents
the same `content` per biomass volume.

The corrected fix: `set_producer_food` stops normalizing and carries the real per-axis composition magnitudes;
the existing `physical_intake` then folds the real content against each consumer's metabolism, so which plants
are worth eating (and which thrive under grazing) emerges from the plant-consumer material interaction, not a
shared nutrition number. No fresh reserved anchor (the existing reserve bridge already converts the magnitude);
a units pin is added only if a scale mismatch is found, its basis equality with that bridge. Confined to
`set_producer_food` (my surface); `physical_intake` is READ to confirm the consumer fold exists, not edited (it
is already correct and alien-clean). The four tracked pins hold byte-identical (only `--scenario living` and the
composition tests arm producer food); the stated behaviour change is that plants stop being uniformly nutritious.
Surfaced to the gate (it revises the spec toward more principled, and simpler); holding at doc-only until it rules.

## Files

- Edit surface: `crates/sim/src/environ.rs` (the enum, the two matches, the `EnvironFields` struct, tests).
- Consumed, not edited: `crates/sim/src/material.rs` (`SoilNutrientField` interface, A's file).
- Docs: this file; a Part-62 record and audit-log block on consolidation, per CLAUDE.md workflow 5a.

## CORRECTED-T3 consumption side: the anchor supersession, and the smoke test that sharpened the scale story

The seeding side (`set_producer_food` carrying real per-axis magnitudes) is signed off. The consumption side
completes T3: where a cell bears a real producer composition, its standing supply IS the physical energy content
at the plant's own `bio.energy_density`, so the forage INGEST (`locomotion.rs step_with_field_dirs`) eats it at
`content = supply` directly rather than bridging through the reserved `food_energy_density` anchor (3000). A
composition-less cell (the abstract climate-productivity default) keeps the anchor bridge, per the gate's ruling
("keep the 3000 anchor only as the fallback"). The branch is carried by a per-cell `real_composition: Vec<bool>`
marker on `ResourceField`, set each tick by `regrow_supply` from `producer_food.is_some()` and read by INGEST. The
marker is NOT hashed (it is a per-tick derived read of the static `producer_food`, recomputed by `regrow_supply`
before the graze in the scheduled order, so a restore recomputes it deterministically before use; its effect
reaches the hash through the supply values, which ARE hashed). The four pins hold byte-identical, because the four
tracked scenarios seed no producer food, so every cell stays unmarked and the INGEST branch is the same
`supply * food_energy_density` as before.

Two consumption stores were checked for a double-eat and confirmed disjoint: locomotion INGEST grazes the
`ResourceField` standing food; `runner.rs ingest_located` eats `self.material` located matter plus carried
inventory. `physiology::physical_intake` is a shared fold PRIMITIVE both call on their own store, not a second
consumption of the same food.

The section-11 input-bias smoke test on my first audit construction FAILED CLOSED, and it was right to. It caught
that I had stated the scale story too cleanly: I had written "the real scale is ~100x below the anchor, an
owner-gated recalibration of the drain." Verified against source, the precise mechanism is sharper. My change
divides a real-producer cell's `content` by the full `food_energy_density` (3000), while a composition-less
climate cell's `content` is unchanged. So in a live mixed grid (`--scenario living` seeds ~45 producer cells and
leaves the rest of the habitable grid as climate food), two regimes coexist: a marked producer cell yields
`volume * density` with `density` in the floor-declared `bio.energy_density` range [0, 38] kJ/g, and an unmarked
climate cell yields `volume * 3000`. A producer cell therefore carries far less standing food value than the
abstract climate food on the cell beside it, the branch decided by whether a producer composition was seeded. The
pre-adapted recurrent grazers, bootstrapped at the dawn and foraging the real producer occupants, collapse by
starvation (population 10 to 0 over two windows, 24 then 10 deaths, final hash 07d867a5). This is surfaced, never
tuned: whether the abstract climate fallback should persist alongside real-plant food, and how the reserve/Kleiber
drain scale reconciles with the real `bio.energy_density` intake scale (the drain side was calibrated against the
3000 anchor, and the reserve-to-joule reconciliation is itself a documented open limit), is the owner's
biosphere-BALANCE question. The mechanism is correct and byte-neutral; making the world thrive on real food values
is the calibration the `worldbuild.rs` T3 owner-gate holds.
