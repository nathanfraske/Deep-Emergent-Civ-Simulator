# Layer 0 file-structure layout (Agent A lane), with the disjoint-file boundaries confirmed

This is the design-first layout the gate asked for before a line of Layer 0 is written. It maps every file my
lane touches, confirms the disjoint-file boundaries against B's and C's lanes so the gate can relay them, and
states the one slice in my lane that re-pins rather than staying byte-neutral. The lane split the gate carved:
A (this doc) takes the periodic-table plus phase-thermodynamics reference tier, the surface-pressure datum,
Stage 1's flux derivation, and the Layer-0 coordination (the G add); B takes the internal-heat W/kg axis and
the memory primitives; C takes the determinism primitives and the provenance-DAG accounting.

## The floor-data axis-versus-substance split (the gate's explicit ask)

The floor `.toml` files key everything on the QuantityAxis / InteractionLaw registry, and the top-level table
kinds are already separate arrays-of-tables: `[[axis]]` (a property axis, for example a strength or a
conductivity), `[[substance]]` (a reference-material vector, the fifteen-substance tier), `[[law]]` (an
interaction law), and, in `periodic_table.toml` alone, `[[element]]` (a periodic-table row). B's internal-heat
W/kg axis is a new `[[axis]]`; my periodic-table extension is `[[element]]` columns and my phase-thermo tier is
`[[substance]]`-shaped rows. So the two are disjoint by table KIND even inside one file. To make them disjoint
by FILE as well (so a merge never touches the same file), I put my phase-thermo registry in a new dedicated
`.toml` rather than appending substances into `mechanical_floor.toml` or `ground_floor.toml`, and B's W/kg axis
lands in its own geodynamics floor file or in `mechanical_floor.toml`, neither of which my lane edits. Confirmed
disjoint at the file level.

## My lane's file map, piece by piece

**(a) The periodic-table plus phase-thermodynamics reference tier.** Two files, both mine, neither touched by B
or C:
- `crates/physics/data/periodic_table.toml`: extend the `[[element]]` rows (today `symbol`, `name`, `z`,
  `standard_atomic_weight`, the interval bounds, and the cited `real` provenance) with the element data the
  petrology kernel needs downstream: valence and the standard thermochemical reference (Gibbs energy of
  formation, standard entropy) per element. Additive columns; existing rows keep their current fields, so the
  molar-mass kernel is unchanged.
- `crates/physics/data/phase_registry.toml` (NEW): the candidate-phase registry, one entry per candidate phase
  carrying Gibbs formation energy, standard entropy, molar volume, the equation-of-state and thermal-expansion
  parameters, and the Clapeyron solidus slope in K/MPa. An extensible registry (mechanism fixed Rust,
  membership data that grows with the world), sibling to the fifteen reference substances, NOT a closed enum of
  Earth minerals. Read by a new `crates/physics/src/petrology_data.rs` module (or an extension of `periodic.rs`)
  that exposes typed accessors; no existing accessor changes.

