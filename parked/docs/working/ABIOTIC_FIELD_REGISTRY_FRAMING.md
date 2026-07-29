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

## The section-9 panel on the consumption side: a real latent scope leak, and the counterfactual that corrected the causal story

The section-9 seven-panelist audit (five mandatory lenses, a cross-axis unit-coherence lens, a correctness/determinism
lens), run behind a section-11 smoke test that failed closed four times until the packet was neutral and complete,
caught two things worth acting on. Both were verified against source before I trusted them.

First, a real latent scope leak. The per-cell `is_real_composition` marker gated EVERY homeostatic axis in the INGEST
loop, including the water axis (`bio.water_fraction`), whose supply is the drinkable [0, 1] mirror `regrow_supply`
writes on every cell and which `set_producer_food` explicitly excludes from the producer composition. So on a
producer-food cell the marker also stripped the `food_energy_density` anchor off the water axis, scaling water intake
there differently from a bare cell (by the anchor factor), a consequence the cell-level ruling did not intend: the
anchor is a food-content bridge, and water is not a producer-composition axis. The fix keys the marker PER CLASS: it
carries the set of nutrient classes the cell's `producer_food` holds (the food axes), so the supersession
applies only to those, and the water mirror and any non-composition axis keep the anchor on a producer cell exactly as
on a bare cell. Verified byte-neutral: the four pins hold bit-exact (2b7e1035 / 1873c44e / 4eea5d06 / bae5a82), and
`--scenario living` is ALSO byte-identical (final hash 07d867a5 unchanged), because the leak was unreachable in the
current scenarios (a being never drinks on a producer-food cell there, so the water axis never took the leaked branch).
The fix is a correctness hardening that removes the trap for a future scenario with wet producer cells, changing no
observed result.

Second, the counterfactual that corrected my causal claim. The confirmation-bias lens flagged that I had asserted the
`living` collapse is caused by the change without a pre-change baseline. I ran one: at the parent commit (the
de-normalized seeding in place, the consumption anchor NOT yet superseded, so every cell including producer cells is
consumed at the `food_energy_density` anchor), `living` ALSO collapses to zero, but it peaks at population 19 and dies
of MIXED thirst and starvation, where the superseded version peaks at 10 and dies of PURE starvation. So the collapse
is PRE-EXISTING (the biosphere-balance calibration the roadmap and gate already flag: body-mass scaling, metabolic
rate, or productivity), not created by this change. Because the fixed consumption side handles water identically to the
parent, the counterfactual cleanly isolates the food-axis supersession as the cause of the sharpening: removing the
food double-scale on producer cells is what turns a peak-19 mixed-death collapse into a peak-10 pure-starvation one. The
honest surfacing is therefore not "the change starves the world" but "the world was already non-viable at this body
scale, and pricing the producer food at its real derived value without the double-scale deepens the starvation
component," which is the same owner-gated biosphere-balance calibration, resolved at its true cause and never by
inflating the food value.

## Corrected Nernst: grounding (the last AbioticField piece, four seams gate-ruled in)

Surface mapped against source before framing. The redox mechanism is entirely in `environ.rs` (my surface):
`effective_conversion` (the segment-3 core) currently returns `battery_emf(acceptor, donor).max(0) * emf_to_biomass`,
a flat coupling. The corrected form is the thermodynamic yield `efficiency_lineage * n * F * max(0, E_cell)` with
`E_cell = E0 - (R*T / (n*F)) * ln(Q)` (Nernst), all four seams ruled in by the gate.

