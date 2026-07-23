# PR #215 live-source audit

Status: implementation audit for the canonical integration on PR #215 branch
`claude/thermoelastic-rung3`, updated 2026-07-22.

This record covers the integration worktree that normalizes the physical floor,
separates SI representation from causal information, gives the open
stellar-birth census a modular value-free structure and stellar-state
contract, and preserves the Stage 1 refusal while the stellar-birth measure
remains open.

## Governing invariant

The sealed absolute physics floor is the sole source of physical causal
values. The exact SI numbers in the representation receipt define an immutable
coordinate transform; they are not caller inputs, ledger facts, or physical
degrees of freedom. Provenance, tier, source custody, and accounting never
admit a magnitude. Every later physical quantity must derive from admitted earlier bits, or its non-derived leaf must
complete every required derive-first and floor-admission receipt after
exhaustion. Otherwise it remains a named refusal. The runpath must admit alien
physics and must not select the Sun, Earth, a familiar chemistry, a viewer
state, or an authored realization.

## Floor state after remediation

The generated physical-floor inventory contains three Universal entries. The
units layer separately publishes a ten-identity representation view and a
fourteen-identity execution view:

- three `[M]` physical invariants: `alpha`, `G`, and `m_e`;
- seven exact SI representation definitions with no provenance mark;
- one runtime `[D]` event for `eps_0`, derived from `e`, `alpha`, `h`, and `c`;
- three representation-derived values: `sigma`, `R`, and
  `A3_per_cm3_mol`, completing the ten-identity representation view;
- one fourteen-identity execution capability containing that representation
  view, the three admitted physical coordinates, and derived `eps_0`;
- zero `[E]`, `[C]`, `[A]`, `[W]`, and `[X]` entries;
- zero Reference, Residue, and Contingency entries.

The seven marks are `[D]`, `[M]`, `[E]`, `[C]`, `[A]`, `[W]`, and `[X]`.
The four tiers are Universal, Reference, Residue, and Contingency. The census
is generated from the sealed catalog at
`docs/working/CANONICAL_LEDGER_INVENTORY.txt`; it is not maintained by hand.

## Numerical execution boundary

The floor receipt is not globally Q32.32. Each admitted or derived constant is
published as a signed integer plus its own binary scale, chosen from the exact
decimal rational and its projection contract. Values such as `h` and `m_e`
therefore retain more than 32 fractional bits. Existing physical kernels use
`civsim_core::Fixed`, an integer Q32.32 type. A capability method performs the
one explicit rounding into that kernel representation and refuses values that
cannot be represented.

Floating-point comparisons are diagnostic or test oracles only. They do not
author the floor, bind a transcript value, or feed the current canonical
runner. The active GPU crate already implements integer Q32.32 arithmetic,
transcendentals, and field kernels against the CPU `Fixed` bit oracle, but it
is not reachable from the canonical planet package yet. A later typed adapter
must prove exact CPU and target-backend GPU parity before GPU output can enter
the causal bitstream.

## Findings closed in this slice

1. **Candidate declarations could authorize themselves.** The units table was
   both the declaration and the admission authority. The independent ordered
   registries in `civsim_units::physics_floor` now seal physical invariant
   admission, exact SI representation definitions, and every execution
   relation. Planet code can request the sealed floor but cannot define its
   authority. A candidate must match its complete fingerprint, and an extra
   candidate is rejected.
2. **Derived values did not replay from the ancestry bits in the transcript.**
   The gas constant and Stefan-Boltzmann projections were influenced by
   parallel reference decimals. Composite evaluation now consumes only the
   exact rational values represented by the published projected inputs. A
   recursive-descent producer and an independently written shunting-yard
   watchdog enclose the whole formula with exact rational intervals. They emit
   only when both Machin-series paths select one magnitude bracket and the same
   round-half-even integer. A finer approximation agreeing by coincidence is
   no longer treated as a proof of correct rounding.
3. **A measured source decimal was being represented too coarsely.** Floor
   projection now preserves at least the source decimal's last stated place.
   Each `[M]` transcript record carries the source identifier, SHA-256,
   locator, source decimal, exact or standard uncertainty, projection rule,
   and maximum half-ULP projection error. The bit projection is explicit; the
   measured physical quantity is not mislabeled as an exact binary fact.
4. **Identity depended on parallel array order.** Every projected magnitude is
   sealed to its symbol. The canonical floor view names each expected symbol
   at the binding site and rejects relabeling.
5. **Dimensions and ancestry were unchecked text.** Every floor value now
   carries a typed seven-exponent SI dimension. The composite formula parser
   evaluates dimensions with the same operation grammar used for values,
   records every exact symbol read, and rejects either a dimensional mismatch
   or an ancestry mismatch.
