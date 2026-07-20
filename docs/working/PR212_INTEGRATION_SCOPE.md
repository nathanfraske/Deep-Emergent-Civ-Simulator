# PR #212 integration scope: what landed, what is unread, and the path to a visible result

A scoping pass over PR #212 (`claude/abiotic-field-registry-lmqbqd`), head `a559e034`, base `origin/main` at
`e29df9a9`. Nothing here is implemented; this document is the plan and the finding list. The companion
six-blocker adversarial audit is `docs/working/PR212_ARC_AUDIT.md`, and this pass assumes it rather than
repeating it.

**Reference discipline for the line numbers below.** File:line citations for #212's own files are against the
PR head `a559e034`. Citations for the receiving surface (the viewer, the calibration manifest) are against
`origin/main` at `e29df9a9`. Because #212 touches no viewer file, `origin/main`'s viewer IS the post-merge
viewer, so those citations hold after the merge. Line numbers taken from the current local working tree
(branch `claude/thermoelastic-rung3`, 23 ahead and 18 behind `origin/main`) are NOT used here: that tree
predates #210 and #211 and would misreport both symbol existence and line positions.

**Verified state at the time of writing.** CI on `a559e034` is green (`build, format, lint, test`, `Stone 0
gate`, and `documents and prose customs` all `success`; nightly skipped). The PR merges onto `origin/main`
with zero conflicts: `git merge-tree --write-tree origin/main a559e034` exits 0 with a single clean tree,
and the old-form merge-tree produces no conflict markers. `mergeable_state` is `clean`.

---

## 1. What #212 built, verified against the diff

The PR body's "What landed" list is STALE and must not be used as the inventory. Commit `116116d` ("Audit
remediation 1/N: revert the unsound DiskEvolutionState and the dead EUV phase enum") withdrew two of the four
capabilities the body advertises, and the body was never reconciled. Verified by grep against the head
commit, these seven symbols exist in ZERO files at `a559e034`: `DiskEvolutionState`,
`derive_disk_evolution_state`, `DiskEvolutionInterims`, `EuvDispersalPhase`, `euv_dispersal_phase`,
`EuvPhaseTransition`, `DepartureArchiveGrade`.

The real diff is 10 files, +1063/-61, of which three are Rust: `crates/sim/src/astro.rs` (+257),
`crates/sim/src/giants.rs` (+593), and `crates/sim/src/planetary_assembly.rs` (+1, a test-only `.flatten()`).

### 1a. New public surface in `crates/sim/src/giants.rs`

| Symbol | Site | What it is |
|---|---|---|
| `TruncationSink` | `giants.rs:584` | Closed 3-variant enum: `CompanionAccretion`, `CircumbinaryReservoir`, `ViscousSpreadInward`, plus `tag()` returning a `&'static str` label |
| `TruncationResidualReading` | `giants.rs:613` | Sum type: `InitialCondition`, `DynamicOutflow { sink }`, `Unresolved` |
| `TruncationResidualDisposition` | `giants.rs:636` | Result struct carrying retained gas, system budget, optional sink label, and a `residual_conserved` flag |
| `TruncationResidualRefusal` | `giants.rs:656` | Refusal enum: `ReadingUnselected { .. }` carrying both candidate budgets, and `InconsistentLedger` |
| `dispose_truncation_residual` | `giants.rs:688` | The disposition function, validating the ledger before reading it |

### 1b. Changed behaviour in `crates/sim/src/giants.rs`

