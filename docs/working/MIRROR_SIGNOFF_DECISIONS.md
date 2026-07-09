# Mirror sign-off decisions (owner rulings, 2026-07-09)

The owner's rulings on the Mirror dial-set and the escalate_owner list, captured as they are made so the agent
applies the authoritative set and boots Mirror under `Profile::Calibrated`. Each is the owner's call; the agent
applies it with the stated basis and never re-fabricates.

## Batch 1 — world-defining physical values

- `climate.mean_surface_temperature` = **287 K** (~13.9 C, Earth's PRE-INDUSTRIAL mean). Owner's reasoning: the
  world starts before any industry, so it starts at the pre-industrial baseline, leaving room for emergent
  warming to go somewhere. (Pollution + climate-change substrate added to the roadmap, SUBSTRATE not authored.)
- `climate.latitude_temperature_range` = **60 K** (equator-to-pole full swing).
- `world.orbital_period_seconds` = **31556952** (the TROPICAL year, 365.2422 d) for a strict 1:1 Earth.
- `physiology.thermal_half_band` = **8 K** (temperate-mammal thermoneutral half-width). Owner condition: this
  must NOT prevent races evolving heat/cold attunement over generations. RESOLVED: the fixed core setpoint plus
  evolvable body morphology (covering/mass/surface, genome-derived, driving the thermal coupling per the
  physics floor) IS how endotherms adapt (Bergmann/Allen), so attunement emerges without changing the setpoint.
  A drift-able genome setpoint (for ectothermy/torpor) is flagged in the roadmap as a later substrate option.

## The owner's standing gate (applies to every authored value below and beyond)

Before ANY value is set as authored, the agent first checks "can this be derived further?" If it can derive
from a deeper substrate, derive it (emergent, and evolvable where it is a per-race disposition); only if it is
genuinely irreducible is the recommended value authored. This is the derive-vs-author line applied per-value.

## Batch 2 — derive-vs-author and emergence-shaping

- `transmission.drift_rate` (0.03) and `enculturation.stubbornness_split` (0.40): AUTHOR with basis now (boots
  Mirror), DERIVE in the later social-depth arc. Flagged so it is not forgotten.
- `axiom.group_aggregation_rule`: AUTHOR equal-weighting now, DERIVE from member variance in the social arc
  (a divided group less firm). Flagged, not forgotten.
- `inst.crystallization_threshold` + `inst.crystallization_rate`: HARD-WON (slow ~0.001-scale rate, high
  threshold) so institutions mean something. GATED on the cannot-be-derived-further check first (if it can
  derive from the coordination dynamics / need-vector alignment, derive it; else author hard-won).
- `discovery.exploration_floor` + `discovery.surprise_gain`: BALANCED / curiosity-positive. GATED on the
  cannot-be-derived check first (if exploration can derive from a per-being novelty-seeking disposition, an
  evolvable genome trait, derive it; else author balanced).

## Batch 3 — the world's character

- `felt_conviction.retention`: SLOW, lifetime-scale (a sustained pattern shifts a conviction, a single day does
  not). Gated on the per-being belief-plasticity derivation first.
- `metabolism.body_mass_kg_scale`: ~70 kg (Mirror human; makes the Kleiber drain match real human BMR).
  Confirmed: every value here is MIRROR-specific; other worlds use their own dials.
- `behavior.controller_target_mutations` + `mutation_step_fraction`: MODERATE evolutionary pace. Gated on
  whether the mutation rate should be a per-race genome trait (evolvable) rather than a global.
- `genome.allele_presence_threshold`: MODERATE (Earth-like genetic distance for speciation).

## The remainder (delegated to the agent under the gate)

The lower-taste items are directed to the agent under the standing gate above:
- Group A (non-Mirror `.low`/`.high` dials for Venus/Europa/Tempest): DEFER to each world's calibration.
- Group B (engine/determinism bounds: `lang.dawn_round_cap`, `planning.depth_cap`/`hop_cap`, `field.cell_size`,
  `langmod.dawn_cap_unitsize`): set generous (a performance bound, not a realism one), each gated on derive.
- Group D (units anchors / conventions: `body.promotion_shape` human values, `langmod.channel_registries` as
  world data, the salinity/evaporation reference-choice-then-value, the equal-weighting convention defaults):
  take the agent's recommendation once the derive-check clears.
- Group E (the 5 AUDIT CATCHES): the agent re-derives or flags each from its own real basis, never the
  research-tagged value (the Ne~50 analogy, the invalidated pin, the key-vs-wiring mismatch, etc.).
The reviewer surfaces to the owner any of these that turn out to need their taste.
