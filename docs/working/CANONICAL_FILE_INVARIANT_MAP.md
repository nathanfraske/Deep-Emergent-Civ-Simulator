# Canonical file-to-invariant decomposition map

Status: planning-only no-move map, baseline inspected 2026-07-22 at PR #215
head `2ae61a3d920aafca74ba6154e14fc83ebb302187`. The same integration slice adds
observer-only scene children after this baseline; the map still moves no
candidate physics source.

This record maps active canonical and candidate files to coherent future child
modules. It moves no source, test, formula, constant, schema, receipt, or public
API. A proposed boundary says where one already-present responsibility could
live after direct evidence exists. It does not validate the mechanism inside
that boundary, admit a value, close a proof, or make the candidate substrate
reachable from the canonical runner.

The current canonical run still enters Stage 1 and refuses on
`stellar_birth.realization_measure`. Biology, civilization, dawn, and the
causal legacy viewer remain parked. They appear here only where a shared-source
compatibility constraint can break a pure move.

## Rules for every future split

1. Split tests before production where practical. A test move preserves test
   names, inputs, branch order, ignored status, and expected bytes.
2. Move one invariant per commit. Do not change arithmetic grouping, iteration
   order, reduction order, branch order, fixed-point scale, refusal identity,
   event order, hash order, or random coordinate in a move commit.
3. Preserve the existing facade path and visibility. A move cannot broaden a
   private candidate into a public API or add a constructor for a sealed type.
4. Treat every path change as a gate change. Path-bound baselines and generated
   discovery records move in the same reviewed commit, with their prior audit
   classifications preserved rather than regenerated without review.
5. A green unit test proves regression coverage for its exercised fixture. It
   does not prove canonical admission, physical correctness outside that
   fixture, or admit-the-alien coverage.
6. A source citation or held artifact establishes evidence custody. A value
   still derives from the sealed floor, passes the complete floor-admission
   process after derive-first exhaustion, or remains absent while the stage
   refuses.
7. A pure split retains the model version, receipt schemas, and canonical
   bytes. A change that needs a schema or model-version bump is semantic work
   and belongs in a separate slice.

## Current ownership and facade constraints

| Surface | Current authority boundary | Constraint on a future split |
|---|---|---|
| `crates/planet` | Canonical seven-stage runner. Its runtime dependencies are only ledger and units. | Canonical child modules stay private unless an existing read-only view is already re-exported. No candidate-substrate dependency lands as part of a file move. |
| `crates/planet-substrate` | Private pre-migration candidate mechanisms. The crate exports no item. | Keep the fourteen top-level raw module filenames and `absolute_floor.rs` as private facades. Put children beneath matching directories. |
| `crates/physics` | Active abiotic mechanism and candidate-data substrate. | Preserve public paths such as `civsim_physics::moment_equivalence::*` through explicit facade re-exports. A re-export must not add construction or authority. |
| `crates/viewer` | Observer-only leaf with one direct dependency on `civsim-planet`. | It may borrow a sealed `PlanetObservation`, borrow a `PlanetSnapshot`, and inspect an immutable `RunReceipt`. It cannot call the runner or own causal state. |
| `parked/crates/sim` | Legacy compatibility consumer, excluded from planetary readiness. | It compiles the retained substrate files through exact `#[path]` attributes. A retained facade must continue to compile under both crate roots until this compatibility edge is retired. |

The private substrate facade filenames are load-bearing. The planet boundary
gate requires exactly the current top-level `.rs` set, and the parked simulation
points at those same files. Replacing `astro.rs` with `astro/mod.rs`, for
example, breaks both contracts. The compatible shape is:

```text
src/astro.rs                 private facade, still present
src/astro/orbits.rs          future child
src/astro/disk.rs            future child
src/astro/tests/mod.rs       future test root
```

The facade may use private child modules and visibility no broader than needed
for existing sibling callers. Existing public functions inside a private
top-level module remain private to the package from the canonical workspace's
point of view. The parked compatibility crate currently exposes those modules
under its historical API, so compatibility checks remain required without
granting that API canonical standing.

## Gate and generated-record assumptions