`truncation_gas_ledger` (`giants.rs:498`, pre-existing from the #210/#211 lane) was rewritten. The straddling
ring at the truncation radius is now split into two sub-rings, each integrated at its own midpoint, through a
`subring` closure at `giants.rs:512`. This is the correct fix for the proxy angular momentum partition, since
`L = m * sqrt(r)` weights radius non-linearly and a single mass-fraction split corrupts the `L` partition.
The change is real and is covered by a NON-circular test
(`the_truncation_ledger_partitions_mass_and_angular_momentum_separately`, `giants.rs:2861`) that builds an
independent f64 reference integral rather than asserting against the implementation.

Three kernels gained the same two guards: a `steps > i32::MAX as u32` refusal (the ring index is formed
through `Fixed::from_int(i32)` and would otherwise wrap negative) and a `dr <= Fixed::ZERO` refusal (the span
divided by a very fine step count truncates to zero). The sites are `feeding_zone_gas_mass_earth`
(`giants.rs:352`), `disk_gas_content` (`giants.rs:408`), and `truncation_gas_ledger` (`giants.rs:498`).

The same three kernels reordered the ring product from `((2*pi*r) * Sigma) * dr` to `((r * dr) * 2*pi) *
Sigma`, which keeps the intermediate bounded for a wide dense ring whose completed product is representable.
The fixture that exercises it (`a_low_alpha_disc_gas_integral_survives_the_intermediate_overflow`,
`giants.rs:2815`) first ASSERTS that the old ordering overflows, so it cannot pass vacuously.

`giant_formation` (`giants.rs:851`) replaced `.unwrap_or(Fixed::ZERO)` with `?` on the feeding-zone gas
integral, so a failed reservoir integration now refuses the whole verdict instead of reporting a core-only
giant.

`giant_formation_field` (`giants.rs:927`) changed its public signature from `Vec<GiantVerdict>` to
`Vec<Option<GiantVerdict>>`. This is a BREAKING API change; it preserves refusals in place rather than
silently dropping them.

### 1c. New public surface in `crates/sim/src/astro.rs`

| Symbol | Site | What it is |
|---|---|---|
| `HerbigEuvDepartureGrid` | `astro.rs:2673` | Struct holding `[(Fixed, Fixed); 16]` grid points plus `metallicity_z_solar` and `log_g_cgs` coordinates |
| `HerbigEuvDepartureGrid::bstar2006_svo` | `astro.rs:2685` | Constructor with the 16 `(T_eff, log10 departure)` literals, solar Z, log g 4.0 |
| `HerbigEuvDepartureGrid::departure_log10_at` | `astro.rs:2714` | Log-space linear interpolation in `T_eff`, refusing outside 15000 to 30000 K |
| `windless_herbig_departed_spectrum` | `astro.rs:2745` | Applies the departure to a blackbody spectrum through the reused `nlte_departed_ionizing_spectrum`, gated on `(Z, log g)` |

### 1d. Data and provenance

Two new `[[source]]` blocks in `sources/registry.toml`: `bstar2006_lanz_hubeny` (the paper, witness custody,
sha256 `e5435bab...`, DOI corrected from the wrong `10.1086/511268` to `10.1086/511270`) and
`svo_tlusty_bstar2006` (the 16 emergent SEDs, witness custody, 16 sha256 receipts, all 16 now carrying
Wayback timestamps). One `used_by` field corrected in
`crates/physics/data/disk_arc_literature/manifest.toml`, retracting the diffuse-to-direct phase claim to
"NOT wired". `docs/working/BSTAR2006_HERBIG_EUV_DEPARTURE.md` is new (186 lines, the derivation of record).
`docs/working/PHYSICS_FLOOR_REGISTRY.md` moved from 41 to 43 deriving substrates.

### 1e. Three stale claims in the PR body, verified against source

These are recorded because the task is to verify the body rather than trust it, and because a stale body is
what a later reader will read first.

1. The body lists "#1a Diffuse/direct EUV phase (`35cf765`): `EuvDispersalPhase` + `euv_dispersal_phase`" and
   "#3 Canonical `DiskEvolutionState` (`d507b38`, dormant)" under **What landed**. Both were reverted by
   `116116d`. Zero occurrences at head.
2. The body's Verification section claims "floor registry regenerated (45 deriving substrates)". The
   regenerated file at head says 43 (`PHYSICS_FLOOR_REGISTRY.md`, the "The 43 deriving subsystems" line). The
   45 was true before the reversion removed two markers.
3. The body's **Owed by the coordinator (the addendum)** section says the two `archive_pending` points (fid
   590 and 650) still owe a Wayback SAVE-retry. The registry at head records that retry as done, all 16
   witnessed, and the `DepartureArchiveGrade::ArchivePending` machinery retired from the code.

The author's own `HANDOFFS.md` entry is accurate and current; the divergence is confined to the PR body.

---

## 2. What is and is not wired

**Every one of #212's new capabilities is banked-but-unread. There are zero production consumers.**

This was established by grepping the head commit for each new symbol across all `.rs` files and then
partitioning the hits by the test-module boundary (`#[cfg(test)] mod tests` begins at `giants.rs:1558` and
`astro.rs:4447`).

