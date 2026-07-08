# Edibility energy grounding (R-PHYS-BIO edibility measure): scope

Owner directive (2026-07-08): the food-VALUE must not be a made-up "biomass" number; eating should
replenish a reserve by the food's own NUTRIENTS, derived from the physics substrate, or we identify the
missing material. This note scopes that derivation. It is the deferred R-PHYS-BIO edibility measure the
physiology arc flagged ("the intake yield stands in for the R-PHYS-BIO edibility measure until wired to the
located matter").

## The seam (verified against source)

The metabolism is HALF-GROUNDED.

- **Drain (physical).** `physiology::derive_base_drain` composes Kleiber basal (`laws::basal_metabolic_rate`
  over `body_mass_kg`) plus thermoregulatory heat loss, then divides by the reserve's stored energy to a
  fraction. The reserve's stored energy is `energy_capacity * body_mass_kg * bio.energy_density`
  (`base_drain_from`, `reserve_mass = energy_capacity * mass_kg`, then `metabolic_drain_fraction(.., reserve_mass,
  energy_density, ..)`). So a reserve unit is worth `body_mass_kg * bio.energy_density` joules. The absolute
  kJ/g-to-joule reconciliation is R-UNITS-PIN (an owner anchor already flagged).

- **Intake (abstract).** `Embodiment::geophage` and the locomotion `INGEST` compute
  `gain = laws::satisfaction(supply, assim, req) * capacity` (bounded by room), where
  `satisfaction = clamp(supply * assim / req, 0, 1)` is a saturating ratio and `capacity` is the reserve
  capacity. The food's own content DOES enter as `supply` (the forage reads the standing food's
  `bio.energy_density` off the `ResourceField` composition; a body's tissue reads it off the parcel), but it
  is squashed through the SATURATING fill-toward-capacity model rather than delivered as linear physical
  joules: a rich food saturates to one and delivers `capacity` (fills the reserve in a bite), regardless of
  its actual joule content, and `capacity`/`req`/`biomass_per_stock` are the non-physical scalings that make
  the delivered amount a made-up number rather than `mass * energy_density`.

The asymmetry is the bug the owner sensed: a physical drain (real joules out) paid against a non-physical
intake (a saturating fill in), reconciled by made-up conversions. It also explains the pre-existing viability failure (the full dev world goes extinct in
~6 generations at every seed, byte-identical before and after the chemistry arc).

## The derivation (no new material needed for energy)

Eating a bite of physical mass `m` of a food whose own axis value on the reserve's backing class `C` is
`food[C]` delivers `m * food[C] * assimilation` of C-content. For the ENERGY reserve (`C = bio.energy_density`)
that content IS joules, and the reserve gain in capacity units is:

    gain_capacity = (m * food.energy_density * assim * eta) / (body_mass_kg * body.energy_density)

because a reserve unit is worth `body_mass_kg * body.energy_density` joules (the drain's own bridge). Every
term is a physics-floor axis a plant/body already declares (`mat.density` gives `m = volume * density`,
`bio.energy_density` the content) or the eater's own physiology (`body_mass_kg`, `assim`, `eta`). So the
food-value is DERIVED; `biomass_per_stock` and the T3 food-value bridge DISSOLVE.

What remains authored are genuine physics/biology CONSTANTS (Principle 9): `kleiber_a`, the trophic/assimilation
efficiency `ingest_efficiency`, the weathering and decomposition kinetics, and the one R-UNITS-PIN anchor (the
reserve's absolute joule scale). None is a fudge.

Non-energy reserves (water, minerals, a condition axis) generalize the same way, each reading the food's own
content on its backing class; the per-reserve unit bridge is the same R-UNITS-PIN family. Water content is
`bio.water_fraction` (already a floor axis); a mineral is the substance's own composition. So no missing
material for the modelled reserves; a reserve whose backing class has no physical content axis would be the
"more materials" case, flagged if one appears.

## The derivation must be ALIEN-CLEAN (owner constraint, 2026-07-08)

The energy-from-food path must NOT hardcode `bio.energy_density` (a Terran chemical-energy assumption). A
thaumic being draws its reserve from a mana axis, a chemosynthetic one from a redox axis, another directly
from some alien substance. The mechanism must let each being derive its reserve a DIFFERENT way from data.

This is already the substrate's shape and must stay so: a reserve is backed by a DATA class
(`HomeostaticAxisDef::backing_component`), and the intake reads the food's content on THAT class. The physical
form keys on the reserve's OWN backing class `C` and the being's OWN body composition (its storage density on
`C`, from `whole_body_composition_vector`), never a fixed `bio.energy_density`:

    reserve gain = (content_of_C_eaten * assimilation(C) * eta) / (body_mass_kg * body_composition[C])

- `content_of_C_eaten`: how much of the food's `C`-content the being takes (the food declares `C`, whatever
  `C` is: `bio.energy_density`, `arcane.mana_density`, a redox axis, ...).
- `body_composition[C]`: the being's own storage density on `C` (from its body plan), so the reserve-amount
  units are body-relative and the SAME bridge the drain uses (`reserve stored = amount * mass_kg * body[C]`).
- A being whose body carries no `C` (no storage density) has no reserve of that kind and eats none of it.

So a thaumic grazer with a mana-backed reserve eats a mana-bearing plant and fills its mana reserve by the
SAME mechanism a chemical grazer fills its energy reserve from an energy-dense seed. The membership (which
axis a reserve draws on) is data; the mechanism is fixed. No `bio.energy_density` in the engine's decision
path; the Steering Audit invariant is that swapping every axis label leaves the mechanism unchanged.

## Build shape (proposed)

1. A `laws::edible_energy` (or a physiology helper) giving the reserve gain from `(mass, food_axis_value,
   assim, eta, body_mass_kg, body_axis_value)`, the physical form above, replacing the `satisfaction * capacity`
   intake in BOTH `geophage` and the locomotion `INGEST`. Mass-honest: the bite mass removed equals the
   content credited divided by `assim * eta` (the trophic step already used).
2. Retire `biomass_per_stock` and the T3 food-value gate from the arming path; the standing-food volume the
   producer regrows becomes a real biomass (mass = volume * plant density), and its energy is
   `mass * plant.energy_density`.
3. The reserve's absolute joule scale is the single R-UNITS-PIN anchor (surfaced, owner-set), not a per-food
   fudge.

## Why this is NOT byte-neutral, and touches an owner anchor

This rewrites the UNIVERSAL intake path (every scenario forages/geophages), so it re-pins crucible, viability,
default, and full: it is not an opt-in slice. Its absolute scale is R-UNITS-PIN, an owner calibration. So the
build is autonomous, but the reserve joule-scale anchor and the viability calibration are the owner's to set
(dev values may be stood up to run and prove the big dev world, per the owner's standing allowance). The
proof-of-work target: the full dev world SURVIVES (no universal extinction) once the intake is physical and
the anchor is dev-set to a plausible scale.