| Check | Current path assumption | Required treatment during a split |
|---|---|---|
| `scripts/planet_boundary_gate.py` | Requires the exact fourteen top-level substrate raw files and `absolute_floor.rs`; rejects any additional top-level `.rs` file. It recursively scans package source for boundary violations. | Retain each top-level facade. Add children only beneath directories. Run the gate and its self-test after every compiling move. |
| `scripts/constructor_gate.py` | The whole-file exemption names `crates/planet-substrate/src/astro.rs` exactly. The hard baseline names five `from_bits` sites in `moment_equivalence.rs` and three in `deeptime.rs`, each by path and argument digest. | A child does not inherit the `astro.rs` exemption. Audit each moved constructor and either keep the parse boundary in the facade or add a narrow reviewed classification. Relocate hard-baseline rows with identical digests; never use baseline regeneration as the classification decision. |
| `scripts/derives_gate.py` | Every non-test function in physics, materials, and planet-substrate is keyed by repository path plus function name. | Moving a function makes its old row stale and its new path unclassified. Relocate the reviewed classification or retain its substantive `@derives` marker. Duplicate function names need distinct path rows. |
| `scripts/gen_floor_registry.py` | Every `@derives` marker under `crates/**/*.rs` is rendered with source path and line into `PHYSICS_FLOOR_REGISTRY.md`. | Regenerate and review the discovery map after any marked function moves. The generated diff should show location changes only for a pure split. |
| `scripts/diamond_gate.py --strict` | Scans public struct fields and declared providers across `crates`, while treating `laws.rs` as a special direct-kernel source. | Re-run after facade re-exports and field moves. A move must not duplicate a provider or make one visible twice. Do not split `laws.rs` until this special-path assumption is generalized with failing self-tests. |
| `scripts/determinism_gate.py` | Recursively scans canonical state crates and binds any accepted occurrence to its file path. The viewer is governed by the stronger package and source boundary instead. | Preserve zero new nondeterminism vectors. If a path carrying a classified occurrence moves, relocate the row with the original reason and count. |
| Parked derived-output liveness | The `column_convection` registry row names `crates/planet-substrate/src/geodynamics.rs`; the annotation scan is recursive. | If the marked function moves, move the registry site string in the same compatibility slice and prove the liveness test still finds one unique annotation. This is legacy gate parity, not planetary evidence. |
| Ledger inventory and canonical transcript | `scripts/gates.toml` hashes all canonical source, and the generated inventory reads canonical modules. CLI tests pin the current refusal shape and transcript output. | A canonical module split must leave inventory bytes, refusal order, schema ids, transcript text, and the direct CLI digest unchanged. |
| Rustdoc and shared-source compilation | Intra-doc links use current module paths. Retained files compile under the substrate crate and the parked simulation crate. | Repair links without changing the stated contract. Run private-item rustdoc plus both workspace compile paths. |

The baseline `doctor` command was started during this mapping pass and timed out
after 64 seconds without a verdict. That is an unresolved baseline observation,
not a pass or a failure attributable to this documentation-only slice.

### Risk summary

| Surface | Move risk | Decisive seam |
|---|---|---|
| `planet-substrate/astro.rs` | Very high | Exact-file constructor exemption, old scenario-argument methodology, 146 inline tests, and dense stellar, disk, and formation coupling. |
| `planet-substrate/geodynamics.rs` | High | One-state property atomicity, reverse test dependency on deep-time constants, marked liveness site, and shared-source dual compilation. |
| `planet-substrate/deeptime.rs` | Very high | State-hash order, conservative stock and raster folds, deterministic draw coordinates, open physical clock, and path-bound raw-bit sites. |
| `physics/moment_equivalence.rs` | High | Public facade, quadrature and fixed-point order, five path-bound raw-bit sites, two ignored tests, and open load-geometry obligations. |
| Canonical stellar birth and transcript | Very high authority risk, low need to split | Private proof seals, exact refusal frontier, schema ids, and pinned transcript bytes. Existing directories already separate most invariants. |
| Active viewer | Low size pressure, high boundary sensitivity | Only three borrowed planet artifacts are legal. The next work should add projections rather than decompose the 137-line facade. |

## Primary hotspot: `planet-substrate/src/astro.rs`

Current shape: 9,113 lines, with production through line 4,443 and 146 inline
tests. It combines reference-coordinate parsing, irradiation, orbital
relations, disk thermodynamics, collapse, cloud-core thermal balance,
multiplicity, stellar structure, spectra, activity, photoevaporation, disk
clocks, and formation outputs.

### Proposed child boundaries