| Symbol | Files containing it | Non-test references |
|---|---|---|
| `dispose_truncation_residual` | `giants.rs` only | Definition (`:688`) and doc links only |
| `TruncationResidualReading` | `giants.rs` only | Definition (`:613`) and doc links only |
| `TruncationSink` | `giants.rs` only | Definition (`:584`) and doc links only |
| `TruncationResidualDisposition` | `giants.rs` only | Definition (`:636`) and doc links only |
| `truncation_gas_ledger` | `giants.rs` only | Definition (`:498`) and one doc link (`:670`) |
| `HerbigEuvDepartureGrid` | `astro.rs` only | Definition (`:2673`) and doc links only |
| `windless_herbig_departed_spectrum` | `astro.rs` only | Definition (`:2745`) and doc links only |
| `bstar2006_svo` | `astro.rs` only | Definition (`:2685`) only |

No file outside the defining module names any of them. All call sites are inside the two test modules.

### The dormancy is structural, and deeper than #212

The finding that matters for sequencing is that #212's work sits on top of a lane that is ITSELF unread, two
levels down from anything the run path executes.

**The giants lane.** `crates/sim/src/giants.rs` is referenced by exactly one other module,
`crates/sim/src/planetary_assembly.rs` (`:64`). `planetary_assembly` in turn is referenced by nothing except
its own `pub mod` declaration in `crates/sim/src/lib.rs:106` and three prose mentions in
`crates/sim/src/secular.rs`. So the giants and planetary-assembly modules together form a closed island: no
binary, example, or other module calls into them.

**The EUV lane.** The entire ionizing-spectrum and photoevaporation chain
(`blackbody_ionizing_spectrum`, `nlte_departed_ionizing_spectrum`, `EuvWindFit`,
`IonizingSpectrumEvaluation`, `radiative_euv_photoevaporation_wind_rate_msun_myr`) has no reference outside
`crates/sim/src/astro.rs` at all. `radiative_euv_photoevaporation_wind_rate_msun_myr` is defined at
`astro.rs:4060` and referenced only from `astro.rs:7037` onward, which is inside the test module. #212's
Herbig departure grid is therefore a leaf on a branch that is itself entirely dormant.

**The run path and the viewer.** `crates/sim/examples/run_world.rs` imports no `astro`, `giants`, or
`planetary_assembly` symbol. `crates/viewer/src/main.rs` imports `civsim_sim::deeptime`,
`civsim_sim::genesis`, and `civsim_sim::geodynamics`, and it DOES reach `civsim_sim::astro` (see section 3),
but it reaches a disjoint set of astro functions. `crates/sim/src/deeptime.rs` imports no astro symbol at
all; its imports are geodynamics, materials, melting, ballistic, crater, impact, redistribute, and terrain.

### What this means for byte-neutrality

The PR's claim that both pins are bit-exact is structurally sound, and the reason is the dormancy rather than
numerical invariance. The ring-product reorder in section 1b DOES change the fixed-point result of
`disk_gas_content` and `feeding_zone_gas_mass_earth` in the general case, because Q32.32 multiplication
rounds and the intermediate ordering differs. The author acknowledges a related consequence directly: the
test `the_ledger_total_matches_the_unpartitioned_gas_content` (`giants.rs:2617`) was changed from exact
equality to a 1e-6 relative tolerance, with a docstring explaining that the sub-ring refinement broke the
prior bit-identity. The pins hold because no production path reads these kernels. A future consumer will get
different numbers than the pre-#212 ordering would have produced, which is correct (the reorder is a fix) but
should be recorded so it is not later mistaken for a regression.

---

## 3. The path into the viewable sim

### 3a. What the viewer shows today, and where the disk already reaches it

The viewer is NOT disk-blind. `crates/viewer/src/main.rs` runs a live Lynden-Bell-Pringle disk clock in
production, landed by the slice-3b-ii wire whose design of record is
`docs/working/DISK_EVOLUTION_SLICE3B_WIRE_SCOPE.md`. The chain, all on `origin/main`:

1. `derive_pre_ms_bolometric_luminosity` (`main.rs:1625`) derives `disk_T(R_1)` from the closed-form
   constant-opacity midplane, then `t_visc` through `derive_viscous_time_myr`, then the formation epoch
   `t_formation` as a bisection root through `derive_formation_epoch_myr`, then reads the Hayashi wall
   `T_eff` per star from the vendored BHAC15 grid, and returns the pre-main-sequence bolometric luminosity
   plus a `FormationRateConsistency` verdict.