6. **A hidden reference-decimal composite path remained callable in normal
   builds.** The old comparison helpers are test-only. Production composite
   construction has one projected-input path.
7. **Richer evidence made a transcript enum needlessly large.** The future
   written-value payload now uses owned indirection. This changes no serialized
   field or causal rule.
8. **Exact SI definitions were counted as measured physical inputs.**
   `Delta_nu_Cs`, `c`, `h`, `e`, `k_B`, `N_A`, and `K_cd` now live in a
   versioned, untagged representation receipt with source custody. They encode
   all seven SI base coordinates but contribute no causal degrees of freedom.
9. **Vacuum permittivity duplicated the electromagnetic coupling coordinate.**
   The floor now admits measured dimensionless `alpha`; `eps_0` is recomputed as
   `e^2 / (2 * alpha * h * c)` and its cited decimal is only a drift oracle.
10. **Universal leaves bypassed derivation exhaustion.** Admission now requires
    a complete receipt for every non-derived leaf at every tier. The three
    current invariant receipts state derivation attempts, Buckingham-Pi budget,
    Gap Law with typed Chaos Protocol, Residual Law, and unique residual slot.
11. **Receipt prose could vary after structural admission.** Canonical preflight
    now exact-matches the sealed receipt fingerprints as well as the ledger, so
    a caller cannot replace the evidence narrative while keeping the same IDs.
12. **Physical helper APIs could bypass the verified floor.** Saha,
    polarizability, opacity, electronic transport, Harrison scaling, d-state
    radius, QEq, lattice modulus, and their active material consumers now
    require the sealed execution capability. Their physical folds derive from
    its published bits. Representation-only conversions use the separate
    noncausal representation view.
13. **Receipt construction and receipt verification shared authority.** The
    units authority now pins an independent length-prefixed SHA-256 digest for
    every ordered physical receipt. Changing constructor prose, budgets,
    attempts, evidence, or residual slots no longer changes the expected
    authority.
14. **Some declaration reorderings were accepted.** Representation,
    execution-relation, and physical-admission registries are all checked by
    exact length and position. A set-equivalent reorder now refuses.
15. **A verified capability could be mutated after construction.** Both SI
    view types now keep every projected value private and expose only read-only
    symbol lookup and derived operations. The private seals and private fields
    jointly prevent caller construction and post-verification reassignment.
16. **Dormant production APIs still crossed from floats into fixed state.**
    Covalent-radius and Badger tables now hold decimal text and parse through
    exact rationals. Perovskite temperature keys now use integer Q32.32
    arithmetic with explicit round-half-to-even behavior. The unused public
    `f32` and `f64` core quantizers were removed. The canonical planet gate now
    rejects floating-point types, and the GPU no-float test scans Stage 0,
    shared primitives, fields, and transcendentals.
17. **Gap Law did not carry its Chaos Protocol as typed evidence.** Admission now
    requires either a not-applicable basis or a nonempty dynamical regime
    partition with a transition law. Each regime proves that input bands remain
    resolved for direct evolution or carries the stationary measure,
    conservation projection, stability, validity, coordinate discipline, and
    exact replay required by a sub-resolution disposition. Empty evidence or an
    empty regime list refuses. Receipt fingerprints and transcripts cover the
    complete ordered partition.
18. **The Stage 1 contract could name leaves but could not evaluate them.** A
    fixed-order evaluator now consumes only two opaque repository-owned proof
    capabilities. It reports the exact unresolved leaf frontier, closes the
    root only when both capabilities exist, and exposes no string, citation,
    scalar, tag, or caller boolean as a closure path.
19. **Dimensional derivation debt existed only in prose.** A checked exact
    rational engine now computes rank, pivots, free columns, target-span
    projections, and primitive integer null-space bases over the seven SI
    coordinates. The stellar-birth census applies it per phenomenon so rank
    from thermal variables cannot conceal a rotational or collapse gap. Every
    returned null vector is rechecked against its typed dimension matrix.
20. **A downstream analysis could name the floor without binding its complete
    authority.** One independently pinned, length-prefixed v1 SHA-256 now
    covers the representation schema and base order, ordered representation
    fingerprints, ordered physical admissions, receipt-fingerprint schema and
    pinned receipt digests, and ordered execution relations. The census records
    that binding but receives no magnitude through it.
21. **Receipt and transcript duplicated the open-frontier serializer.** One
    prefix-aware formatter now writes both views. A direct test strips their
    distinct prefixes and proves the complete frontier payloads match.