| Future child | Existing responsibility it would own | Dependencies that must stay explicit |
|---|---|---|
| `astro/orbits_irradiation.rs` | Exact reference-coordinate conversion helpers, flux from luminosity, effective temperature, radiative reprocessing, Kepler period, Hill radius, and orbital mass geometry. | Sealed floor bridge, units exact rationals, Stefan-Boltzmann representation relation, and physics radiative kernels. |
| `astro/disk.rs` | Irradiated and viscous disk temperature, surface-density profiles, viscous time and accretion history, gas mean-particle reduction, and formation midplane solves. | Opacity, exact logarithmic arithmetic, collapse outputs, stellar luminosity history, and disk-state validity domains. |
| `astro/cloud_collapse.rs` | Collapse model, inside-out accretion, centrifugal radius, gas and dust thermal balance, extinction, and radiation-field estimates. | Species-state support, line cooling, radiation measure, angular-momentum history, and floor-bound mass and temperature coordinates. |
| `astro/multiplicity.rs` | Roche geometry, circumstellar component handling, resonant truncation, and truncation bands. | Realized component topology and interaction-sector authority. A familiar binary label supplies no dispatch authority. |
| `astro/stellar_state.rs` | Envelope structure, Kraft conditioning, evolutionary phase, pre-main-sequence state, rotation, Rossby activity, and spin-down state. | Open stellar-state coordinates, regime predicates, validity support, and stellar-history artifacts. |
| `astro/spectra_winds.rs` | Blackbody and departed ionizing spectra, NLTE grid handling, X-ray and EUV wind models, gravitational radius, and rate brackets. | Spectral support, atmosphere-model validity, metallicity domain, and source-custody artifacts. |
| `astro/formation.rs` | Formation epoch, disk lifetime crossing, feeding-zone mass, and planet-radius readout. | Disk history, composition support, termination law, and later Stage 2 body identity. |

Keep shared exact conversions and evaluation helpers with the invariant that
owns their rounding contract. Do not create a general `utils` child that hides
unit promotions or rearranges fixed-point expressions.

### Current direct evidence and blockers

The 146 inline tests cover deterministic arithmetic, guard behavior, published
reference oracles, interpolation domains, coupled clock composition, and many
failure paths. They are useful move guards. Most prominent fixtures are Solar,
Earth, Mars, or familiar-composition references, so this set does not establish
alien-system feasibility. There is no direct canonical Stage 1 golden because
the birth measure refuses before any raw mechanism is reachable.

The file was built under the old caller-argument methodology. Its header still
describes star mass, orbit, exponent, and similar quantities as scenario-set
arguments. Production code also contains familiar astronomical anchors,
fit-specific model structs, a standard Solar abundance route, and many raw
`Fixed` arguments. Their presence in a private candidate package preserves
work; it does not qualify them for a typed canonical adapter. Each future
adapter must bind every value to admitted authority or refuse.

The constructor gate's whole-file `astro.rs` exemption is the sharpest move
hazard. Splitting reference parsing into children silently converts exempt
constructors into new unclassified sites. The active stellar-birth registry,
support, thermal, topology, spectral, and history proofs are also open, so no
child may be advertised as Stage 1-ready after a structural split.

### Acceptance for an astro split

- Preserve all 146 test identities and results, then pass the full 372-test
  substrate package battery.
- Compile the same facade under `civsim-planet-substrate` and the parked
  compatibility crate.
- Preserve every public path visible to the parked compatibility surface while
  adding no root-workspace export.
- Review every relocated constructor, derives-baseline row, and `@derives`
  registry location.
- Preserve function bodies and fixed-point operation order. Formula repair,
  parameter typing, and floor binding land later.
- Demonstrate that no child branches on a named stellar class, familiar
  composition, Solar identity, or requested viewer result. Existing familiar
  reference tests remain reference checks only.
- Keep the canonical Stage 1 refusal and transcript bytes unchanged.

## Primary hotspot: `planet-substrate/src/geodynamics.rs`

Current shape: 3,943 lines, with production through line 2,128 and 37 inline
tests. It combines column state, coherent material-property resolution,
pressure closure, rheology, convection, SI and scaled execution, secular
cooling, field mutation, and surface readout.

### Proposed child boundaries