2. `derive_formation_condensation_temperature` (`main.rs:1737`) feeds that luminosity into
   `formation_midplane_temperature` at each orbit and snaps the result to the condensation grid.
3. The condensation temperature sets the condensed density and the surface composition, which reach the
   rendered globe.

So the seam where a disk result becomes a visible pixel already exists, and it is one function deep. That is
the good news for integration.

### 3b. Why #212's results cannot reach that seam as built

Neither capability can be wired to visibility without building something that does not exist. This is by
design in one case and by omission in the other.

**The truncation disposition refuses by construction.** `dispose_truncation_residual` returns
`Err(TruncationResidualRefusal::ReadingUnselected { .. })` for `TruncationResidualReading::Unresolved`
(`giants.rs:727`), and `Unresolved` is the honest default because which physical reading holds is DERIVED
from the binary's formation history, which no substrate models. The doc at `giants.rs:487` states this
plainly. To make the disposition produce a number, a caller must supply `InitialCondition` or
`DynamicOutflow { sink }`, and choosing either without the formation-history substrate is exactly the
value-authoring the refusal exists to prevent. Wiring this to the viewer today would mean authoring the
selection. It should not be done.

Beyond the reading, the disposition needs a binary companion at all: a mass ratio, a separation, and an
eccentricity feeding the Pichardo truncation relation to produce `truncation_radius_au`. The viewer renders a
single star whose mass is a CLI argument (`main.rs:464`, defaulting to `Fixed::ONE`). There is no companion
in the scene, no `TruncationEvaluation` construction anywhere in the viewer, and no place in the render that
would show a truncated disk.

**The Herbig departure grid is gated shut for any derived star.** `windless_herbig_departed_spectrum`
refuses unless the star's coordinates match the grid slice EXACTLY:

```rust
if star_metallicity_z_solar != grid.metallicity_z_solar || star_log_g_cgs != grid.log_g_cgs {
    return None;
}
```
(`astro.rs:2755`)

This is exact `Fixed` inequality, so only a caller passing literally `Fixed::ONE` and `Fixed::from_int(4)`
passes. That gate is the correct remediation of audit blocker 4 and it is principled; the consequence is that
the capability is unreachable from any derived path. Two independent reasons:

- **There is no stellar `log g` producer anywhere in the repository.** Grepping all `.rs` files at head for
  `log_g`, `log10_g`, and `surface_gravity_cgs` outside `astro.rs` returns exactly two hits, both a local
  variable in `crates/physics/src/molecular_opacity.rs:395` that means something unrelated. No code derives a
  star's surface gravity, so the `star_log_g_cgs` argument has no supplier and any caller must author it.
- **The viewer's star is nowhere near the grid domain.** The grid spans 15000 to 30000 K. The viewer's
  default star is one solar mass, whose effective temperature is roughly 5772 K, well below the 15000 K
  floor, so `departure_log10_at` refuses on `T_eff` before the coordinate gate is even consulted.

And even if both gates passed, the departure feeds `nlte_departed_ionizing_spectrum`, which feeds
`radiative_euv_photoevaporation_wind_rate_msun_myr`, which has no production consumer (section 2). The
photon rate would be computed and discarded.

### 3c. The concrete wiring, if the owner wants it

Stated as the honest ladder rather than a single step. Each rung is a real build.

**For the EUV departure to change a visible pixel:**

1. Derive stellar surface gravity `log g` from the star's mass and radius, which the stellar module already
   carries. This is a small, well-posed derivation and it retires the authored-argument problem.
2. Build the `(Z, log g)` interpolated lookup over the full BSTAR2006 grid (6 compositions by 13 gravities by
   16 temperatures, 1248 points), the rung `astro.rs:2657` already names. This must be a data column, not
   Rust literals (see section 5).
3. Build the windless-versus-windy crossover selector for the 25000 to 30000 K overlap, keyed on the star's
   derived wind strength. The doc at `astro.rs:2748` names this as a derivation gap.
4. Wire `radiative_euv_photoevaporation_wind_rate_msun_myr` to a disk-dispersal clock so the photoevaporative
   mass-loss rate shortens the disk lifetime.
5. Couple that lifetime back into `derive_pre_ms_bolometric_luminosity`, so a disk dispersed early truncates
   the formation epoch and therefore the condensation temperature at each orbit.