22. **Exact dimensional edge cases could panic or refuse representable
    rationals.** Formula powers now use checked multiplication before narrowing
    to an `i8` exponent. Rational sign normalization reduces unsigned
    magnitudes before conversion, handles zero over `i128::MIN`, and
    cross-cancels division before constructing a reciprocal. The original
    falsifiers now return the exact result or the typed overflow error.
23. **Dimension reachability could be mistaken for a law-shaped witness.** An
    attempt now reports `target_dimension_reachable`, never dimensional
    soundness. Its nonzero dimensional support is serialized explicitly, so a
    witness such as mean particle mass from `m_e` alone exposes that composition
    remains physically unused and its species registry remains missing.
24. **Refusal detail was typed internally but text-only to library callers.**
    Immutable public views now expose both registry contracts, index domains,
    carrier schemas, variables, local matrices, null spaces, attempts, nonzero
    support, missing dependencies, dropped mechanisms, and coverage gaps.
    Constructors and proof seals remain private. The views live in a separate
    requirement-analysis module instead of expanding the receipt formatter.
25. **Several future closure semantics were underspecified.** Spectral flux now
    names its per-log-frequency density convention and gauge-reference rule.
    Composition is an explicit number-fraction simplex over complete species
    support. Material mass history names the missing initial state, flux, and
    topology law. Attempts are identity-sorted, duplicate-checked, and
    constrained to their enclosing phenomenon, and the coordinate-law leaf now
    owes Residual Law and a unique slot if irreducible.
26. **Carrier, index, and component topology semantics were implicit.** The v1
    structure module now separates component and species registry contracts,
    six physical support domains, and eleven carrier shapes. Exact schema
    catalog completeness and canonical order fail closed. Its rule identities
    require a future component count to come from the approved realization
    coordinate within joint-measure support, and require physical content to
    control identity and topology labels under permutation-equivariant multiset
    semantics. They likewise require convergence-derived resolution, named
    capacity refusal, and serialization-only ordinals. No realized registry,
    content encoding, collision handling, permutation operation, topology
    validation, convergence controller, capacity comparison, or typed capacity
    refusal exists yet. Lagrangian material identity replaces the former
    single-center shell axis in the census, while local frames, multicenter
    binding, and symmetry reductions remain open physical obligations.
27. **Execution verification did not require the aggregate floor-authority
    pin.** The value-opaque singleton checked independent value and receipt
    registries, but a coordinated edit to both static tables could project
    without proving the final binding. `verify_absolute_physics_floor` now
    requires `civsim.units.physical-floor-authority-binding.v3` before comparing
    or exposing execution magnitudes. The v3 seal binds tier and provenance for
    every physical admission, the independent
    `civsim.units.floor-catalog-admission-pair.v1` receipt, and the paired exact
    fixed-math table receipt. The capability remains private and does not accept
    caller values.
28. **Build-gate authority failures could be overrideable or skipped.** Stone 0
    now treats a missing runner, unavailable interpreter, runner crash,
    unrecognized exit, caller-selected override trust path, and every unmarked
    nested leaf failure as non-overridable operational failure. A nested policy
    result requires exact exit 1 plus
    `civsim.gate-leaf.policy-detection.v1`; only then may the runner emit
    `civsim.gate-runner.policy-detection.v1`. Existing unmarked leaves remain
    intentionally non-overridable.
29. **The shared structure writer trusted its caller.** Although normal census
    construction validated the schema, a future internal caller could serialize
    a reordered or mutated structure beneath the v1 label. The writer now
    validates the complete sealed structure before its first write. A negative
    test proves a reordered domain catalog returns a formatting error and leaves
    the output buffer empty.
30. **The Git for Windows history hook could not open Bash process
    substitution.** The declarative pre-push gates passed, but the credential
    range scan handed `git grep` a `/proc/.../fd/...` pattern path that the child
    Git process could not reopen, then followed its documented fail-open
    operational policy. The hook now streams patterns through `git grep -f -`.
    Pattern bytes remain in memory and never enter a file or command argument.
    A temporary-repository test proves a clean commit scans without the notice
    and a later commit carrying a synthetic live token blocks without printing
    that token.
31. **Named stellar classes could have become causal selectors.** Five focused
    structure modules now separate open state-coordinate membership,
    interaction-sector admission, physical predicate and mechanism proofs,
    complete stellar-state history, and noncausal classification projection.
    Every law, coupling, basis extension, sector, predicate formula, threshold,
    and use must derive or complete immutable pre-seal admission. One acyclic
    pre-dispatch physical DAG authorizes only a unique, globally conservative,
    non-double-counting transition. Coordinate identity binds the complete
    physical descriptor over an open variable-cardinality basis; sector identity
    binds the complete admitted artifact; unresolved identity collisions refuse.
    State history is lineage-preserving across physical birth, death, merge, and
    split. Presentation uses typed taxonomy identities over a total read-only
    projection and has no causal authority. The schema enumerates no named class
    or hypothetical sector. Structure v2 and census v4 expose the rules in the
    exact refusal wire, while a seventh coverage gap and complete common
    admission obligations on both leaves keep realized behavior open.