| Future child | Existing responsibility it would own | Boundary invariant |
|---|---|---|
| `geodynamics/column.rs` | `ColumnState`, parameter and geometry carriers, readout, and typed refusal surface. | One column state and one declared representation contract. This child owns no material dispatch. |
| `geodynamics/column_properties.rs` | Assemblage census, one-state thermoelastic ladder, density, heat capacity, expansivity, conductivity, viscosity, and pressure fixed point. | Thermal and rheological properties close as one coherent state or refuse together. Do not split them into independently selectable branches. |
| `geodynamics/convection.rs` | Linear and log Rayleigh inputs, conduction loss, scaled and SI step functions, onset state, and fixed-cap solve. | One temperature-state transition with explicit onset, residual, and representation behavior. |
| `geodynamics/secular.rs` | Secular state, source decay, one-step history, and bounded history construction. | Time evolution remains separate from spatial field projection and from viewer cadence. |
| `geodynamics/field.rs` | Populate and step `GeodynamicField`, mantle depth, surface elevation, and derived terrain tiles. | Ordered field traversal and state lineage; classification remains a readout rather than a causal selector. |

### Current direct evidence and blockers

The 37 inline tests cover one-state property coherence, capacity mixing,
pressure representability, viscosity refusals, convection direction, scaled
and SI agreement, deterministic solving, field writes, and surface readouts.
There is no external property-column golden or canonical reachability receipt.

The tests currently read the rigid-rigid critical Rayleigh value and wavenumber
from `deeptime.rs`. That reverse test dependency is a split hazard. A narrow
shared critical-convection contract must be established before moving the
tests, with its physical status audited independently. Relocating the constants
cannot turn them into admitted inputs.

`ColumnParams` and `SiColumnParams` remain raw candidate bundles. Fixed solve
caps and scaled operating coordinates must retain their present bytes during a
move, then receive separate convergence and authority review. The canonical
planet cannot import this package until a typed stage adapter binds all inputs
and refusals. The property cluster is especially sensitive: separating density,
expansivity, conductivity, and viscosity dispatch could recreate mutually
inconsistent thermodynamic states that the current atomic result prevents.

### Acceptance for a geodynamics split

- First establish the direct property-column golden required by
  `P-DIRECT-EVIDENCE`; the fixture may use existing test-only inputs but cannot
  become a canonical value source.
- Preserve all 37 tests, exact refusal variants, pressure fixed-point result,
  onset state, and field write order.
- Keep the one-state property result atomic and make partial census coverage a
  refusal at the same point as before.
- Preserve the `column_convection` derives marker and parked liveness registry
  relationship if its source path changes.
- Compile both shared-source owners and run constructor, derives, floor
  registry, determinism, Diamond, and planet-boundary checks.
- Add no Earth-material dispatch, plate label, or familiar tectonic regime.
  An unfamiliar lawful composition must follow the same registry path or return
  the same typed missing-support category.

## Primary hotspot: `planet-substrate/src/deeptime.rs`

Current shape: 3,505 lines, with production through line 1,638 and 48 inline
tests. It combines realization hashing, thermal and crust evolution, conserved
stocks, impacts, support-bound relief, stellar aging, chronology, and province
parameter construction.

### Proposed child boundaries

| Future child | Existing responsibility it would own | Boundary invariant |
|---|---|---|
| `deeptime/state.rs` | `DeepTimeState`, crater rows, state construction, read-only derived fields, and realization digest. | The exact state and hash field order. This is the replay boundary, not a place for process laws. |
| `deeptime/crust_ledger.rs` | Areal-mass conversion, melt and crust transaction, source debit, crust credit, and thickness readout. | One conservative transaction; thickness remains a view of stock rather than a second accumulator. |
| `deeptime/thermal_step.rs` | Ordered column convection step and the deep-time state transition shell. | Atomic per-tick transition and typed refusal. Physical tick derivation remains an upstream prerequisite. |
| `deeptime/impacts.rs` | Impact-flux support, deterministic draw coordinates, crater rows, basin raster, impact heat, and mass-conserving surface write. | Event identity, draw coordinate, row ordering, and cross-scale write behavior stay together. |
| `deeptime/support.rs` | Strength support bound and conservative relief redistribution. | Support residual and redistribution conservation stay together; the current missing rate, path, dissipation, and fluctuation partner stay visible gaps. |
| `deeptime/stellar_age.rs` | Main-sequence lifetime and luminosity history readouts. | Stellar evolution remains tied to a verified stellar history rather than a named familiar class. |
| `deeptime/provinces.rs` | Province-count and column-parameter construction from convective scale. | Spatial resolution derives from physical convergence and validity, never display resolution. |

### Current direct evidence and blockers

The 48 inline tests cover hash coverage, mass-ledger conservation, deterministic
thermal history, crust saturation, typed refusals, impact accounting, support
redistribution, and province construction. No direct canonical deep-time
golden exists yet.