Only at rung 5 does the globe change, and only for a star hot enough to matter. Rung 5 is where the viewer's
byte pins move, so it is owner-gated.

**For the truncation disposition to change a visible pixel:**

1. Build the binary-formation-history substrate (companion origin and timing), which selects the reading.
   This is the named blocker and it is a real modelling arc, not a wiring step.
2. Put a companion into the rendered scene, with the mass ratio, separation, and eccentricity the Pichardo
   relation consumes.
3. Wire `truncation_gas_ledger` and `dispose_truncation_residual` into the disk state, so the retained gas
   budget caps the disk the formation clock reads.
4. Couple the capped budget into the formation epoch and the isolation mass, which is where it would reach
   the globe.

Rung 1 alone is larger than #212. The realistic reading is that this capability is banked for a future arc
and should be evaluated on the quality of the substrate rather than on time-to-visibility.

**The one cheap visible thing.** If the goal is that #212 show SOMETHING to a human, the achievable step is a
provenance-readout line rather than a physical change: the `--derived` globe's readout already surfaces what
the render rests on (design of record section 5), and it could state that the disk clock runs untruncated and
that the Herbig EUV branch is off-domain for this star. That is honest, cheap, and moves no pixel and no pin.

---

## 4. Conflict and sequencing

### 4a. #212's touch-set

Ten files: `crates/sim/src/astro.rs`, `crates/sim/src/giants.rs`, `crates/sim/src/planetary_assembly.rs`,
`sources/registry.toml`, `crates/physics/data/disk_arc_literature/manifest.toml`, `docs/SOURCES.md`,
`docs/working/BSTAR2006_HERBIG_EUV_DEPARTURE.md` (new), `docs/working/CONSENSUS_ROADMAP.md`,
`docs/working/PHYSICS_FLOOR_REGISTRY.md`, and `HANDOFFS.md`.

### 4b. Collision with the two named lanes: NONE

The task asks specifically about (a) the stagnant-lid Nusselt work in
`crates/physics/src/convection_scaling.rs` and (b) the flexure and Seam D work in
`crates/physics/src/moment_equivalence.rs` and `crates/physics/src/flexure.rs`.

**#212 touches none of those three files, and does not touch `crates/sim/src/deeptime.rs` or
`crates/viewer/src/main.rs` either.** The `GEODYNAMICS_LANE_MAP.md` serialization constraint (that lanes B,
C(ii) and D all converge on `deeptime.rs` plus `viewer/main.rs`) does not bind #212 as it stands, precisely
because #212 is dormant. The collision the lane map warns about would arise at the FUTURE integration step
described in section 3c, not at this merge.

Both lanes are currently LOCAL AND UNPUSHED, on the working-tree branch `claude/thermoelastic-rung3`. Against
`origin/main` versus that branch: `convection_scaling.rs` 350 to 559 lines, `flexure.rs` 1466 to 2672,
`moment_equivalence.rs` 5781 to 7178, `deeptime.rs` 2178 to 2785. No open PR contains any of them, so they
cannot collide with #212 through the PR queue at all.

The one file where they DO meet is `docs/working/PHYSICS_FLOOR_REGISTRY.md`. #212 edits it (+6/-4, the 41 to
43 count plus two `@derives` entries), and the local branch has an uncommitted modification to the same file.
That is a regenerated artifact (`scripts/gen_floor_registry.py`), so the resolution is to regenerate rather
than to merge by hand.

### 4c. Collision with the open PR queue

Fifteen PRs are open. The load-bearing overlap is narrow:

- **`crates/sim/src/astro.rs` is the only real code contention.** #212 adds +257/-1 there. PR #175
  (`claude/genesis-arming-step`) adds +134, and PR #191 (`claude/kappa-r-assembly`, a draft) adds +371/-17.
  Both are stale against a main that has moved; #191 and #190 are additionally based on
  `claude/property-emission`, which is already an ancestor of `origin/main`, so their bases need retargeting
  before their true conflict surface is even knowable.
- **`crates/viewer/src/main.rs` and `render.rs`** are claimed by PR #173 (`claude/viewer-solar-system-zoom`),
  which touches only those two files. #212 does not touch them, so there is no conflict now; there WOULD be
  one at the section 3c integration step.
