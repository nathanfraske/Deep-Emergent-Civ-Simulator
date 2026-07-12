# Q1 Stone-3 extension: the biosphere-to-founder energy-transfer conservation ledger (design-first)

This is the design opener for the next arc: extend the Stone-3 `Conserved<Q>` ledger from the matter
cycle to the energy handoffs that feed the founder, so an energy leak in the food chain fails the way
a mass leak now does. Design-first: this document grounds the current energy transfers at source,
classifies them into exact-conservative transfers and legitimately open boundary flows, and surfaces
the one load-bearing finding and the fork before any code lands.

## Grounding (what the tree holds), at source

The reserve substrate (`crates/sim/src/homeostasis.rs`): each being carries `Fixed`-valued reserves
(energy, water, and whatever else its physiology declares) each with a capacity. The per-tick reserve
flows are three, all centralized:
- `metabolize_derived` (line 495) drains each reserve by a per-being derived fraction of capacity, the
  metabolic OUTFLOW (dissipation to work and heat).
- `ingest(axis, amount)` (line 524) deposits a food yield into a reserve, capped at capacity, and
  returns the amount the reserve could hold.
- `adjust(axis, delta)` (line 535) applies a signed change, used by the medium-respiration gas flux
  (`medium::respire`, uptake in a richer medium, loss in a poorer one) and by harm application.

The food inflow (`crates/sim/src/runner.rs`, forage at 3120-3162, the whole-body bite at 3172-3284):
the being bites the cell's MATTER (`material.take(coord, substance, want)`, line 3132), and its reserve
gain is `gain = taken * assim * eta` (line 3143), the assimilated fraction of the taken MASS. The
un-assimilated remainder `taken * (1 - assim*eta)` is a trophic boundary loss.

The producer side (`runner.rs:4347`, `crates/sim/src/biosphere.rs`): the step writes the standing
producer biomass into the resource field, keyed off the physical productivity (a rate fed by insolation
and soil fertility). The producer stock is MASS and supply, not an energy stock.

## The load-bearing finding (Prime Directive 1, verified at source)

The biosphere-to-founder chain conserves MASS, and ENERGY is derived at the point of assimilation
rather than tracked as a stock. The bite decrements the cell's mass; the being gains an assimilated
reserve `gain = taken * assim * eta`; there is no producer-side energy stock that loses exactly that
`gain`. So the closed ENERGY transfer this arc names, a grazer's intake equal to the plant's
fixed-carbon energy loss, does not exist on the current tree, and it cannot be wired as a
`Conserved<Q>` transfer until a producer fixed-carbon energy stock exists. That stock is precisely B's
Fork-4 food-value supersede, as you noted in the Q1-wrap sign-off, so the intake-equals-loss energy
transfer is B-Fork-4-gated. This is a source-verified confirmation of the follow-on I flagged, not a
new claim, and it reshapes what the arc can build now versus design now.

## The energy and reserve flows, classified

- Boundary flows, legitimately open (recorded with a tagged source or sink, never a leak): insolation
  into productivity (source), radiation and metabolic dissipation out (sink), and the un-assimilated
  trophic remainder `taken * (1 - assim*eta)`, which the matter cycle already returns to soil.
- Reserve inflows: forage ingest and whole-body-bite ingest (energy derived from bitten mass), plus
  the respiration adjust (gas flux, signed).
- Reserve outflows: `metabolize_derived` (dissipation) and the harm adjust (a `CONDITION` decrement).
- Clamp-drops, the reserve's real silent-leak sites: `ingest` caps at capacity and returns the held
  amount, so an ingest into a near-full reserve drops the excess (satiation waste); the drain and take
  floor at zero, so a draw beyond the reserve drops the unmet part (a starvation shortfall). A closed
  energy budget must RECORD these as boundary flows rather than let them vanish.
- Closed transfers guardable as `Conserved<Q>::transfer` today: none on the food side, because the
  producer energy stock does not exist. The one place a closed transfer will live is the
  intake-equals-loss leg once B lands Fork-4.

## The design (the Stone-3 energy extension, staged)

Piece A, buildable now, the energy analogue of the matter-cycle gate: a per-being reserve
BOUNDARY-FLOW closure gate. The being's total reserve changes each tick by exactly the sum of its
recorded flows (ingest in, metabolize out, harm out, respiration signed), with the satiation-cap and
starvation-floor clamps recorded as tagged boundary flows, so a silent clamp-drop of reserve energy
fires the gate. This lifts the being's energy budget to a runtime invariant the way the matter gate
lifted the ground's mass budget, and it does not wait on B.