The module's retained header states that viewer time control drives evolution.
That is legacy behavior and cannot survive a canonical adapter. The physical
integration interval and termination law are open work. `ImpactFluxParams`
and other bundles carry raw candidate values, and the impact path uses a world
identity plus tick to construct deterministic draws. A canonical draw cannot
exist until an admitted measure and approved realization-coordinate law exist.

The support-bound implementation declares its own missing transport rate,
path, dissipation, damage state, and fluctuation partner. Splitting it into a
cleaner file does not close those Gap and Residual Law obligations. The three
hard `from_bits` sites also have one path-bound constructor baseline digest,
and the state hash is sensitive to field and fold order.

### Acceptance for a deep-time split

- Land the direct deep-time column golden first, including initial and terminal
  digest, reached processes, refusal identity, conservation residuals, and draw
  coordinates when impacts are enabled.
- Preserve all 48 test identities, the realization digest, event order, crater
  row order, ledger transfer bits, and refusal variants.
- Relocate the three `from_bits` classifications with the same argument digest
  and reason.
- Keep viewer cadence absent from every new interface. The retained facade may
  preserve legacy signatures for compatibility, but no canonical adapter may
  supply a timestep before physical error control and termination derive.
- Preserve exact random-coordinate semantics in a pure move. Any rekey is a
  model change with a separate Chaos Protocol review.
- Do not add a fixed province count, Earth crust, familiar impact population,
  or named tectonic branch. Unsupported unfamiliar systems refuse at the same
  causal obligation rather than falling back.

## Primary hotspot: `physics/src/moment_equivalence.rs`

Current shape: 7,107 lines, with production through line 3,045 and 53 inline
tests. Two tests are ignored pending a physically valid shallow all-brittle lid
replacement. The public module combines load chords, sign conventions,
brittle and ductile yield, neutral-surface integration, line and axisymmetric
geometry, fixed-point solves, rigidity bands, and lid arbitration.

### Proposed child boundaries

| Future child | Existing responsibility it would own | Boundary invariant |
|---|---|---|
| `moment_equivalence/contract.rs` | Load class and chord, curvature convention, shared refusal types, readings, and common validity carriers. | Units, signs, conditioning variables, and refusal vocabulary remain explicit. |
| `moment_equivalence/yield_envelope.rs` | Mohr-Coulomb brittle band, creep-based ductile band, transition selection, envelope traits, and edge views. | Brittle and ductile branches plus their transition form one contract and must not become independently selectable modules. |
| `moment_equivalence/moment_integral.rs` | Fibre stress, axial-force residual, neutral-surface solve, bounded moment integral, and tail status. | Quadrature order, neutral-surface residual, and domain-limited refusal remain one invariant. |
| `moment_equivalence/line_load.rs` | Line-load curvature, load-demand constant, and line-specific adapter. | Cylindrical-bending geometry and its sign convention. |
| `moment_equivalence/axisymmetric_load.rs` | Kelvin-function coefficients, point-load curvature, reported curvature, and disc-equivalence band. | Axisymmetric moment operator and its stated approximation domain. It does not claim finite-disc closure. |
| `moment_equivalence/solver.rs` | Shared rigidity bracket, fixed-point iteration, edge combination, and convergence result. | Evaluation order, bracket endpoints, and residual identity. |
| `moment_equivalence/plate.rs` | Settled and banded plate outputs, rigidity conversion, and display thickness readouts. | Rigidity remains the physical output; thickness remains a conditioned readout. |
| `moment_equivalence/lid.rs` | Conductive lid base, column assembly, convective-stress referee, and verdict attachment. | The lid validity decision stays separate from load geometry and from viewer classification. |

### Current direct evidence and blockers

The 53 tests exercise published envelope examples, neutral-surface behavior,
integral convergence, line and axisymmetric solves, representation migration,
rigidity bands, refusal surfaces, and lid arbitration. The two ignored tests
mean this battery is not a complete direct evidence claim. `P-DIRECT-EVIDENCE`
still requires a named moment-equivalence golden with residuals.

Five hard `from_bits` sites share one constructor-baseline row. Many functions
are path-keyed in the derives baseline, and ten substantive `@derives` markers
feed the generated floor discovery record. The module is publicly reachable
from `civsim-physics`, and `flexural_relief`, hindcast comparison, flexure docs,
and other modules use its types. A parent facade must re-export the identical
surface without duplicate providers.