- **No open PR touches** `giants.rs`, `planetary_assembly.rs`, `deeptime.rs`, `convection_scaling.rs`,
  `moment_equivalence.rs`, `flexure.rs`, `sources/registry.toml`, the disk-arc manifest, or `docs/SOURCES.md`.
  #212 is the sole claimant on all of them.
- The remaining overlap is ledger churn in `HANDOFFS.md`, `CONSENSUS_ROADMAP.md`, and
  `PHYSICS_FLOOR_REGISTRY.md` (PRs #182, #178, #155, #204, #193), which is line-level and mechanical.

### 4d. Recommended merge order

1. **#212 first, and soon.** It merges clean today, CI is green, it is the sole claimant on eight of its ten
   files, and it is dormant so it cannot destabilize any run path. Every day it waits, its `astro.rs` delta
   ages against #175 and #191. Merging it first makes #212 the base those two rebase onto, which is the
   cheaper direction: #212's astro addition is a self-contained block (one struct, one impl, one function)
   that will rebase cleanly under other changes, whereas holding it means re-verifying a 28-commit branch
   against a moved main.
   The gate before merge is the re-gate the roadmap and `HANDOFFS.md` both ask for, plus the section 5 items
   below. The roadmap line at `CONSENSUS_ROADMAP.md:11` says "awaiting coordinator RE-GATE, do NOT merge",
   which is correct and should be honoured; this document is input to that re-gate, not a substitute for it.
2. **Then the local geodynamics lanes (A and B), which are file-disjoint from #212** and are blocked only on
   being pushed. Their `PHYSICS_FLOOR_REGISTRY.md` edit regenerates over #212's.
3. **Then #175 and #191 rebased onto the new main**, in that order (#191 is a draft and the larger of the
   two, at 186 files).
4. **Then #173**, the viewer lane, which must be serialized against any later #212 integration work because
   both would own `viewer/main.rs`.
5. **The section 3c integration work last, and separately scoped.** It lands in the contended zone
   (`deeptime.rs` plus `viewer/main.rs`) and is owner-gated because rung 5 moves the byte pins.

Before the PR body is used by anyone downstream, the three stale claims in section 1e should be corrected in
place, since a merged PR body becomes the permanent record of what landed.

---

## 5. The honest gaps

Recorded with evidence, not fixed. Items 5a to 5c are #212's own; 5d to 5f belong to the receiving surface
and are recorded because they are what an integration would land on.

### 5a. The 16 derived values have no drift guard, and break the repo's own cited-column convention

The 16 `(T_eff, log10 departure)` points are Rust literals at `astro.rs:2687` through `astro.rs:2702`,
constructed as `Fixed::from_ratio(-26892, 10000)` and so on.

The repository has an established and strictly better pattern for exactly this shape of artifact. The closest
analogue is the Hayashi wall grid, which is a cited stellar-model grid indexed by a stellar parameter and is
consumed by the very same viewer function (`main.rs:1705`). It lives at
`crates/physics/data/hayashi_wall.toml`, and its header declares (verbatim, lines 4 to 8):

> BLOCK KIND: [[wall]], the cited-data-COLUMN idiom (matching the optical-constants [[species]] and the
> ionic-radii [[radius]] columns), NOT a reserved floor kind: an immutable transcription of the cited values
> from the vendored BHAC15 source, out of the floor real/fantasy authorship axis. WITNESS: the held source
> bytes at bhac15/BHAC15_tracks+structure (sha256 in bhac15/manifest.toml); scripts/bhac15_provenance_test.py
> re-derives THIS grid from those bytes offline, so a drifted transcription fails the build.

That last clause is the point. The repository carries NINE such re-derivation scripts
(`aesopus_`, `bhac15_`, `condensation_mu_`, `convection_scaling_`, `disk_arc_literature_`, `gruneisen_`,
`janaf_`, `rayleigh_eigenvalue_`, `thermoelastic_anchors_provenance_test.py`). There is no
`bstar2006_provenance_test.py`, and `grep -rl "bstar\|BSTAR" scripts/` returns nothing. **If a digit in
`astro.rs`'s 16-point table drifts, nothing in the repository catches it.** The sibling grid in the same call
path fails the build on exactly that.