32. **The retained mean-particle-mass path authored familiar chemistry.** Its
    element table, Solar abundance convention, logarithmic conversion,
    caller-selected molecular state, and familiar collapse endpoints cannot
    enter the canonical runpath. One private exact kernel now keeps only the
    complete-support weighted reduction. It distinguishes exact SI mass
    coordinates from dimensionless number fractions, orders content identities,
    refuses duplicates, unequal-content collisions, support mismatch,
    nonpositive weights, absent state proof, missing or negative mass, and a
    nonunit simplex, and admits an unfamiliar or massless state through the same
    structural path. Its authority and proof seals have no production
    constructors or physical bindings, its resolver returns `None`, and no
    production caller exists. The repair is therefore byte-neutral and does not
    claim a derived species registry or close Stage 1.
33. **The floor's declared Pi budgets were sealed but not independently
    proved.** The existing exact-rational RREF producer now agrees with a
    separately implemented fraction-free integer checker. Their receipt binds
    the ordered matrices, phenomenon membership, residual slots, declared
    budgets, ranks, nullity, bases, algorithm identities, and mutation
    canaries. The physical-floor v3 authority binding includes that agreement.
34. **Mechanical checkers could still define the claims they checked.** A
    closed authority inventory now distinguishes active paired authority,
    blocked authority, and non-authoritative diagnostics. A schema-first
    producer and profile-first watchdog independently pin every complete row
    and its semantic closure. Stone 0 pins the exact gate blocks and also runs
    the authority inventory, build-wiring, fixed-math, and external-claim gates
    directly, so the declarative runner cannot suppress its own cross-checkers.
35. **Formula precision knobs and final-value spot checks did not certify the
    selected integer.** Recursive-descent and shunting-yard implementations now
    evaluate one resource-bounded exact interval claim and emit only when both
    select the same round-half-even terminal integer. Opaque invariant factors
    carry their own receipt and two independent outer bindings to the terminal
    projection. Byte, token, nesting, decimal, exponent, coordinate, and
    intermediate-rational limits refuse oversized formulas before expansion.
36. **Canonical path scans could miss source included through `#[path]`.** The
    planet boundary gate now resolves explicit path bridges, scans their source,
    and exercises nested, parked, and hostile path canaries. It also rejects
    raw arithmetic projection APIs in canonical planet source because exact
    formula coordinates prove arithmetic, not physical ancestry.
37. **Exact Q32.32 transcendental tables could drift behind deterministic
    tests.** Two independent scripts derive and bind Pi, half-Pi, log-two,
    inverse-log-two, every CORDIC angle, inverse gain, order, and occurrence in
    the two canonical CPU and GPU implementation files. That narrow table claim
    is active and bound into the floor; whole-domain error, rails, iteration
    semantics, backend parity, and vendor execution remain separately blocked.
38. **The SI table aggregate was named like an authority receipt.** It is now an
    `aggregate_digest_sha256` drift diagnostic. SI execution-table completeness,
    ancestry, and scale policy remain blocked until their own independent
    inventories and aggregate receipt checkers exist.
39. **An adverse external claim or author contact had no fail-closed release
    boundary.** The new governance pair requires exact text and destination,
    five independently connected evidence lineages beyond the subject, a
    private dossier digest, an unrevoked human signature, scope and expiry, and
    independent implementation agreement. No release row exists, so the
    repository currently authorizes no adverse publication or contact.
40. **Certified formula proofs were repeated inside iterative retained stellar
    and disk tests.** The invariant Pi-bearing coefficients are now certified
    once and consumed through factored terminal receipts; the fully invariant
    Kepler reference period is cached after its certified derivation. Direct
    whole-formula confirmation preserves every prior Q32.32 bit. The 28 giant
    tests fell from a stopped run with one tail past 25 minutes to 272 seconds;
    two older integration tests still exceed 60 seconds and remain profiling
    debt rather than authority shortcuts.
41. **Concurrent gate runners could stampede one uncached verdict.** Two Cargo
    build processes could compute the same pre-run snapshot, miss the same
    receipt, and launch duplicate canonical work. One duplicated planet-boundary
    worker exhausted its supervisor budget during a fast check. Every live gate
    now takes one portable, bounded, fail-closed process lock. Content-hash
    followers recompute the snapshot and may consume the leader's receipt only
    under that lock; uncached gates serialize and still rerun. Independent
    two-process canaries prove both behaviors, and hostile link state refuses.