**(b) The world surface-pressure datum.** Additive, disjoint by id, and (corrected from the earlier walk-back)
with NO `P_ref` retirement, because the `laws.rs` `P_ref` is the boiling-point reference-pressure parameter,
correct physics:
- `calibration/reserved.toml`: a new `[[reserved]]` entry `world.surface_pressure` (category per_world, Mirror
  pinning Earth's cited ~101325 Pa), following the existing id idiom, additive and disjoint from B's and C's
  reserved rows.
- `calibration/profiles/mirror.toml` and the sibling profiles: the per-profile value rows for the new id.
- The phase-gate reconciliation reads `world.surface_pressure` alongside the existing
  `climate.mean_surface_temperature` scalar; that read lives in `environ.rs`, which is my lane for Stage 1
  anyway. Before I wire it, I verify whether any live ambient or driving-pressure read (`fluid.driving_pressure`
  and its default) silently assumes one atmosphere, and if such a live default exists I bring it to the gate
  with the file:line before touching it, rather than asserting a defect on a grep's implication.

**(c) Stage 1's flux derivation.** All in `environ.rs` (the Star, DiurnalSky, and insolation home), so
`laws.rs` stays out of my lane entirely:
- Split the `Star` intrinsic luminosity from the delivered flux (today `Star` carries only the pre-attenuated
  luminosity), add the mass-luminosity kernel L = L_sun*(M_star/M_sun)^exponent (dimensionless ratio form, no
  G), and add the flux kernel flux = L/(4*pi*d^2), replacing the inline `Fixed::from_int(1361)` in
  `DiurnalSky::reference` and `::mirror`. The insolation product downstream is unchanged; only the watt-scale
  becomes derived.
- `calibration/reserved.toml`: the Stage 1 reserved leaves (M_star, orbital distance d, the mass-luminosity
  exponent) and the reference-data anchors (M_sun, L_sun, AU), each surfaced with its basis, additive by id.

**(d) The Layer-0 coordination: the G add.** One file, mine:
- `crates/units/src/fundamentals.rs`: add `GRAVITATIONAL_CONSTANT` (symbol `G`) to the `FUNDAMENTALS` array,
  following the existing `Fundamental { symbol, name, value, unit, provenance }` shape, cited to CODATA, when
  the owner or the gate sends the value. Stage 1's flux does not need G; Stage 2's Kepler year-from-orbit does,
  so the add lands in Layer 0 but is exercised in Stage 2. No other lane touches `fundamentals.rs`.

## The disjoint-file boundary table (for the gate to relay)

| Surface | A (me) | B | C | Boundary |
| --- | --- | --- | --- | --- |
| `periodic_table.toml`, `phase_registry.toml` (new) | owns | - | - | A-only |
| `petrology_data.rs` (new) | owns | - | - | A-only |
| `fundamentals.rs` (G) | owns | - | - | A-only |
| `environ.rs` (Star split, flux kernel, phase-gate read) | owns | - | - | A-only |
| internal-heat W/kg `[[axis]]` (own geo floor file or `mechanical_floor.toml`) | - | owns | - | B-only, no A edit |
| `laws.rs` | - | memory-primitive law-forms | determinism kernels | B+C only (A carries flux in `environ.rs`); sequence B then C |
| `calibration.rs` (`Category` enum) | - | - | provenance tags | C-only code |
| `calibration/reserved.toml` | surface-pressure + flux ids | internal-heat id | - | additive by id, disjoint |

The only multi-lane file is `laws.rs` (B then C, per the gate's sequencing); everything in my lane is A-only or
additive-by-id. No shared floor `.toml` file, because my phase-thermo tier is a new file.

## Build order and the one stated re-pin

Most of Layer 0 in my lane is additive and byte-neutral to the five canonical pins: new data files, new
reserved entries, a new fundamentals entry, and new accessors change nothing on the live path of the default,
full, discovery, viability, or living scenarios. The one exception, stated rather than hidden, is Stage 1's
flux kernel. Replacing `solar_constant = Fixed::from_int(1361)` with the derived flux L/(4*pi*d^2) lands at
Earth's true total solar irradiance (~1361.3 W/m^2 from L_sun, the AU, and the inverse-square law), which is
not the exact-integer 1361 anchor. The living scenario is the one that arms the sky (`run_world.rs`
`arm_diurnal`), so its pin re-pins from the 1361 anchor to the derived flux, and the other four scenarios,
which do not arm the sky, stay byte-identical. I confirm both by measurement when I build the slice: the four
non-sky pins bit-identical, the living pin's new value enumerated for the owner, never tuned to reproduce 1361.
The derived flux is the honest value the reserved M_sun, L_sun, AU, M_star, and d produce; if the owner sets
those anchors differently the pin lands accordingly.

The slice order I propose: the G add and the periodic-table columns first (pure additive, zero pin risk), then
the phase registry and its reader, then the surface-pressure datum and the phase-gate read, then Stage 1's flux
derivation last (the one re-pin, measured and stated). Section-9 lenses run once by me at the Layer 0 milestone;
the gate double-checks. Each slice is its own reviewed commit.