Floor state (grown per Prime Directive 6, gate-authorized): `chem.standard_potential` exists (the donor/acceptor/
cathode/anode potential, a signed volt window). MISSING and to be grown: a `chem.electron_count` per-substance axis
(the Nernst `n`, electrons transferred); a UNIVERSAL molar gas constant R (only the SPECIFIC `R_s` exists, inside
`ideal_gas_density` at laws.rs:1294, not reusable); a Faraday CONSTANT F (note `law.faraday_emf` at em_floor.toml:325
is electromagnetic INDUCTION, flux-linkage and the Lenz sign, a naming collision to avoid, NOT the electrochemical
F = 96485 C/mol); an ACTIVITY law with an activity coefficient gamma for the reaction quotient Q; and a Nernst law
kernel in `laws.rs`. Each new axis/constant/kernel reaches `PHYSICS_FLOOR_REGISTRY.md` via `gen_floor_registry.py`.

Two couplings to resolve in framing. First, TEMPERATURE: the `R*T` term needs `T` at the `effective_conversion` call
site (the productivity Pass 1/2 at environ.rs:1161/1193), which today takes only `&binding`; `T` must be threaded
(stored on `EnvironFields` or passed), byte-neutral by defaulting the redox path off (Earth declares no redox source).
Second, and emergence-critical, PER-LINEAGE EFFICIENCY: `effective_conversion` is ENVIRONMENTAL (a per-cell source
yield), not per-being, so where the evolved per-lineage efficiency enters is a real design question (an environmental
per-couple yield that is the thermodynamic maximum, with efficiency a downstream CONSUMER trait applied where a being
draws on the source; versus a per-lineage term folded into the cell yield). This wires through the genome/selection
substrate, so it MUST be framed blind (section-11 then section-10) and its surface-disjointness from Agent A
(learn/runner/evolve) and Agent B (affordance/composition) confirmed BEFORE any code. That framing is the next step;
no code until the gate rules on it.

### Nernst surface-disjointness (grounded against git history)

Two cross-surface touchpoints with Agent A, so the Nernst is NOT a solo build and needs the gate to sequence:
- `crates/physics/src/laws.rs`: the Nernst kernel and the activity law belong here beside `battery_emf` (the floor's
  law-kernel home, which `PHYSICS_FLOOR_REGISTRY.md` indexes). But this file was last touched by Agent A's #109
  perception arc (the `geometric_spread` kernel and the transduction family), so A actively adds kernels here. The
  additions are additive `pub fn`s (low textual-conflict risk if placed apart), but the file is contended and the edit
  must be declared and sequenced, not made unilaterally.
- The per-lineage EFFICIENCY trait wires through the genome/selection substrate (`evolve.rs`/`controller.rs`, the
  living recurrent-controller foundation), which is A's ground. Whether efficiency is a new genome channel, a reused
  trait, or a downstream consumer multiplier is the framing's to settle; either way the wiring touches A's substrate
  and must be coordinated.
The environ side (`effective_conversion`, the redox binding, temperature access) is mine. The floor DATA growth (the
`chem.electron_count` axis in the chem floor toml, the universal R and F constants) is floor data, to place where the
existing constants live. Surfacing this to the gate with the framing result; no edit to A's files until it sequences.

## Corrected-Nernst blind framing: the section-10 panel result (surfaced to the gate, no code until it rules)

The section-10 blind framing panel (five diverse lenses, behind a section-11 smoke test that failed closed four
times until the statement was neutral and the three efficiency architectures symmetric) returned a decisive set of
findings. The load-bearing ones are verified against source. This reshapes the Nernst well beyond the four seams
originally ruled, so it goes to the gate before any code.