42. **Two guarded Cargo packages each owned a repository-wide Stone 0 run.**
    `planet` and `planet-substrate` could therefore duplicate the complete
    provenance suite in one Cargo graph. The new build-only
    `civsim-stone0-build` anchor owns the run and writes a compile-time marker
    only after success; both packages depend on that marker rather than calling
    the gate. A manifest parser and a separately implemented raw scanner agree
    on the exact build-only topology, marker ordering, consumer sentinels, and
    absence from runtime and aggregate package surfaces. This changes
    verification ownership only and cannot enter simulation state.
43. **The fixed-math receipt bound one Windows checkout representation.** The
    checked receipt hashed CRLF worktree bytes, while Git and CI materialized
    the declared canonical LF text. Both mathematical implementations agreed,
    but the stored receipt correctly refused the different digest and Stone 0
    blocked canonical and parked CI. Receipt v2 now binds canonical Git LF
    bytes. The producer and watchdog use separately implemented CRLF mappings,
    prove LF and CRLF checkout equivalence, and reject bare carriage returns.
    The changed receipt is included in the reviewed physical-floor authority
    digest; no mathematical table bit or physical magnitude changed.
44. **The authority-inventory receipt repeated the checkout-byte weakness.**
    It did not break CI because no checked receipt pinned the meta-pair output,
    but its semantic-closure hashes could differ between stale CRLF and fresh
    LF checkouts. Receipt v3 now binds canonical Git LF text for the registry
    and every enrolled closure member. The two validators use distinct byte
    mappings, prove native Windows and Linux receipt equality, and reject bare
    carriage returns. Binary research holdings remain under their exact-byte
    custody contracts rather than this repository-text rule.

The resulting projected composite values are replayable from the transcript:

- `sigma`: bits `2042967686`, scale `55`;
- `R`: bits `35710345014`, scale `32`;
- `A3_per_cm3_mol`: bits `7131960987`, scale `32`.

## Source custody

The held floor remains the 2018 CODATA set. This slice did not silently upgrade
values to the later CODATA adjustment.

- NIST 2018 ASCII table, 40,689 bytes, SHA-256
  `8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1`;
- NIST 2018 values PDF, 189,260 bytes, SHA-256
  `0baec5980ef4956f3047fe6b6113a27013483999ceb8078421f4b3acdaa6159a`;
- CODATA 2018 adjustment paper, 2,312,070 bytes, SHA-256
  `6d712bdc99719540bec65c7d1ef11b00f5d321e6083e9e6ed7d3de6fb8062908`.

Live NIST bytes matched their archived witnesses. The source registry carries
the identifiers, hashes, archives, scope, and extract anchors. No source bytes
were added because redistribution rights were not established. Custody proves
what was consulted; it does not admit a physical value.

## Canonical runner receipt

Two direct no-argument runs produced:

- exit code `2` on both runs;
- zero stderr bytes;
- six transcript events;
- no `[W]`, no `[X]`, and no snapshot;
- byte-identical stdout of `367,628` bytes, SHA-256
  `579db1856ba0134a5f747a0ace6d116376161f1998706ba9c2ce6c89b15dae45`;
- receipt `civsim.planet.run.v11` and transcript schema major `9`;
- refusal `stellar_birth.realization_measure` at Stage 1.

The umbrella refusal has two unresolved leaves:
`stellar_birth.joint_physical_measure` and
`stellar_birth.realization_coordinate_law`. The executable evaluator serializes
those leaves in fixed canonical order. The repository resolver supplies neither
opaque proof capability yet, so the root refusal is the correct output.
Only the open joint-measure leaf carries analyses. The first is the
`exact_dimensional_census`. Census v4 contains 31 typed, value-free
coordinates, seven phenomenon-local matrices, two realized-membership
contracts, six shared domains, eleven carrier schemas, and the composed open
stellar-state contract with coordinate, dimension-basis, interaction-sector,
physical-regime, and classification registries. Its status is `computed`, its
`closure_effect` is `none`, and its `coverage_claim` is `false`. The second is
the v1 species derivation analysis. It binds the floor digest and exact mass
anchor to the validated open schemas while reporting zero candidate members,
zero verified support, no value payload, no residual slot, and eight ordered
open proofs. Its status is `open_dependencies`, its `closure_effect` is `none`,
and its `coverage_claim` is `false`. Gap Law, Chaos Protocol, and Residual Law
remain not reached. The open coordinate-law leaf carries analysis count zero.

## Verified reachability

