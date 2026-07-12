# The interior arming bridge (design-first, co-sequenced with A's Mirror composition producer)

Agent B, the co-arming bridge the gate asked for before it merges the interior column-wiring (#176). This
opener scopes what the interior lane needs to ARM on a Mirror run, held to co-sequence with A's Stage-3c
composition producer so the interior and surface arm into one coherent Mirror rather than half a world.
Design-first, doc-only, off current main. No build until the gate rules the datum and the sequence.

## What #176 lands, and what arming still needs

The interior column-wiring (#176, on the gate's merge) lands on main: the additive continuous
`GeodynamicColumn` fields (`temperature`, `convective_stress`, `rayleigh`), `populate_interior_column` and
`step_interior_field` (snapshot-apply), `derive_mantle_density` (A's petrology kernel over a mantle
composition at mantle pressure and temperature), and the derived boundary-layer stress. All of it is dormant,
armed by no scenario.

Arming means a Mirror run calls `step_interior_field` each geodynamic tick, which needs three inputs the
wiring consumes but does not itself supply, each DERIVED or per-world DATA, none authored:

1. **The mantle COMPOSITION datum** (per-world data, the input `derive_mantle_density` reads). The key is
   `world.mantle_composition` (gate ruling, the single shared source with A: A's Stage-3c producer reads the
   SAME key, plus a sibling `world.crustal_composition`, so the mantle density is derived once from one datum,
   never two, and the interior adds no second mantle-composition datum). For Mirror the value is Earth's
   primitive mantle, pyrolite: the Mg-Fe-Si-O system dominated by olivine, orthopyroxene, clinopyroxene, and
   an aluminous phase (spinel or garnet with depth). The element-to-amount vector is a per-world datum with a
   real-world basis and citation (the primitive-mantle / pyrolite estimate, McDonough and Sun 1995, "The
   composition of the Earth"), reserved-with-basis-and-citation for the owner to confirm rather than
   fabricated. The petrology kernel minimizes this composition to its stable assemblage and reads the density,
   so roughly 3.3 grams per cubic centimetre falls out; no density is authored.

2. **The lithostatic PRESSURE the petrology reads** (derived from geometry). The mantle pressure is the
   overburden weight, `P = rho * g * depth` integrated down the column; a reference-pressure first pass uses a
   reference density over the depth, and a short fixed-point iteration then reads the petrology-derived density
   back into the pressure to resolve the mild density-depends-on-pressure self-consistency (density depends on
   the pressure that depends on density). Both are derivations: the depth is per-world geometry (the mantle's
   extent from the planet and core radii), and the gravity `g` is READ from A's 3c DERIVED gravity accessor,
   never a hardcoded value (gate ruling, the same discipline as C's ballistic integrator); the depth-varying
   `g` inside the planet is a later refinement, and the surface derived `g` is a sound first-pass reference for
   the mantle overburden. No pressure is authored.

3. **The mantle TEMPERATURE** the petrology reads, which is already the interior heat chain's own thermal
   state (the column `temperature` the convection evolution carries), so it needs no new input.

## The co-sequencing (why this is a bridge, not a build now)

A's Stage-3c is the surface composition producer, which writes the crust's `crustal_density` from the crust
composition through the same petrology kernel. The interior's `isostatic_elevation` floats the surface crust
on the interior-derived mantle density, so arming the interior against a crust the surface has not yet
produced would float a crust on a mantle with no crust to float. Co-sequencing the interior arming with A's
3c means the crust composition and the mantle composition arm together, and the two-way `GeodynamicColumn`
boundary (snapshot-apply, order-independent, #176) comes alive coherently: the surface writes
`crustal_density`, the interior reads the snapshot and writes `isostatic_elevation`, and the surface
relaxation reads it, all on the same armed run.

So this bridge is design-first: the mantle composition datum with its citation and the lithostatic-pressure
derivation are scoped here for the gate's ruling, and the arming scenario lands co-sequenced with A's 3c, a
single gated move.

## Seams surfaced for the gate's ruling

1. **The mantle composition datum: SETTLED (gate ruling).** The key is `world.mantle_composition`, the single
   shared source A's Stage-3c producer also reads (plus a sibling `world.crustal_composition`), so the mantle
   density derives once from one datum. The interior reads exactly `world.mantle_composition` and adds no
   second datum. The value is Mirror's pyrolite composition (element-to-amount, McDonough and Sun 1995),
   reserved-with-basis-and-citation for the owner's confirmation, no fabricated density.

2. **The lithostatic-pressure self-consistency.** I propose the reference-pressure first pass plus a short
   fixed-point iteration (both derivations), bounded by a fixed iteration cap (a determinism bound, never an
   unbounded spin, the same discipline the convection solve uses). Confirm the reference-pass shape and the
   cap basis.

3. **The arming sequence.** Confirm the interior arming co-sequences with A's 3c (one gated move), and rule
   which run scenario arms the geodynamic tick and at what cadence (a deep-time coarse-LOD pass, per the
   geodynamics arc proposal's Tier 1 accelerated spin-up).

## What this bridge does not do

It does not arm any scenario (that is the co-sequenced move on the gate's ruling). It does not author a
density, a pressure, or a composition (the composition is per-world data with citation, the pressure and
density derive). It does not touch A's crust composition producer (3c) or C's ledger coupling (#174). No
build until the gate rules the datum, the self-consistency shape, and the sequence.