The existing `LOADKIND_TAXONOMY_AUDIT.md` records finite-footprint, point-sign,
conservation, validity, admissibility-bound, and alien-ice concerns, while its
own header says its source citations have not all been re-verified. Those are
leads to verify against current source before any semantic repair. The live
queue independently keeps finite two-dimensional loads, typed load geometry,
finite-disc treatment, and the `D -> 0` limit open. A pure split may preserve
known behavior; it cannot declare the load arc complete.

### Acceptance for a moment-equivalence split

- Land the direct load-equivalence golden first, recording envelope branch,
  neutral-surface and moment residuals, solver branch, rigidity bits, validity,
  and refusal identity.
- Preserve all 53 test identities and both ignored statuses in a pure move.
  Replacing the ignored fixtures is a separate evidence slice.
- Preserve every public path, visibility, rustdoc link, and downstream type
  identity through the parent facade.
- Relocate the five `from_bits` classifications and every moved derives row
  without changing their reasons or digests.
- Preserve quadrature, bisection, fixed-point, and band-edge order exactly.
- Keep finite-disc, arbitrary-field, shell-validity, and conservation gaps
  explicit. Do not add a closed world-load taxonomy or treat a solver strategy
  enum as the set of loads an alien planet may contain.

## Canonical stellar-birth and transcript modules

This area is already the strongest modular example in the active tree. The
`stellar_birth_structure/` directory separates component, species, index,
carrier, coordinate, sector, regime, classification, stellar-state, view, and
wire contracts. The species authority analysis already separates model, view,
and wire. Those directories should remain intact unless a concrete invariant
crosses one of their present boundaries.

### Keep cohesive now

| File or directory | Present invariant | Current evidence | Constraint |
|---|---|---|---|
| `stellar_birth_measure.rs` | Exact conjunction of the joint-measure and coordinate-law leaves, ordered open frontier, and typed refusal. | 8 unit tests. | Keep cohesive. Splitting leaf declarations from evaluator logic would add navigation without separating authority. |
| `stellar_birth_artifacts.rs` | Sealed proof-capability carriers and repository resolver. | Compile-time seal plus measure tests. | Keep private; production resolver remains empty until verified artifacts exist. |
| `stellar_birth_structure/` | Value-free open registry and stellar-state schemas with private validation and read-only views. | 12 focused tests across facade and wire, including mutation and unfamiliar-state checks. | Preserve child boundaries and schema id `civsim.planet.stellar-birth-structure.v2`. A schema rule is not a realized member or physical value. |
| `stellar_birth_species.rs` | Exact complete-support verification and mean-particle-mass reduction over sealed candidates. | 6 unit tests, including permutation and unfamiliar-state behavior. | Keep the reducer cohesive. Production returns no support and no packet path may construct its seals. |
| `stellar_birth_species/authority_analysis/` | Permanently non-admitting analysis bound to the floor and open proof frontier. | 3 unit tests plus typed read-only views. | Keep separate from the future admitting authority. Analysis success cannot mint support. |
| `stellar_birth_species/physical_registry/` | Conditional admitted-artifact proof graph, exact mass and dimension validation, and complete species closure by bottom-up producer and top-down watchdog. | 11 focused tests cover the exact repository refusal, unfamiliar and massless graphs, admission routes, closure and dependency defects, order invariance, exact arithmetic, depth, resource caps, and non-authority reachability. | Keep separate from conditioned support and the reducer. Repository production has no roots, agreement has no authority effect, and synthetic validation cannot mint membership. |
| `stellar_birth_species/support_packet/` | Bounded canonical species descriptors and conditioned-support packet validation by independent structural algorithms. | Focused synthetic unfamiliar, massless, mutation, coverage, canonical-order, and resource-refusal tests. | Keep separate from the reducer and physical authority. Packet agreement binds bytes only and cannot prove membership, mass ancestry, support, or closure. |
| `requirement_analysis.rs` | Read-only projections over non-admitting census and species analysis. | Exercised by measure, receipt, and CLI tests. | Keep construction private. New viewer needs should add narrow getters rather than expose payload constructors. |

### Candidate splits after the authority arc stabilizes

`stellar_birth_dimensions.rs` has about 1,010 production lines and 9 tests. A
future split may place value-free variable and phenomenon declarations in
`stellar_birth_dimensions/spec.rs`, exact matrix and RREF assembly in
`evaluator.rs`, and artifact validation in `validation.rs`. The parent retains
schema id `civsim.planet.stellar-birth-dimensional-census.v4`, checker id
`civsim.units.exact-si-rref.v2`, construction privacy, and the one public view
path. This is lower priority than completing the species, state, support,
thermal, history, topology, and spectral proofs the census reports as open.