The active viewer accepts only a sealed borrowed observation of an immutable
snapshot or exact refusal receipt. `PlanetRunOutcome` is a public query wrapper
over private state, so an external caller cannot forge an outcome and use it to
mint an observation. The viewer cannot receive the causal outcome, call the
runner, construct the observation, promote a refusal, mutate a receipt, or
advance a world. The canonical package depends at runtime only on ledger and
units. Retained star, disk, planet, crust, geodynamics, deep-time, moon, and
flexure code is private in `civsim-planet-substrate`; it is not reachable from
the canonical stages until a typed adapter is admitted. Biology,
civilization, authored world generation, and the old causal viewer remain in
`parked/` and are not canonical readiness evidence.

## Validation

Focused warnings-denied Clippy, 71 planet library tests, eight CLI tests, three
viewer tests, and doctests cover the typed views, duplicated refusal wire,
deterministic output, canonical versions, input refusal, observer projection,
and the non-admitting species frontier. Formatting, diff hygiene, and the
strengthened planet boundary self-test are clean. After the independent
authority, fixed-math, external-claim, process-lock, and Stone 0 wiring repairs,
the complete Linux `check-pr` parity recipe passes in 1,178.5 seconds under
concurrent external routing load. It includes hooks and canaries, all 29
declarative gates, ledger regeneration, the all-target suite, deterministic GPU
integer and no-float checks, available-backend parity, warnings-denied Clippy,
Rustdoc, and doctests. A warm `check-fast` passes in 78.3 seconds. A fresh
isolated Cargo target proves exactly one shared Stone 0 anchor invocation and
one lightweight sentinel for each guarded consumer.

Clean and hostile-environment direct binary runs confirm exit `2`, zero stderr
bytes, `367,628` stdout bytes, byte equality, and SHA-256
`579db1856ba0134a5f747a0ace6d116376161f1998706ba9c2ce6c89b15dae45`.
No earlier receipt or inventory pin is evidence for the v11 result.

One bounded blind-generalizer pass over the species-analysis and observation
slice found one live observer-provenance defect. The public outcome enum could
be forged by an external caller and projected into a nominally sealed
observation without running the canonical pipeline. Replacing it with a public
query wrapper over private state closes that path, and source canaries now
reject a public outcome enum, named or tuple field, unit form, or constructor.
The pass found no other live false closure, Gap Law, Chaos Protocol, or Residual
Law shortcut, Terran selector, nondeterministic wire order, or schema-version
mismatch. The dormant species proofs remain explicit future activation debt.

One earlier floor blind-generalizer pass was run against frozen candidate and specification
packets, as requested instead of the historical six-pass overnight loop:

- candidate SHA-256
  `d91f029fd8c54d1a7a5f9062d1ac21b4505683f27e5fab9d8ae12f49c2542c46`;
- specification SHA-256
  `07f03d9a4bd200fce7fca547635148af4af12d99cbf0db02a4995105827a5eb2`;
- three live findings: unaudited physical API reachability, shared receipt
  construction and verification authority, and order-insensitive declaration
  checks;
- all three findings repaired in this worktree without selecting a desired
  physical outcome.

The executable-leaf and Chaos Protocol slice received one earlier
packet-only generalizer pass, not a six-pass loop. Its strongest-model
input-bias smoke first returned `BLOCKED`: the packet treated the initial
three-way chaos classification as exhaustive and tested false admission more
than false refusal. The packet was neutralized with symmetric valid, absent,
and invalid-proof cases plus mixed, stochastic, multi-attractor,
nonstationary, and regime-changing falsifiers; the smoke then returned
`CLEAR` before the auditor ran.

The blind pass found that one exclusive not-applicable, dissipative, or
Lyapunov-sensitive tag could not express mixed or changing dynamics. Source
verification confirmed the enum had that shape. It is repaired as a nonempty
validity-regime partition with a transition law. Each regime now records one
of two resolution dispositions: direct evolution proves admitted input bands
remain resolved, while sub-resolution evolution requires the stationary
measure, conservation projection, stability, coordinate discipline, and
replay gates. Multiple regimes can coexist; unsupported regimes refuse instead
of being misclassified.

The pass also correctly identified the future danger of treating runtime type
and presence as semantic proof. Source verification found no current live
bypass: production has no proof constructors and returns typed absence for
both leaves, while the artifact seals are private. The exact open obligations
now include absolute-floor binding, artifact-schema version,
semantic-checker version, dependency digest, and coordinate-to-joint-measure
binding. Those checkers remain required work before either production proof
constructor may exist. This packet summarized structure rather than carrying
the complete comment-stripped source, so it is evidence for the named
generalization findings only, not a passed full code panel.