This is compounded, fairly rather than accusingly, by the custody choice. `svo_tlusty_bstar2006` is
`custody = "witness"` with `redistributable = false`, so no bytes are held in-tree and an in-tree
re-derivation is not currently possible. The licence reasoning behind that is conservative and defensible.
The consequence is still that the provenance chain for 16 load-bearing numbers terminates in a set of sha256
receipts plus external Wayback URLs plus a coordinator attestation, with no held bytes and no build-time
check. The registry itself is explicit that the archive layer is "coordinator-attested" rather than
self-verified in session.

The practical fix is to move the table to a `[[departure]]` cited-data column with its receipts, and to add
the provenance test against the restricted store when the custody runbook lands. This also unblocks the
named next rung: the full `(Z, log g)` lookup is 1248 points, which is not viable as Rust literals.

### 5b. `TruncationSink` is a closed authored enum in the path of world content

`giants.rs:584` declares exactly three destinations for stripped gas. Its own doc concedes the substance:
"How the removed gas DIVIDES among these channels (if it divides) is a not-yet-derived physics question,
named here rather than fabricated." Naming the gap is the right instinct and this is not a fabricated value.
The residual concern under Prime Directive 4 and Principle 8 is that a closed three-member enum is sitting
where a derived outcome belongs, and it currently has no consumer to constrain it, so nothing pushes back if
a fourth channel is physically real. It should be surfaced at the re-gate as a latent closed list rather than
treated as settled. It costs nothing today because nothing reads it.

### 5c. The remaining audit items, and how each was closed

Verified individually against the head commit. The distinction that matters is CLOSED BY FIX versus CLOSED BY
REMOVING THE CAPABILITY.

| Item | Status | How |
|---|---|---|
| Blocker 1 (verdict ignores truncated gas) | Closed by removal | `DiskEvolutionState` reverted |
| Blocker 2 (48.118x collapse band discarded) | Closed by removal | Same reversion |
| Blocker 3 (conservation sink is theatre) | Closed by fix | `dispose_truncation_residual` now validates non-negativity AND `retained + removed == total`; the sink is retracted to a LABEL and the foundation's tag-discarding is named at `giants.rs:596` |
| Blocker 4 (departure without coordinates) | Closed by fix, with a caveat | The `(Z, log g)` gate at `astro.rs:2755`; see section 3b for why the gate makes the feature unreachable |
| Blocker 5 (dead EUV branch, backwards at tau=0) | Closed by removal | `EuvDispersalPhase` reverted; the manifest `used_by` retracted to "NOT wired" |
| Blocker 6 (overflow plus silent zero) | Closed by fix | Ring reorder at three sites; `unwrap_or(Fixed::ZERO)` replaced with `?` |
| Item 7 (contradictory birth conditions) | Closed by removal | Same reversion |
| Item 8 (authored `R1`) | Closed by removal | Same reversion |
| Item 9 (provenance DAG does not exist) | Closed by removal | Same reversion |
| Item 10 (quadrature defects) | Closed by fix, partly by disclosure | Sub-ring split, `i32` and `dr` guards; the half-ulp-per-ring quantization is now DOCUMENTED as an inherent floor at `giants.rs:495` rather than removed |

Five of the ten were closed by withdrawing the capability. That is the right call and the author says so
plainly, but it means #212's net contribution is materially smaller than the PR body advertises: the
canonical disk state and the diffuse/direct EUV branch, which were the arc's two headline items, are gone.

One item I checked and CLEARED rather than reporting: the `truncation_gas_ledger` doc at `giants.rs:495`
lists "a disk-edge miss" among its `None` reasons while the guard block contains no such check, which reads
as a stale claim. It is not. `gas_surface_density_kg_m2` (`giants.rs:199`) documents "`None` past the disk
edge" and the ledger propagates it with `?`, so the refusal is real and inherited. Recorded so the next
reader does not re-flag it.

### 5d. The receiving surface: "RESERVED" constants with no manifest entry

This is PRE-EXISTING on `origin/main` and is NOT introduced by #212. It is recorded here because it is the
surface any integration lands on, and because it is the "authored value with a confident comment" pattern in
its purest form.

The viewer's disk clock runs on roughly twenty hardcoded Rust constants in a block at `main.rs:1000` to
`main.rs:1091`, each of whose doc comments opens with the word "RESERVED". Verified against `origin/main`:
`calibration/reserved.toml` holds 229 `[[reserved]]` entries; ZERO are sourced from `viewer/src/main.rs`, and
grepping the manifest for `FORMATION_ACCRETION_RATE`, `DISK_CHARACTERISTIC_RADIUS`, `DISK_ALPHA_VISCOSITY`,
`MIDPLANE_BRACKET`, `DUST_SURFACE_DENSITY`, and `BIRTH_ACCRETION_RATE` returns zero for all six.