`transcript.rs` has 1,652 lines and 9 tests. A future split may separate sealed
event and record models, validation and closure, and deterministic wire writers
under a parent facade. Writer modules should follow semantic record groups,
rather than one helper per field. The pure-move acceptance is unusually strict:
schema id `civsim.planet.transcript.v9`, event order, escaping, receipt text,
367,628-byte direct CLI output, and its pinned SHA-256 all remain exact.

The complete canonical stellar slice has 8 direct CLI tests in
`crates/planet/tests/canonical_cli.rs`. Those tests pin refusal exit status,
event count, schema versions, the two-leaf frontier, typed views, repeatability,
and rejection of world, profile, and seed arguments. Any canonical file split
must run those tests plus the ledger-inventory check and the complete planet
unit suite.

### Open authority blockers

- The joint physical measure and realization-coordinate law remain absent.
- The realized species registry contains zero members and verified support is
  absent; the dormant reducer cannot supply a value.
- Dimensional reachability and structure schemas are discovery and validation
  artifacts with `closure_effect=none`. They do not prove a physical law.
- Named classes such as Population III, magnetar, Terran, or thaumic remain
  noncausal presentation labels unless and until derived state satisfies a
  read-only classifier. No class can select a law, sector, value, or branch.
- A split must preserve private seals, floor digest binding, checker binding,
  dependency digest, canonical ordering, and exact refusal frontier. A new
  public constructor or forgeable proof token fails the slice.

## Active viewer surface

Baseline shape: `crates/viewer/src/lib.rs` is 137 lines with 3 tests. It adapts
a sealed `PlanetObservation`, borrows a `PlanetSnapshot`, exposes an immutable
refusal receipt view, and returns a startup refusal while snapshot input is
unwired. The observer-only scene slice added beside this map uses child modules
for distinct projection invariants, not to split this small facade for size.

The next safe implementation can add children by observer invariant:

| Future child | Allowed role | Forbidden role |
|---|---|---|
| `projection/refusal.rs` | Build immutable scene data from typed stages, refusals, obligations, dimensional census, species analysis, Gap Law, Chaos Protocol, and residual views already exposed by the receipt. | Parsing canonical text as authority, promoting a refusal, or filling an absent physical value. |
| `projection/floor.rs` | Present the declared floor-event envelope. Present quantity, bits, tier, or provenance only after planet owns and exposes a sealed read-only value-record view. | Inferring payload accounting from an event name, editing a magnitude, treating a tier or provenance mark as admission, or adding a viewer-side floor row. |
| `projection/scene.rs` | Generic scalar, vector, tensor, spectrum, field, topology, and graph display carriers with stable display identity. | Selecting physical resolution, a realization coordinate, or a familiar class branch. |
| `adapter.rs` | Convert the three permitted borrowed planet types into observer-side views. | Calling `run_planet`, receiving `PlanetRunOutcome`, constructing planet artifacts, or taking owned snapshot or receipt inputs. |

Renderer, camera LOD, materialization, content-addressed storage, and universe
search are later layers in `DERIVATION_AND_VIEWER_PIPELINE.md`. They should not
be stubbed into the current crate as causal-looking placeholders.

The first scene slice now exposes transcript order, exact noncausal SI
representation integers, stage reachability, the refusal frontier, and its
non-admitting analyses. Floor quantity identity, exact value bits, tier, and
provenance are explicitly opaque at the viewer boundary. The planet-owned
public-API evidence test inspects those value records directly; the viewer does
not duplicate the ledger taxonomy or guess it from event identities.

The viewer boundary gate is intentionally narrow. It rejects grouped and
aliased planet imports, non-observer planet APIs, owned snapshot and receipt
inputs, observation construction, mutable borrows, causal method names, legacy
viewer symbols, authored orbit or composition, physical timestep inputs, and
all runtime dependencies except planet. A projection should adapt to this
boundary rather than weaken it.

Acceptance for the first projection slice:

- Keep the current 3 tests and add typed projection tests over the existing
  Stage 1 refusal.
- Prove projection output is deterministic under repeated reads and that read
  order does not mutate or close the receipt.
- Use typed getters. If a getter is missing, add the narrowest read-only view in
  planet with private construction and a boundary self-test.
- Keep the viewer binary unwired unless it receives a sealed observation from
  an explicit transport. Startup without one continues to refuse visibly.