Efficiency placement (the emergence-critical crux). Two lenses independently rule architecture B out: folding a
per-lineage efficiency scalar into the per-cell ENVIRONMENTAL yield makes an abiotic source's output depend on WHICH
lineage draws on it, so on a cell two lineages share the environmental field has no single value, breaking observer
independence (Principle 10) and writing a biological outcome into an abiotic field (Principle 9). Verified against
source: `effective_conversion` is per-cell environmental (feeds the cell's biomass capacity), so B is a defect, not a
neutral option. Between A (environment = thermodynamic maximum, efficiency a downstream consumer scalar) and C (no
authored scalar, throughput emerges from the being's modeled metabolic machinery), the panel ranks C over A: an
efficiency SCALAR authors the FUNCTIONAL FORM of extraction, not merely a value, since `realized = efficiency *
ceiling` imposes linear proportionality where real throughput saturates and is kinetically governed. C lets both the
value and the form emerge and supplies the intrinsic cost that A lacks. A is a defensible interim (it keeps the floor
clean and matches the existing observer-independent food-path separation, `physical_intake` folding a consumer's
own assimilation over an environmental content), but only if two defects are fixed: the efficiency cap belongs in the
floor as a conservation constant (efficiency <= 1, since efficiency > 1 manufactures energy, so it is NOT an
owner-reserved genome-range convention), and without an antagonistic cost a free efficiency good ramps under selection
straight to its bound, so the realized value is just the authored ceiling.

Physics and floor-integrity seams, verified against source. The Nernst core computes an ENERGY (n*F*E, joules), but
`effective_conversion` returns a biomass-per-stock, so the energy-to-biomass stoichiometric bridge (the current
`emf_to_biomass`) does NOT go away: the design's premise that n*F*E replaces it is wrong, and a carbon-fixation-style
energy-to-biomass conversion is still required. The RT/nF term reduces algebraically to (k_B/e)(T/n), so the
Avogadro/mole convention folded into R and F individually cancels: the floor already carries the elementary charge
(`elec.charge`, unit C) but not Boltzmann k_B, so deriving from k_B and the carrier charge is cleaner and more general
than authoring both a molar R and a Faraday F (and it avoids hardcoding the Terran electron as the charge carrier).
The Nernst potential is an EQUILIBRIUM state function (zero net current), so multiplying it by n*F with no kinetic or
rate-limiting term (exchange current, catalytic turnover, diffusion flux) omits what sets a per-tick yield.
E0 (`battery_emf`) is treated as temperature-invariant, omitting dE0/dT, the real temperature dependence of the
standard potential itself. The per-couple `max(0, E_cell)` clamp forecloses emergent energy COUPLING (a lineage
running a thermodynamically uphill couple by coupling it to a downhill one, which real chemolithotrophs do): the
correct clamp is on the NET free energy across coupled reactions, so the defect is the per-couple GRANULARITY, not the
clamp itself. The activity coefficient gamma defaulting to 1 silently bakes the Terran ideal-dilute-aqueous reference
state into every world, and the activity law is not shown to be a data-defined extensible registry (the generalization
seam the resolution workflow exists to catch). The existing `law.faraday_emf` (electromagnetic induction) and a new
electrochemical Faraday would collide on the name (a floor-integrity hygiene risk), and the existing per-substance R_s
should derive from a universal R and molar mass rather than stand as its own authored value (a prior floor seam this
exposes). Determinism: the cross-tick stock-depletion-to-Q feedback and the temperature-field read both have
field-step-order hazards with no specified deterministic resolution protocol, and ln(Q) diverges to minus infinity as
Q approaches zero (a normal early-reaction state), overflowing E_cell before the clamp, so the guard must bound it.

Net: the corrected Nernst is a larger design than the four seams ruled. The gate and owner decisions surfaced: the
efficiency architecture (C emergent-machinery versus A downstream-scalar-with-floor-cap-and-cost, B ruled out); the
energy-to-biomass bridge stays as a stoichiometric conversion; whether to derive the thermodynamic term from k_B and
the carrier charge rather than authoring R and F; whether to add a kinetic rate term; the coupling GRANULARITY of the
clamp; the gamma registry and its non-Terran default; dE0/dT; and the determinism protocols. Surfaced, not decided;
no code until the gate rules.

## Arc 1 (uptake-flux law): blind framing result, and the corrections it made to the third-form spec

The section-10 blind framing panel (five lenses, behind a section-11 smoke that cleared) was unanimous and decisive,
and it corrected the owner-ruled third-form spec itself on several load-bearing points (Prime Directive 2, audit the
input including the spec). The findings are verified against source or math.

The conservation claim in the spec is mathematically false. The spec said conservation is STRUCTURAL, v <= S by
construction, so the <= 1 cap is dropped. Verified: v = Vmax*S/(Km+S) < S holds only when Vmax < Km+S; at low supply
v is about (Vmax/Km)*S with slope Vmax/Km, which exceeds one whenever Vmax > Km, and since Vmax = kcat*catalyst-tissue
grows without bound as the producer's catalytic axis grows, a high-catalyst producer on a low-supply cell draws v > S
and over-draws a depleting source. Monod bounds v <= Vmax (its saturation asymptote), NOT v <= S. So the deletion of the
cap is unjustified as stated: conservation needs an EXPLICIT clamp v := min(v, S) (with pinned rounding direction and a
post-divide clamp for fixed point), not a free consequence of the functional form.

The bare Monod drops the second law. The irreversible Michaelis-Menten form has no driving-force term, so any positive
substrate activity yields positive uptake regardless of whether the couple releases free energy, which lets a
non-spontaneous redox couple power life (a Principle-9 / second-law violation). The old EMF-clamped-at-zero encoded the
second law. The correct floor form is REVERSIBLE Michaelis-Menten / a flux-force relation whose net flux vanishes at
delta-G = 0 and reverses past it, with delta-G = -nF*EMF keyed to the couple. So the couple's EMF (segment-3
battery_emf) does NOT disappear under the third form: it becomes the intrinsic DRIVING FORCE of the reversible flux, and
the spontaneity gate survives as the driving-force half of the same physical law rather than a bolt-on.

The kinetics must be per-source-class registry DATA, not global constants or a closed enum. A single shared kcat and Km
per source class, applied to every race drawing on that class, forces convergent uptake-affinity and turnover across
independently-evolved lineages, an authored evolutionary outcome that fails the locked per-world-outcome rule and
admit-the-alien; and a fixed table keyed on the built-in classes {light, water, nutrient, redox} gives a world-declared
DataScalar source no kinetic path, breaking the alien-as-data-row property the whole registry exists for. The fix is the
registry pattern this arc already uses: kcat, Km, and the response-SHAPE (a Hill coefficient, n=1 recovering plain
Monod, so water's near-linear potential-gradient uptake and a cooperative sigmoidal uptake and an exotic declared curve
are all data) are carried on each source-class registry entry, Mirror-calibrated to real measured kinetics, per-world
and per-race differentiable. Km is a joint catalyst-substrate affinity, so it belongs per-producer (an evolvable
selection axis) rather than shared per class.

The flux is per-(producer, cell), not a per-cell scalar. catalyst-tissue is the producer's own composition magnitude, a
property of the cell's producer that every observer computing "what flux does producer P draw from cell C" agrees on, so
it is observer-independent and differs from the rejected per-lineage efficiency (which contaminated a shared cell yield a
consumer read). Good, but v must be computed and consumed PER PRODUCER drawing from the cell, never cached as one shared
cell yield, or the P10 break returns in disguise; and when several producers occupy one cell the drawdown order over the
shared stock S must be deterministically resolved. The replacement is NOT byte-neutral (Monod diverges from the linear
supply*conversion on every armed abiotic-source scenario), so the determinism pins re-baseline.

Clean under the panel: reading catalyst-tissue as the producer's material catalytic-axis proxy (not a named
photosynthesis skill or status) is the correct Principle-4 shape; the general saturating-flux-as-floor-physics notion is
not itself Terran-biased. The corrected resolved form for the gate: v = min(S, reversible-MM-flux) with the flux
carrying delta-G = -nF*EMF as its driving force (spontaneity intrinsic), the kinetics (kcat, Km, Hill shape) as
per-source-class registry data Mirror-calibrated to real kinetics and per-race differentiable, Km per-producer evolvable,
v computed per-(producer, cell) with a deterministic multi-producer drawdown, the pins re-baselined. Reserved with basis:
kcat, Km, and the per-class Hill shape, basis real measured enzyme/transport kinetics. No code until the gate rules.