The dimensional-census slice then received one sealed, comment-stripped blind
pass, after a strongest-model prompt smoke returned `CLEAR` without a retry.
The packet was 168,566 bytes at SHA-256
`ebb8ce6ecc024f56f66a7771d39f0e659b17d3741e9f6376c7279095b638088e`.
The pass found the dimensional-power overflow, signed-rational normalization,
vacuous target-span witnesses, ambiguous spectral measure, and missing mass
integration boundary. Independent derive-versus-admit, alien-feasibility,
Terran/Solar, steering/observer, and exact-correctness lenses checked distinct
failure modes rather than repeating the same generalizer. Source verification
confirmed and repaired the live findings listed above. The exact-correctness
lens independently reproduced all seven rank/nullity pairs and target
projections and recomputed the authority digest
`0a64c0513683f04461b11d3b6df9f18a4d283825300bae713b9a26d280c13367`.

The modular structure slice received a separate frozen-packet panel at base
`fdcd966a0dd31da125d861a48e44d23408d7d8b4`. Its 558,594-byte packet had
SHA-256 `6eb4b06de41590f53eb91edcb3aa2d7f0c3ca1c4bf4b9b2cb346f21b0a0d58b4`.
The strongest-model input-bias smoke returned `CLEAR` before six distinct
lenses ran. The derive-versus-author, alien-feasibility, and Terran/Solar
lenses found no current value or familiar-system selector; they explicitly
classified future registry realization and admission behavior as not
assessable from the refusing path. The observer lens found build-gate authority
failures outside the physics result, which were source-verified and repaired.
The blind generalizer's broad suspicion that the singleton floor capability
must carry caller-bindable values was refuted by the live private-capability
design, while its narrower coordinated-static-edit falsifier exposed the
missing aggregate authority-pin check and was repaired. The exact-wire lens
found that the structure writer trusted its caller; the writer now validates
the complete sealed schema before emitting its first byte, with a negative
serialization test. Every accepted finding was reproduced against live source
before repair. Future normalization, collision, topology, convergence,
capacity, and proof-constructor behavior received no clearance from this panel.

The open stellar-state slice then received its own sealed packet panel. The
initial 166,000-byte-class candidate was SHA-256
`c111618039b98063c0915c52ad5b8a562285d5c03ef77f0b9028120688e8de76`.
The strongest-model input-bias smoke returned `CLEAR` before six distinct
confirmation, derive-versus-author, alien-feasibility, Terran/Solar,
steering/observer, and exact-correctness lenses ran. Source verification
accepted and repaired the live findings: nested writer revalidation, canonical
wire-prefix validation, complete common admission obligations on both Stage 1
leaves, coordinate totality and measure-consistent push-forward, joint observer
and presentation independence, complete coordinate and sector identities,
variable-cardinality basis extension, applicability-qualified state history,
law-entailed boundary completeness, and explicit birth/death/merge/split
lineage. Packet suspicions that the current absent proof artifacts could close
Stage 1 were refuted against their private seals, absent constructors, and
production `None` resolver.

The repaired packet was 203,254 bytes at SHA-256
`012a2b1058dd881a59d3a05982175b4d1a0f89b593497c3a94c923d456193f9e`.
Its strongest-smoke record was SHA-256
`83dea0732dcc7e2ccc692fe76c1f158a79f3d7bb97a3ec8ed60b7527b6f3ceca`
and returned `CLEAR`. Repaired correctness, derive-versus-author, Terran/Solar,
and steering/observer lenses found no source-traceable live defect. They grant
no clearance to future registry members, digest implementations, proof smart
constructors, semantic checkers, measures, or transition executors.

The exact species-state reducer then received one frozen source-blind
generalizer process, not a repeated panel loop. The candidate source freeze is
commit `0b49c33aaefa237003c92a0c57deb0657379d8c9`, tree
`50414e921e5decf495a0e6740884b66164189f0b`, with module SHA-256
`f97ccfc794aee49d0ce4843aac5bfeb193f6ccfdcd17b8089cf37dd962d760e5`.
Its clean and hostile-environment runner differential is byte-identical to the
baseline refusal. The broader panel-construction smoke could not prove its
requested isolation, chronology, schema-completeness, or adjudication
properties, so no panel clearance is claimed. The source-blind contract was
stricter than this dormant slice and usefully exposed that zero-sized type
seals cannot stand in for physical authority. That is recorded as an activation
blocker rather than hidden behind the passing structural tests. Stage B found
no live defect attributable to the diff: the only production-reachable symbol
is the byte-identical law identity in non-admitting refusal metadata. It grants
no universal-property proof beyond the bounded exact tests and requires finite
support-count and rational-resource domains before activation.

## Remaining audit debt