- Keep camera, query, taxonomy, display units, palette, hardware budget, and
  cache state outside every causal byte and refusal decision.
- Render unfamiliar carriers through their declared shape and metadata. A
  missing familiar label is not a rendering refusal.

## Secondary active hotspots

These files remain in the queue but should follow the primary evidence work.

| File | Future boundary direction | Present blocker |
|---|---|---|
| `physics/src/flexure.rs` | Validity, scaled representation, special functions, Green functions, field response, and tests. | The gate treats `laws.rs`, not flexure, specially, but moment-equivalence and flexure share direct evidence. Finite-disc, arbitrary field, `D -> 0`, shell validity, conservation, and audited bounds remain open semantic work. |
| `physics/src/opacity.rs` | Mechanism-specific opacity regimes, mixing, aggregate dispatch, and tests. | Branch validity, table provenance, interpolation order, and active evidence debt must stabilize before a pure split. |
| `planet-substrate/src/giants.rs` | Formation, disk budget, termination, classification, and tests. | Disk measure, gas ledger, physical termination, and Stage 1 history are not admitted. Classification must remain a readout. |
| `physics/src/laws.rs` | Domain modules only after gate generalization. | Floor-registry and Diamond tooling hardcode the file as the direct-kernel source. Moving first would create a coverage hole. |
| `planet-substrate/src/geodynamics_surface.rs` and `flexural_field.rs` | Keep cohesive until their field contracts grow. | They are already small facades over one field transition. Splitting them now adds files without separating an invariant. |

## Recommended split order

1. Finish audit disposition. Verify each finding against the current source and
   separate semantic defects from move hazards.
2. Land direct property, deep-time, moment-equivalence, and flexure goldens with
   typed refusals, branch identity, residuals, conservation, provenance hops,
   and replay records.
3. Extract inline tests by the proposed invariant groups. Run the unchanged
   production files against the new test layout before moving production.
4. Split `moment_equivalence.rs` behind its public facade. It is independent of
   canonical Stage 1 wiring, but waits for its direct golden and gate-baseline
   migration plan.
5. Split geodynamics properties and convection, keeping their coherent result
   atomic. Resolve the reverse test dependency on deep-time critical values.
6. Split deep-time state, ledger, impacts, support, and provinces. Preserve the
   facade and shared-source compatibility while keeping canonical activation
   blocked on integration control and realization authority.
7. Split `astro.rs` only after the current stellar-birth authority work has a
   stable baseline. Its constructor exemption and dense cross-module coupling
   make it the highest path-migration risk.
8. Leave the existing stellar structure and species analysis directories
   alone. Split dimensions or transcript only when concurrent authority work
   no longer overlaps and exact byte pins can guard the move.
9. Grow the active viewer through new observer-only projection children. Do
   not recover causal code by moving it out of the parked viewer.

## Per-split acceptance checklist

Every future split should record these results in its handoff:

- [ ] One named invariant moved, with no formula, value, branch, or schema
  change mixed into the commit.
- [ ] Public and crate-private item paths were inventoried before and after;
  no visibility broadened and no proof token became constructible.
- [ ] Focused tests and the direct golden pass with identical branches,
  refusals, residuals, conservation, hashes, and bytes.
- [ ] Test count and ignored-test count remain unchanged for a pure move.
- [ ] Root substrate sources compile and the parked shared-source consumer
  compiles separately.
- [ ] Constructor and determinism baselines moved only where the inspected site
  moved, retaining count, digest, and reason.
- [ ] Derives classifications and substantive markers remain one-to-one;
  `PHYSICS_FLOOR_REGISTRY.md` was regenerated and reviewed.
- [ ] Diamond, floor-provenance, planet-boundary, gate-runner, and relevant
  self-tests pass.
- [ ] Canonical ledger inventory, refusal frontier, transcript schema, CLI
  bytes, and snapshot or startup refusal remain exact where applicable.
- [ ] Private-item rustdoc passes after link repair.
- [ ] `cargo fmt --all --check`, targeted Clippy, targeted tests, and the full
  canonical PR tier pass. Shared-source changes also pass the separately
  reported legacy compatibility tier.
- [ ] The diff introduces no named familiar-world dispatch, authored physical
  default, viewer-to-causal edge, or fallback for an unfamiliar lawful case.

A failed item stops the move. If preserving behavior would preserve a known
semantic defect, land the structural move with that defect still explicit,
then repair it in a separately versioned and audited semantic slice.