`CLAUDE.md` section 7 states the contract these violate: "every reserved value is a named constant in a
calibration manifest, defaulting to a sentinel that fails loudly if used unset, never a silent plausible
default." These are silent plausible defaults wearing the word RESERVED.

The sharpest single instance, `main.rs:1006` to `main.rs:1012`:

> RESERVED, the FORMATION-era mass-accretion rate `Mdot` ... Basis: **pinned so the DERIVED 1 AU formation
> midplane reconstructs** the forsterite/enstatite silicate condensation front (~1400 K ...). NOTE the
> degeneracy the owner re-partitions: only the PRODUCT (this rate times the dust column times the opacity) is
> fixed by the 1 AU landmark, so a lower dust column trades for a higher rate.

That is a back-solved value, by its own admission, with an admitted unresolved degeneracy. The same 1400 K
landmark is then reused as `CONDENSATION_OPACITY_REFERENCE_K` at `main.rs:1035`. This is the circular
validation shape: a value fitted to reproduce a target, and the target reused downstream.

The wire's defence is that `formation_rate_consistency` grades the result against independent-basis interims,
so the check is non-circular. The independence is asserted by `InterimBasis::CitedToPopulation` tags written
as literals at the call site (`main.rs:1712` and `main.rs:1716` region). The tag is authored, so the
non-circularity claim rests on an authored tag rather than on a data path that could not have been fitted.
Worth surfacing to the owner; it does not block #212.

### 5e. The receiving surface: duplicated inline stellar scalars

The mass-luminosity exponent 3.5, the reprocessing factor 0.25, and the inner-boundary factor 1.0 are
re-hardcoded at three or four independent sites in `crates/viewer/src/main.rs` (inside
`derive_pre_ms_bolometric_luminosity`, again inside `derive_formation_condensation_temperature` at the
`Fixed::from_ratio(35, 10)` / `Fixed::from_ratio(1, 4)` / `Fixed::ONE` argument block, and again in the scene
builders). Independent copies drift. In the first site they are bare locals whose only comment justifies the
value by pointing at another hardcoded use of it ("the ... exponent the displayed midplane uses ..., reused
here"), which is a circular justification rather than a provenance. Again pre-existing, again not #212's.

### 5f. What I could not close

- **I did not run the byte pins.** The bit-exactness of `40fe8a72` and `be94e310` is the author's claim plus
  green CI. I verified the STRUCTURAL reason it must hold (no production consumer reads any changed kernel),
  which is strong evidence, and it is falsifiable by running the two pins. That run is cheap and should be
  part of the re-gate.
- **I did not re-derive the 16 SED integrations.** The prior `gpt-5.6-sol` audit reproduced all sixteen
  literals to better than 5e-5 dex from independently integrated SEDs, and I took that as established rather
  than repeating it. The residual risk is not the physics but the transcription, which is section 5a.
- **I did not verify the Wayback captures for fid 590 and 650.** The registry records the SAVE-retry as done
  and byte-identical after gzip decoding. That is a coordinator attestation; no in-tree bytes exist to check
  it against.
- **The `(Z, log g)` interpolation error band is unquantified.** The grid's own doc concedes the
  `T_eff` interpolation is "an authored piecewise-linear model whose error is the deviation of the true curve
  from each chord" with the band "a later rung". Since the local slope ranges 0.054 to 0.188 dex per 1000 K,
  the chord error is not uniform across the grid, and no bound is stated.
- **The threshold and denominator mismatches are named but not corrected.** The derivation used 911.28 A
  (13.606 eV) while the code's edge is 13.6 eV (911.65 A), and the derivation's blackbody denominator is full
  Planck while runtime uses a one-term Wien tail. The working note quantifies the combined effect at roughly
  0.25 to 0.29 percent and names it as a real systematic. It is small against a two-decade departure and it
  is disclosed, so it is a recorded limit rather than a defect. `IonizingSpectrumEvaluation` still does not
  carry its own threshold, so the fixed table can be applied to a blackbody built with any caller-selected
  `T_ion`; that one is a typed-contract gap worth closing before a consumer exists.