The honest byte-neutrality caveat (unlike the matter gate): the matter gate needed only a before-and-
after total because the decomposition cycle is CLOSED. The reserve is not closed (it carries legitimate
inflows and outflows), so this gate needs before-and-after equal to the net recorded flow, which means
recording the per-tick flows. Before recommending a build shape I will assess whether that recording is
byte-neutral-cheap (a debug-only accumulator at the three centralized flow sites, `metabolize_derived`,
`ingest`, `adjust`) or invasive (the `Conserved<Q>` token re-wrap I declined for the matter cycle as
over-engineering). My prior is the accumulator is cheap because the flows are few and already
centralized, but the assessment governs the recommendation, not the prior.

Piece B, design-only now, B-Fork-4-gated: the closed intake-equals-loss ENERGY transfer via
`Conserved<Q>::transfer`, wired once B lands the producer fixed-carbon energy stock. The bite becomes a
transfer of `taken * assim * eta` from the producer energy stock to the being's reserve, with the
un-assimilated remainder a recorded boundary loss. Flagged, coupled to B's Fork-4, not built here.

### The intake-equals-loss guard, ready to drop in (grounded against B's #156)

B's #156 (photosynthesis-to-productivity) is where the producer fixed-carbon energy stock arrives: its
Fork-4 food-value supersede retires the unconditional food-value scalar so a grazer's intake DERIVES
from the fixed carbon a producer accumulated from derived net primary productivity, rather than from a
scalar the bite reads today. #156 is design-first with the stock's exact site still an open fork, so the
guard is designed against the CONTRACT, not a line that does not exist yet, and it drops in at whatever
site B's supersede introduces.

The contract the guard enforces at the bite, once the producer holds a fixed-carbon energy stock
`C[cell]`: the being draws `consumed` from `C[cell]`, gains `gain = consumed * assim * eta` into its
reserve, and the un-assimilated remainder `consumed - gain` is egested. The `Conserved<Q>` sequence is
exactly the ledger primitives from slice 1: obtain the consumed quantity as the producer-stock
decrement, `split` it into the assimilated part and the egested part, `transfer` the assimilated part to
the being's reserve, and `destroy` the egested part to the egestion sink (which the matter cycle already
returns to soil). The producer's fixed-carbon loss then equals the reserve gain plus the egested loss
exactly, and a leak (energy vanishing at the handoff) fires the per-step gate. This is the exact
intake-equals-source-loss guard, ready to wire at B's supersede site the moment Fork-4 lands.

The one open question the wiring inherits from B, surfaced not answered: whether the producer stock B
lands is a per-cell fixed-carbon ENERGY (the natural form for the energy transfer this guards) or a
composition MASS vector (the current `set_producer_food` shape), because the guard keys on the quantity
the stock holds. I track B's Fork-2 and Fork-4 rulings on #156 so the drop-in matches the stock B builds,
rather than assuming its type.

## The design questions for your gate

1. Scope now versus hold for B. Build Piece A (the reserve boundary-flow closure gate, real today and
   B-independent) now, and flag Piece B (the closed transfer) for B's Fork-4? Or hold the whole arc as
   design-only until Fork-4 lands, since the leg you named is B-gated? My recommendation is to build
   Piece A now: it is the real energy-integrity guard that does not wait on B, it is the direct analogue
   of the matter gate, and it catches the satiation and starvation clamp-drops that are the reserve's
   actual silent-leak sites.
2. The clamp-drops. Record the satiation-cap and starvation-floor drops as boundary flows (the honest
   closure, my recommendation), or is a reserve that clamps silently at its bounds the intended model?
   My read: the clamps are physical (a reserve cannot store past capacity, nor drain below empty), so
   the drops are real boundary flows (satiation waste, starvation shortfall) and recording them closes
   the budget without changing the sim (byte-neutral). This is a classification question, not a value.
3. Bite and forage MASS coverage. The matter-cycle gate brackets `step_matter_cycle` (decomposition),
   not the bite leg where mass crosses from cell to being. Do you want a sibling mass-closure gate over
   the bite path (the cell mass removed equal to the being's assimilated mass gain plus the egested
   remainder plus the carried-mass change), which I would verify conservative at source first the way I
   did the matter cycle? Offered, not assumed.