- Admission structures still carry human-readable receipt strings. The current
  three receipts are independently sealed and exact-matched, but future leaves
  at any tier must replace prose-only proofs with typed, machine-resolvable
  artifacts before they can become live.
- `AbsolutePhysicsFloor` is a value-opaque singleton capability rather than a
  caller-populated coordinate container. Its identities and receipts select the
  one private coordinate registry only after execution verification requires
  the independently pinned v1 authority digest. This avoids a second mutable
  copy of floor values while keeping static declaration drift outside the
  admitted path.
- Opaque Stage 1 proof types have no production constructors. Future smart
  constructors must validate every obligation, bind the absolute floor,
  schema, checker, dependencies, validity domain, and joint measure, and return
  structured invalid reasons. Runtime type and presence cannot stand in for
  those checks.
- Stage 1 leaf proofs remain absent. The partial v4 census now encodes component
  and species registry contracts, shared support and resolution domains,
  topology shape, spectral measure convention, material/time histories, and
  open coordinate, basis, sector, regime, and presentation contracts. It does
  not supply registry contents, digest implementations, semantic checkers,
  multiplicity interactions, full spectral transport, remaining tensor
  carriers, fragmentation, magnetic braking, persistent disk state, or the
  required stochastic and chaotic regime measure.
- Current vector carriers do not distinguish polar from axial transformation
  character, and the census has no full tensor family. The explicit
  full-field-and-tensor gap and `coverage_claim=false` keep this from closing a
  proof. Future carrier compatibility must derive and validate transformation
  behavior rather than treating equal SI exponents as equivalent physics.
- Dimension-only projection proves that an output unit lies in an input span.
  It cannot prove a physical law or select a dimensionless coefficient. Each
  open attempt still needs mechanism ancestry, support, and uncertainty before
  it can contribute to a joint measure.
- Future contingency coordinates must not rely on a bounded integer domain
  without proving the physical support and mapping law.
- Exact rational helpers need explicit resource and exponent domains before
  formulas from untrusted or expanding catalogs can reach them.
- Variable-cardinality component fields still need realized canonical identity,
  collision, symmetry, topology-label, permutation, and capacity-refusal
  implementations plus a multiplicity phenomenon. Structural rule identifiers
  alone cannot validate a realized graph.
- Species and index-domain schemas still need realized membership, physical
  content identity, mass and dimension ancestry, state and sector proof,
  authority and replay bindings, derived chart and time-reference handling,
  convergence control, and typed capacity refusals. The private reducer now
  checks structural support equality, exact unit normalization, collisions, and
  weights, but its zero-sized seals are not evidence that an unfamiliar system
  is physically admitted.
- Material histories still need derived position and velocity state, local
  frame semantics, multicenter binding, translation and rotation invariance,
  flux topology, and conservation before a shell reduction or disk state can
  be constructed.
- Retained substrate adapters must replace caller embryo caps, old world seeds,
  and binary component identities with derived resolution, named capacity
  refusals, the verified coordinate law, and registry-driven topology.

## Audit-process limitation

Each slice used one frozen-packet generalizer or one panel of distinct
value-boundary, derive-first, admit-the-alien, Terran/Solar,
observer-reachability, and exact-correctness roles. This was not six repetitions
seeking the same result. Packet omissions still limit claims about code not
present in the packet; every accepted live-source finding was checked against
the current worktree before repair, and affected lenses were replayed against
the repaired stellar-state packet.

## Next derivation order

1. Extend the partial machine-readable census from its landed registry, domain,
   and carrier contracts to complete physical coverage while preserving one
   correlation-carrying joint measure.
2. Derive and bind the complete species mass and state registry to the floor,
   joint support, semantic checkers, and replay identity. Only then activate the
   landed exact weighted reducer.
3. Derive coupled gas and dust thermal balance with a proved residual bracket,
   then equation-of-state closure, collapse flow, material mass and position
   histories, local-frame angular-momentum transport, multicenter binding, and
   circularization from the shared state.
4. For each irreducible survivor, complete source custody, uncertainty,
   support, normalization, conditioning, Gap Law including the typed Chaos
   Protocol branch, Residual Law, and unique residual-slot receipts. If any
   item is incomplete, keep the Stage 1 refusal.
5. Define the realization-coordinate law independently of callers, hidden
   seeds, enumeration order, transcript ordinals, and observer state.
6. Only after both leaves close, issue the first `[X]`, derive `[W]` identity,
   and expose SI-native typed adapters for collapse, disk formation, and embryo
   systems.
7. Carry the same bitstream and refusal discipline through assembly,
   composition, orbits, moons, young thermal state, crust, mantle,
   geodynamics, atmosphere, hydrology, loads, flexure, and immutable snapshot
   transport.