## Discipline

No code until you gate the design. The gates are byte-neutral by construction (debug-and-test runtime
assertions, compiled out in release, so the five pins hold), unless one lights up a real leak, in which
case the fix is a reviewed byte change and a finding, a candidate founder-starvation diagnosis exactly
as the matter gate was framed. No authored value: the gates are exact-`Fixed` accounting identities
with no tolerance, keyed on whatever reserves the being's physiology declares, so a photosynthetic,
mana, or redox being is a data row (`physiology.rs`). Section-9 once by me per slice. This PR is the new
bridge; #143 refreshes against main and merges as the accepted volatile substrate the moment this opens.

## As built: Piece A (the clamp-drop diagnostic plus the local clamp forward-guards)

The design-question 1 recommendation was refined by a grounding finding and the gate ruled the honest
form. Grounding the reserve at source, `Stock::amount` (`stocks.rs`) is private and mutated at exactly
four clamped sites (`step`, `deposit`, `take`, `set_capacity`), and every Homeostasis reserve path routes
through them, so the reserve has no conservation partner within itself and cannot leak off-ledger: a
`delta == recorded flows` closure assertion would pass tautologically, not as a leak-hunt with teeth like
the matter cycle (which conserved a total across separate subsystems where a real leak was possible). So
Piece A is two things, not a closure gate:

- The clamp-drop DIAGNOSTIC: each `Stock` accumulates, in `#[cfg(debug_assertions)]` fields, the
  satiation-cap overflow (deposit and regen overflow, and a capacity-decrease spill) and the
  starvation-floor shortfall (a draw or take beyond what is present). `Homeostasis::clamp_drops` sums
  them per being as `(satiation_waste, starvation_shortfall)`, the second lens on the founder starvation:
  a starvation shortfall means the metabolic drain outran the reserve (the being could not gather
  enough), while satiation waste means food arrived but the cap could not hold it (a capture or foraging
  gap, not a food gap), so crossed with a survival sweep the two point at different causes.
- The local clamp FORWARD-GUARDS: inside each mutation a `debug_assert` checks that the drop equals the
  clamp boundary computed independently (regen overflow equals the excess over capacity, the drain and
  take shortfalls equal the demand over what is present), so a future edit that makes a clamp drop energy
  silently fails at the site.

Built in `stocks.rs` (the instrumentation, the debug accessors, a manual `PartialEq`/`Eq` that ignores
the diagnostic scratch so equality stays release-identical) and `homeostasis.rs` (the per-being
`clamp_drops`/`reset_clamp_drops`). Byte-neutral: the fields and asserts are compiled out of release, so
all five pins hold bit-exact (`40fe8a72`/`d05a6488`/`9a28f113`/`967b22bd`/`b62eb73d`). The local guards
ran live across the whole debug sim suite (909 lib tests plus the integration suites, every being
metabolism, forage, and adjust path) with no assertion firing, so on the current tree the reserve's only
energy losses are the recorded clamp-drops and every clamp accounts every unit, the Piece-A analogue of
the matter gate passing. The true energy-LEAK guard stays Piece B (intake-equals-loss), which has the
food conservation partner, on B's Fork-4.

The living readout is wired: a debug-only `Embodiment` accumulator folds each tick's per-being
clamp-drops PER AXIS into a run total BEFORE the cull (so a founder that dies this tick still counts its
final tick's losses), and `run_world` prints it for any embodied run. A PD1 catch shaped it: the coarse
population total on `living` reads satiation-cap dominant (about 6615 versus 0.65), which would say "food
left on the table", but the per-axis breakdown reverses that on the food axis. On the ENERGY reserve the
`living` founders drop only at the STARVATION FLOOR (0.65 starvation, zero satiation): the metabolic
drain outran intake, the founders could not gather enough before starving, not a food-quantity wall. The
6615 satiation is on the CONDITION regulated band (a set-and-regulated health band, not a conserved
metabolic pool), a clamp churn that is not a food or energy-pool signal, so the diagnostic's meaningful
signal is on the metabolic reserves (energy, water). Byte-neutral: the accumulator, the tick hook, and
the print are all `#[cfg(debug_assertions)]`, so all five pins hold bit-exact and the pinned and
scheduled orders stay in step (the totals are never folded into the hash).
