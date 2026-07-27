# PR #215 live-source audit

Status: implementation audit for the canonical integration on PR #215 branch
`claude/thermoelastic-rung3`, updated 2026-07-26.

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
    requires `civsim.units.physical-floor-authority-binding.v5` before comparing
    or exposing execution magnitudes. The v5 seal binds tier and provenance for
    every physical admission, the independent
    `civsim.units.floor-catalog-admission-pair.v3` receipt, the independent
    `civsim.units.codata-2018-floor-evidence-pair.v2` custody receipt, and the
    paired exact fixed-math table receipt. The capability remains private and
    does not accept caller values.
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
    canaries. The physical-floor v5 authority binding includes that agreement.
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
    inverse-log-two, every CORDIC angle, inverse gain, order, and production
    occurrence in the two canonical CPU and GPU implementation files. That
    narrow table claim is active and bound into the floor; whole-domain error, rails, iteration
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
45. **The fixed-math occurrence claim omitted most CORDIC members.** Both
    implementations counted only the scalar constants, so a copied CORDIC
    angle could survive while both receipts agreed. Their CRLF self-tests also
    searched raw checkout bytes with LF-only mutation anchors, and their checked
    receipt comparisons accepted only an LF checkout. Receipt v3 covers all 32
    angle members, mutates canonical source, and canonicalizes checked receipt
    text while rejecting bare carriage returns.
46. **The first v3 repair shared Rust lexical blind spots and stopped before the
    floor consumer.** Both implementations accepted a whitespace-varied core
    constructor, a hexadecimal GPU literal, a GPU literal after `"//"` inside a
    string, and production code hidden behind commented `cfg(test)` text while
    still agreeing on 77 occurrences. They could also reject matching text in
    comments or strings. Separately, Python accepted a CRLF checked receipt but
    Rust embedded and hashed the raw checkout bytes. The producer now uses a
    Rust token stream; the watchdog independently masks non-code bytes and then
    uses structural regular expressions plus a brace walk. Direct integer
    literals normalize across binary, octal, decimal, hexadecimal, underscore,
    suffix, and whitespace forms. Both exclude only complete test modules and
    the same three exact non-Q32 GPU contexts, and both resume into later
    production code. The reproduced bypasses are refusal canaries and non-code
    lookalikes are acceptance canaries. Rust now canonicalizes the receipt to LF
    before the physical-floor digest, rejects bare carriage returns, and proves
    LF and CRLF produce one digest. No table bit or physical magnitude changed.
47. **Token-aware scans still did not make the claimed production set exact.**
    Both implementations initially accepted raw `r#from_bits`, unsuffixed GPU
    copies, duplicated or removed non-Q32 exemptions, a `cfg(test)` marker
    inside a macro token tree, and Unicode Pattern White Space. The repaired
    pair requires ASCII source, normalizes raw identifiers, covers unsuffixed
    and `i64` GPU literals, permits test exclusion only for a real root module
    item, and requires one word-width plus two separately named quadrant roles.
48. **Authority roles and exemption roles could still come from syntax that did
    not own the live computation.** Raw line parsers accepted correct
    declarations in comments while a differently written live declaration was
    wrong. Macro inputs, tool-attribute arguments, dead `cfg` items, and moved
    exemption statements exposed the same distinction. The final pair now
    parses required roles from independent production-only views, keeps
    selected literals inside attribute and macro token trees visible as
    unbound occurrences, rejects every production Rust configuration mechanism
    except root test modules, and compares every role by exact path, line,
    value, and multiplicity. The three GPU exemptions are additionally bound to
    complete declaration inventories inside `fixed_ln`, `fixed_sin`, and
    `fixed_cos`. ASCII vertical-tab and form-feed whitespace share the same
    lexical meaning in both implementations, and both exclude nested
    conditionals inside an already excluded root test module. Thirty-six
    canaries cover the reproduced cases. The checked receipt is SHA-256
    `b1906c81dd72c793fa3948d4864cbf6c210bb8f2749ec83cc533f0e1664c84b7`,
    and the resulting physical-floor authority digest is
    `fa324850646f2bf2d0c4d30951ce68b1c1867662673676af1fec6bbb0e8d931e`.
    No Q32.32 table bit or physical magnitude changed.
49. **CODATA canary hashes proved outcomes but not the exact mutations.** Cases
    that reached the same error text could be substituted without changing the
    earlier canary digest. Canary execution v2 now constructs concrete changed
    bytes, rejects a candidate equal to its baseline, and binds a
    domain-separated mutation digest plus detector-specific refusal code for
    each of 13 producer and 14 watchdog cases. The mandatory offline route
    executes those transcripts; the explicit online audit remains the distinct
    live-and-archive source-byte check.
50. **Stone 0 trusted a PATH-selected Python process to report its own
    success.** An exit-zero shim could skip every authority script, and the
    first receipt repair disclosed its token in the shim's arguments. The
    supported Linux and WSL route now accepts only fixed absolute Python
    candidates under a root-owned, non-group-writable, non-world-writable
    ancestry, rejects set-id interpreters, scrubs import and dynamic-loader
    injection, disables both site paths, and requires one wrapper-completion
    receipt. A replaying PATH shim writes a sentinel; success requires that the
    sentinel remain absent. The nested Rust path is bound to the external Cargo
    toolchain that launched Stone 0. Native Windows fails closed and uses WSL.
51. **Future physical-registry artifacts could present receipt-shaped data as
    authority.** The live zero-member path was safe, but any non-root
    activation could have supplied distinct nonzero digests without proving
    their roles. Production artifacts now require a private opaque
    `VerifiedAdmissionCapability`. Only the verified repository-root pair can
    mint the current capability, which binds artifact identity, exact
    admission, and pair receipt. Both validators independently reject drift.
    Future non-root mints remain blocked on their own claim-specific authority
    pairs.
52. **One retained plasma fold rounded before an unproved square root.** The
    prior path certified a Q32.32 radicand and then called `Fixed::sqrt`, so
    activation would have combined double rounding with a blocked kernel. A
    recursive-descent producer now selects the terminal positive-square-root
    floor cell from the complete exact radicand interval. The shunting-yard
    watchdog independently proves the candidate's squared cell against both
    intervals. The production bits remain `5043327366`; the later
    state-dependent carrier-density square root remains a separate blocked
    kernel-authority question.
53. **The physical-root agreement receipt did not bind its source ancestry.**
    The escaped artifacts carried ancestry, but the producer and watchdog pair
    receipt could be reconstructed from ancestry-neutral semantic projection
    bytes. Producer and watchdog now construct separate canonical ancestry
    manifests over candidate kind, source identity and tier, artifact
    identity, and ancestry digest. Receipt v5 binds both equal nonzero manifest
    digests, both checker claims bind the same ancestry fields, and a sixth
    post-projection canary proves that a coordinated ancestry and capability
    substitution loses authority even when semantic projection bytes remain
    unchanged.
54. **Floor-admission canaries collapsed different refusal meanings into one
    outcome.** A semantic defect and a structurally valid alternate canonical
    claim both reported only `checker.refused`. Canary execution v2 now pins
    three stable classes: malformed input, semantic validation, and canonical
    input custody. Provenance and receipt-semantic defects exercise semantic
    validation; alternate tier, member, and receipt-authority claims exercise
    custody only after their structure remains valid.
55. **Floor arrival order could steer transcript construction.** The admission
    pair correctly canonicalized an order permutation, but transcript
    construction zipped the admitted entries positionally against the three
    expected physical coordinates. The transcript now joins each sealed
    structural entry to its typed audited magnitude by exact identity. A
    differential test reverses the admitted floor and proves identical run
    receipts and observer bytes.
56. **Direct mandatory Stone 0 detections were owner-override eligible.**
    Ordinary provenance findings and direct mandatory authority commands fed
    one overrideable collection, so a valid owner token could suppress a
    mandatory authority detection. Stone 0 now separates ordinary
    override-eligible detections from non-overridable operational and direct
    mandatory failures. Its self-test proves direct semantic and operational
    failures remain hard while the ordinary owner boundary is preserved.
57. **The Stone 0 build guard selected its authority root through ambient
    `git`.** The build anchor derived the real workspace root from
    `CARGO_MANIFEST_DIR` for rerun inputs, then called a rootless gate entry
    point that accepted `git rev-parse --show-toplevel` from `PATH`. A shim
    could therefore redirect every mandatory check to a decoy tree while Cargo
    compiled the real workspace. The anchor now passes its manifest-derived
    root explicitly. Stone 0 canonicalizes it, validates repository identity,
    and requires equality with the top level returned by a root-owned absolute
    Git executable under a cleared environment. CLI discovery walks filesystem
    ancestry and then applies the same validation. A fake-`PATH` Git sentinel
    proves ambient redirection is ignored. Independent review then found that
    the first wiring repair observed only the call site, so deletion of the
    trusted-Git equality could still receive a green wiring receipt and
    self-test. The wiring pair now includes and independently scans the Stone 0
    library boundary, and a shaped decoy repository that passes file identity
    checks must still refuse because it is not the trusted Git top level.
58. **Conflicting Stone 0 command-line modes could suppress the CI scan.**
    Argument selection preferred `--self-test` whenever it appeared and ignored
    extra arguments, so `--ci --self-test` ran only the self-test. The binary
    now accepts exactly no argument, `--ci`, or `--self-test`. Unknown,
    duplicate, conflicting, and positional arguments return usage status 2
    before any gate mode runs.
59. **Repository-root canary agreement did not bind the mutations or observed
    outputs.** Packet entries hashed only the canary label and expected refusal
    code, while post-projection entries hashed only a label and fixed expected
    phrase. One implementation could therefore replace an uncertainty mutation
    with a different mutation that produced the same refusal and retain the
    paired digest. Canary suite v5 independently encodes every exact packet
    mutant and full observed checker result through separate producer and
    watchdog codecs. A second paired transcript encodes every
    post-projection mutant, complete receipt, admitted artifact, and opaque
    capability binding plus the observed verifier result. Only that
    transcript's own digest field is excluded to avoid self-reference. Both
    final verifiers recompute it. Same-code packet and projection falsifiers
    now produce distinct preimages and distinct observation transcripts.
60. **A structurally consistent refusal receipt could author its own
    evidence.** The parent physical-registry consumer called a verifier that
    checked identities, stage consistency, field relationships, and
    self-digest, but never replayed the current sealed pair. A parent-visible
    constructed receipt could therefore pass without representing the current
    decision. Structural verification and synthetic receipt construction are
    now private to the root module. The production boundary first checks that
    structure, reruns `decide_repository_roots()` over the current sealed
    inputs, and requires the fresh decision to be the exact same refusal.
    Tests prove an internally consistent synthetic refusal remains useful for
    mutation coverage but cannot verify as current production evidence.
61. **Both root projectors shared the dimension reconstruction that selected
    pure mass.** Producer and watchdog called the same SI-exponent mapper and
    compared against the same mass constant. Their apparent pure-mass mutation
    changed the sealed packet first, so it observed only a seal mismatch and
    never reached the classification predicate. The v6 implementations now
    construct the seven current SI mappings through distinct traversals and
    build separate pure-mass expectations from the domain-separated mass axis.
    The seal mutation remains under an accurate binding name. A separate
    non-admitting v6 canary binds exact positive dimensionless and pure-mass
    source inputs to each live classification predicate and requires zero
    versus one mass projection. It cannot mint an artifact or capability.
62. **Formula producer and watchdog shared terminal cell decisions.** Both
    paths used the same rational rounding helper, and significance mode also
    used the same `floor_log2` helper to select its magnitude cell and target
    exponent. The v4 watchdog now derives quotient, remainder, midpoint,
    parity, sign, signed range, and the exact power-of-two cell from raw
    rational components without calling either producer-side selector. Exact
    midpoint, negative parity, positive-exponent, signed-boundary,
    power-of-two-boundary, forged-certificate, and overflow falsifiers cover
    the terminal decisions. Its Pi path uses independent dyadic floor and ceil
    operations, direct signed enclosure tests, and a runtime containment
    assertion.
63. **A post-projection canary failure produced a refusal its verifier
    rejected.** The live finalization branch labeled `CanaryFailure` as
    `ProjectionReceipt`, while the refusal verifier allowed only
    `ReceiptInvalid` there. Debug builds could panic and release builds could
    emit an unverifiable refusal. Refusal receipt v5 permits
    `CanaryFailure` at that stage only when the pre-projection canary digest is
    present, resource contracts match, and the projected pair agrees. A
    private production-used stage mapper and its direct test prevent the live
    branch from drifting back to the earlier `Canary` stage.
64. **Representation failure re-entered the same failed projection.**
    `RunReceipt::refused` constructed an empty transcript through
    `RepresentationReceipt::sealed().expect(...)`, so the recovery path could
    repeat the cached error and panic. The pipeline now attempts noncausal
    representation before constructing the audited execution view. Failure
    becomes one closed V10 transcript carrying the V1
    `representation_unavailable` status, a stable reason, zero representation,
    floor, or derived values, no snapshot, and one terminal refusal event. If
    structural refusals were already known, they remain in canonical order
    beside the representation refusal. The successful V10 wire remains
    unchanged; complete representation fields imply availability, and the V1
    status fields appear only for the unavailable variant.
65. **The first viewer projection exposed a non-observer planet status
    type.** The receipt was immutable, but exporting the planet enum through
    `civsim-viewer` broadened the mechanically enforced observation boundary.
    The viewer now maps the borrowed receipt into its own local scene status.
    The planet boundary gate confirms that the viewer remains an immutable
    observation leaf.
66. **The common raw decimal parser still has no shared admission cap.**
    Canonical certified ingress already bounds source bytes, significant
    digits, net powers, and coordinate shifts before large-integer expansion.
    Other raw text or TOML loaders can still reach the common parser without
    one cross-loader contract. This is recorded as
    `P-BOUNDED-DECIMAL-ADMISSION`, not repaired incidentally in a root audit.
    No caller-authored decimal or raw loader enters the current canonical
    planet path.
67. **SI representation-scale authority remains explicitly blocked.** Formula
    producer and watchdog agreement proves the requested arithmetic cell, not
    that the table-wide scale policy chose the physically lawful request.
    `units.si-representation-policy` remains blocked with no readiness effect.
    The new unavailable state prevents a failed policy projection from
    fabricating values; it does not promote the policy to an active authority.
68. **The affected replay packet has bounded source limits.** Raw CODATA bytes
    remain outside the packet because the repository holds factual-row
    receipts rather than redistributable source bytes; the explicit online
    audit is the separate custody proof. GPU kernel source and several Python
    gate implementations are also outside this three-lens root/formula/refusal
    projection. Those omissions limit the verdicts that may be claimed from
    this packet and are not silently treated as passes.
69. **Formula parsers assigned different precedence to unary signs and
    exponentiation.** The recursive-descent producer applied a leading sign
    before a following power, while the shunting-yard watchdog bound the power
    first. A lawful positive formula such as `5 + -2^2` therefore selected 9 in
    one path and 1 in the other. Recursive producer v5 now parses
    term, unary, power, then primary while the independently retained watchdog
    v6 continues to apply its precedence table and independently requires each
    exponent operand to be exactly one unsigned integer literal. Grouped
    negative bases, explicit grouping, signed multiplicative operands, and
    chained signs all certify the same terminal integer through both paths.
    Signed, grouped, decimal, and chained exponent operands refuse in both
    paths.
70. **The physical-registry producer could reject a power that fit the
    declared component bound.** It multiplied the base bit length by the
    exponent before computing the result. That estimate rejects `4^10` under a
    25-bit component cap even though the exact result has 21 bits and the
    complete mass expression remains in range. Physical-registry producer v5
    now uses its own left-to-right bounded exponentiation and checks each
    materialized product. Watchdog v4 retains its separate right-to-left
    rational exponentiation. A direct unfamiliar-input canary proves both
    validators accept and byte-agree on the counterexample. This repair grants
    no species authority; the live registry still has no admitted derivation
    rule.
71. **Variable-cardinality dimensional analysis could materialize an
    unbounded quadratic basis.** A large set of unique dimensionless columns
    has rank zero and one dense vector per free column, so the public analyzer
    could reach allocator failure without a typed refusal. Analysis now checks
    the worst-case augmented dense-basis cell count before matrix or basis
    construction and returns `BasisCellCapacityExceeded` above 1,048,576
    cells. The limit is an execution-resource contract, not a seven-axis or
    familiar-physics assumption. Larger lawful censuses require a partitioned
    or sparse authority path instead of silently changing the physical
    verdict.
72. **Formula certificates did not attest that their declared canaries ran on
    the production implementations.** Producer and watchdog now own separate
    nonrecursive, cached production suites with distinct fixture construction,
    encoders, transcript domains, and more than 40 cases each. Fixed-scale,
    significant, and positive-square-root issuance retrieves both successful
    attestations before emitting a certificate. Each issued certificate and its
    receipt digest bind each suite's schema, identity, case count, and complete
    transcript digest.
    The suites live in separate child modules so the active parser and
    evaluator files do not become a second shared canary oracle.
73. **The physical-registry validators inherited one shared resource oracle.**
    The v5 producer and v4 watchdog now own separate production cap tables,
    resource-contract constructors, work constants, and digest encoders. Each
    refuses an advertised input contract that differs from its local contract,
    and the pair refuses unless both independently reproduced resource digests
    agree. Resource-contract v2 binds the limits, semantic-work profile,
    operation weights, power formula, and closure formula rather than sealing
    caps alone.
74. **Algorithm-local work meters could turn lawful bounded input into checker
    disagreement.** On the minimal power fixture, producer charged 146 units
    and watchdog 152; on an elementary closure graph they charged 1 and 3.
    Semantic-work profile v1 now meters canonical wire bytes, expression nodes
    and edges, executed operations, rational work, identities, hashes, and
    registry emission independently of traversal helpers. Both paths accept the
    exact evaluation boundary at 149 and refuse at 148. Closure preflights
    `2V + 2E + D`; producer uses a linear indegree elimination while watchdog
    retains DFS, reverse tracing, and a forward worklist. Exact-boundary,
    permutation, multibyte-rational, negative-power, and digest-substitution
    canaries cover the contract.
75. **Power multiplication could allocate one over-limit temporary before
    refusing it.** The prior behavior did not breach the exact-value bit cap and
    was therefore not a verdict defect, but it left allocator work outside the
    preflight contract. Producer and watchdog now independently divide the
    maximum allowed value by opposite operands before multiplication. A product
    that cannot fit refuses without materializing the oversized temporary.
76. **The floor's reviewed prose pins did not express the complete irreducible
    admission route as a distinct owner capability.** Floor-catalog admission
    v3 now domain-separates derivation exhaustion, Buckingham Pi, Gap Law, the
    typed Chaos Protocol branch, Residual Law, and the unique residual slot for
    every leaf. An explicit owner-admission record pins each exact six-receipt
    tuple. The producer reconstructs those records from typed receipts; the
    independent byte parser reconstructs them from the canonical claim and
    rejects missing, changed, or mismatched owner decisions. Changed receipt
    prose or identity can no longer remain structurally admitted without a
    separate reviewed owner-admission change. The public sealed view exposes
    all six receipts, the owner receipt, and the independent watchdog receipt
    per identity. These repository pins are durable claim-scoped review
    capabilities; they do not claim cryptographic proof of the human reviewer.
77. **Formula projection shared exact arithmetic below its independent parser
    and rounding paths without production known-answer coverage at large
    widths.** Producer canary suite v2 now binds the frozen
    `round(pi * 2^96)` result and a separate 192-bit rational identity.
    Watchdog suite v2 independently binds `round(pi * 2^80)` and a different
    240-bit rational identity. Every issued formula certificate already binds
    both suite identities, case counts, and transcript digests, so a shared
    multi-limb defect or duplicated Pi specification must now disagree with at
    least one external frozen result before production issuance.

Two panel objections did not justify code changes. Shared exact `BigRat` and
`BigUint` operations are declared low-level primitives in the authority
contract; producer and watchdog still choose terminal quotient, midpoint,
parity, sign, magnitude, and rounding cells independently. The sealed-packet
mutation suite claims exact custody and input binding, while the separately
named positive dimensionless versus pure-mass canary reaches the live
classification predicate below the seal. Neither objection demonstrated a
claim violation. The absence of raw historical CODATA endpoint bodies remains
the explicit online-audit limit recorded in finding 68.

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
were added because redistribution rights were not established. The repository
does hold the three reviewed row facts under
`civsim.units.codata-2018-floor-facts.v1`, SHA-256
`c0839f9c3497724f04522b70699c969d225db62baad5507638162f274259da11`,
and the independent fixed-column producer plus record-scan watchdog receipt
under `civsim.units.codata-2018-floor-evidence-pair.v2`. The receipt file has
SHA-256
`08c67f6d6e4543e7cb35be383f08aa1bbe7361878ff78bbc8567d08104f97983`
and binds pair digest
`7aef3108ee63e017d739be0fd9b229a7d1fac675f35e81fbdfaa7a7844696ee0`.
The resulting physical-floor authority digest is
`75a20e5b22cd52dadb1beb44cba1a1b16bbaf45a353b32510c67753403932933`.
Those repository bytes are custody facts and receipts, not redistributed NIST
source bytes. Custody proves what was consulted; it does not admit a physical
value.

## Previous canonical runner receipt

Before findings 72 through 75, two direct no-argument runs produced:

- exit code `2` on both runs;
- zero stderr bytes;
- six transcript events;
- no `[W]`, no `[X]`, and no snapshot;
- byte-identical stdout of `373,048` bytes, SHA-256
  `3bb77971d7e61793356654366979b6df31e1c14139c5c4a3245f3ca9099a63c1`;
- receipt `civsim.planet.run.v12` and transcript schema major `10`;
- refusal `stellar_birth.realization_measure` at Stage 1.

The `--readiness` front-door alias returns the same exit, stdout, stderr, and
digest as the no-argument run. It does not produce a separate readiness result
or supply readiness evidence.

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
the v3 species derivation analysis under its v4 watchdog. It binds the floor
digest, exact mass
anchor, live root-pair claim, paired receipt, and physical-registry refusal to
the validated open schemas. The root frontier contains three exact scalar
coordinates and one membership-neutral pure-mass expression, then reports zero
candidate members, zero verified support, no value payload, no residual slot,
and the open obligations emitted by the first live physical-registry refusal.
The earlier producer and watchdog repeated the same authored three-path list,
so their agreement could not support a completeness reading. The repaired
checker compares the one reached attempt to a fresh registry frontier and
rejects omitted live obligations. Unreached downstream paths are absent rather
than predicted. The receipt states
`frontier.scope=first_executable_refusal_only` and
`frontier.completeness_claim=false`. Its status is `open_dependencies`, its
`closure_effect` is `none`, and its `coverage_claim` is `false`. Gap Law, Chaos
Protocol, and Residual Law remain not reached. The open coordinate-law leaf
carries analysis count zero. The viewer borrows the same diagnostic contract
and cannot use it to choose or alter an outcome. The checker proves propagation
of the current refusal, not completeness of the registry's own diagnostic
vocabulary.

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

The 2026-07-26 final-slice audit confirms one active claim-scoped
`eps_0 = e^2 / (2 * alpha * h * c)` derived relation and no species-forming
authority. The bounded panel found and repaired supplied-scale authority,
pair-receipt, mutation-transcript, semantic-closure, observer wording, viewer
evidence, and replay-cost defects. A frozen repair replay found one remaining
shared canary-transcript identity. Producer and watchdog now use distinct
transcript identities and domains, and pair receipt v2 independently binds
both identities, case counts, and complete transcript digests. The authority
and observer replay lenses pass with no further source-confirmed defect. Five
focused relation tests, nine Stage 1 authority tests, and the typed viewer
projection test pass. Two direct canonical runs from the explicitly rebuilt
binary each exit `2`, write no stderr, and produce byte-identical 393,806-byte
receipts at SHA-256
`f6814b0036217723d7e763b38f993a6479cc6fd6c816c54ac6e42d30af6159a4`.
Warnings-denied planet and viewer Clippy and the 101.1-second `check-fast`
pass. The complete PR route passes in 1,485.8 seconds with every declared gate,
all canonical all-target tests, deterministic integer and CPU/GPU parity
checks, private-item documentation, and doctests.

The 2026-07-23 baseline had 71 planet library tests, eight CLI tests, three
viewer tests, a 1,178.5-second complete Linux `check-pr`, and a 78.3-second warm
`check-fast`. Those timings and counts describe that earlier candidate, not the
current root-verifier repair.

Current focused repair evidence includes all 159 units library tests, five
units determinism tests, the units i256 integration test, and 21 selected
physical-registry tests. The physical set covers independently encoded
resource profiles, exact evaluation and closure boundaries, rule-order
invariance, multibyte rationals, negative powers, preallocation refusal, and
unfamiliar dimension axes. The earlier candidate's broader crate and replay
results below predate findings 72 through 75. A fresh full planet run,
warnings-denied Clippy, bounded panel, merge-grade `check-pr`, direct replay,
and exact Stop hook remain publication checks until recorded here.

Two v12 direct binary runs after the v5 root-agreement, v5 refusal, CODATA
custody, variable-axis, and paired final-verifier repairs produced exit `2`,
empty stderr, `373,048` stdout bytes, byte equality, and SHA-256
`3bb77971d7e61793356654366979b6df31e1c14139c5c4a3245f3ca9099a63c1`.
That is a pre-findings-72-through-75 receipt and is retained only as historical
evidence. It will not be promoted as the current bitstream until a fresh direct
replay produces a new byte-for-byte result.

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

## Conditional physical registry audit

The physical species proof-graph slice received one frozen six-file audit
packet, not repeated outcome-seeking runs. Its manifest SHA-256 is
`0417df650ce9dbfcccbd85834c701d0525558329d6fbe39192a147db69432ea7`.
The first strongest-model smoke blocked because the packet omitted identity,
upstream-call, receipt-boundary, terminology, independence-scope, downstream,
and ledger-taxonomy facts needed to judge the source. The corrected packet
added those facts without changing the candidate; the second smoke returned
`CLEAR`. One blind generalizer then supplied nine falsifiers. Every alleged
source defect was checked against the live files before repair.

Eight findings reproduced. The shared mass dimension used the length axis;
derived and irreducible admission receipts were not required to be pairwise
distinct; a caller could hide a dangling composite dependency outside its
declared closure; watchdog graph and expression walks could recurse beyond
their stated depth boundary; producer expression work depended on node storage
order; producer exact arithmetic constructed avoidable oversized
intermediates; one rational bit-length addition was unchecked; and verified
members omitted their physical-content artifact. Repairs use the literal SI
mass axis in both validators, canonical ledger enums, distinct complete receipt
bindings, whole-rule dependency closure, explicit bounded stacks and
topological worklists, cross-reduced rational operations, checked size
arithmetic, bound checker identities, and physical-content bytes in each
member. Focused canaries cover deep chains, reversed storage, cancellation,
dangling dependencies, noncanonical provenance, and receipt collapse.

The remaining alleged defect, a missing separate big-endian `u64` list count,
was rejected as a packet-contract overstatement. The source schema defines
lists as repeated typed TLV fields whose payload lengths and enclosing record
length make the sequence self-delimiting; it does not promise a separate
element-count word. Adding one would change canonical identities without
closing a real ambiguity.

The repaired pair gives no production species authority clearance. Synthetic
unfamiliar and massless graphs prove bounded validator agreement only. The
current v4 registry and proof graph encode bounded variable-cardinality
dimensions over domain-separated axis identities. Seven SI axes describe the
current floor, but the type, arithmetic, and wire do not close the physical
basis at seven; focused tests admit and cancel unfamiliar axes. The
repository now supplies three exact scalar coordinates and one
membership-neutral pure-mass projection derived from the sealed floor. It has
no admitted species-forming rules and returns
`no_admitted_species_derivation_rules`, with zero members, no coverage claim,
and no authority effect. The live analysis serializes that exact frontier and
the viewer borrows it read-only. It exposes no reducer, conditioned support,
ledger effect, Stage 1 proof, or causal viewer path, and leaves
`planet.species-state-support` blocked.

## Physical-root pair hostile audit and repair

A subsequent hostile audit confirmed seven defects or incomplete claims before
the coordinate bridge could be treated as active:

1. A common adapter performed the sole floor lookup and source association, so
   both checkers could agree on the same mislabeled packet.
2. Receipt v1 omitted separate checker results and resources, claim identity,
   executable canary binding, decision state, and refusal evidence.
3. Downstream validators treated generic receipt strings as proof-shaped data
   and did not yet revalidate a root capability against its exact scope.
4. The root pair existed only in tests and did not enter the live Stage 1
   refusal.
5. The claim promised ordered leaves while both paths normalized source order.
6. The declared canary list exceeded executable mutation coverage.
7. The authority row omitted live floor, ledger, catalog, and adapter
   dependencies from semantic closure.

### Verified repair note, 2026-07-25

The follow-up source pass confirms a live `TranscriptSchema::V10` selector for
the v10 transcript contract and preserves `TranscriptSchema::V9` as a distinct
v9 selector. Producer and watchdog independently canonicalize source arrival
order by semantic coordinate identity, so a sealed source reorder is
nonphysical and produces the same result. A coordinated entry-id and symbol
rename that retains the old receipt returns the typed
`sealed_source_binding_mismatch` refusal from each implementation.

The post-projection canaries call both production verifiers over a private
candidate manifest. The executable v6 suite catches artifact omission,
artifact duplication, `MembershipNeutral` to `SpeciesRestMass` scope
escalation, canonical projection byte drift, admission-receipt substitution,
and coordinated ancestry plus capability substitution. Producer and watchdog
also construct separate ancestry manifests over every source identity, tier,
artifact, and ancestry digest, and receipt v5 binds both equal nonzero
digests. Each final verifier first re-extracts and reprojects its current
sealed packet, exact-matches the manifest, canonical bytes, counts, and
ancestry, reconnects both receipt resource digests to its local resource
contract, and recomputes its own packet-canary execution digest. A
self-consistent unsealed projection therefore fails the final boundary. Each
packet-canary transcript entry binds the complete exact mutant and observed
checker result. A second paired transcript binds every post-projection mutant,
receipt, admission, opaque capability, and observed verifier result except its
own digest field. Both final verifiers recompute it. A production refusal must
also exact-match a fresh current sealed-decision replay; structural synthetic
receipts remain test-only. The generic alien-coordinate projector remains
test-only inspection output and cannot mint an admission receipt or root
capability.

The root success receipt is
`civsim.planet.stellar-birth-repository-physical-root-receipt.v5`; the refusal
receipt is
`civsim.planet.stellar-birth-repository-physical-root-refusal-receipt.v5`; the
canary suite is v6. The root packet and projection remain v5; the producer and
watchdog implementations are v6; coordinate content is v3. The active
authority profile hash is
`7963fbd5fb857fb2516e11579e76409c5bdfc3cc53d172b15251016b87d80750`.
This repairs the downstream verification gap without granting scientific
sufficiency to the floor. Every species-forming field, operator, state, sector,
validity, constraint, excitation, bound-state, mass-uncertainty, and
massless-law root remains open. The live result remains zero members, no
coverage claim, and no authority effect.

### Bounded live vocabulary audit and repair, 2026-07-26

One read-only audit of the newly bound physical-vocabulary path found two
material generality defects. First, the repository frontier accepted only
`no_admitted_species_derivation_rules`, so the next lawful
`physical_vocabulary_coverage_incomplete` refusal would have been reclassified
as an internal error. Second, the frontier required every root to be
Universal `[D]` through the derived route and serialized one aggregate label,
even though the admission and vocabulary validators already accept lawful
irreducible and unfamiliar roots.

The live frontier now accepts exactly those two scientific refusals and still
rejects checker, binding, resource, and integrity failures. It derives a sorted
identity-keyed census over every admitted root, recording the exact ledger
tier, canonical provenance tag, and derived, irreducible, or evidence-custody
route. Duplicate identities or noncanonical provenance refuse. Species
analysis v4, watchdog v5, the canonical wire, typed planet view, and
observer-only viewer carry the same rows without a causal return edge.
Focused tests cover both accepted refusal codes, rejection of checker
disagreement, the current four Universal `[D]` derived rows, and one unfamiliar
Residue `[M]` irreducible row.

The independently classified current vocabulary remains exactly zero
descriptor roles, four relation targets, and zero constraint laws.
`current_input_partition_complete=true` is scoped to those four roots;
`global_physical_vocabulary_coverage=false`,
`membership_authority=false`, zero registry members, and no authority effect
remain unchanged. No repeat audit packet or semantic smoke loop was run after
the two confirmed fixes.

## Remaining audit debt

- Admission structures still carry human-readable receipt strings as evidence.
  Those strings cannot mint production authority: the current root pair mints
  one private capability bound to exact admission data and its pair receipt.
  Future leaves at any tier still need typed, machine-resolvable artifacts and
  a claim-specific capability mint before they can become live.
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
- The variable-cardinality dimension encoding accepts unfamiliar nonzero axis
  identities and does not close the basis at seven SI axes. Before unfamiliar
  axes can enter a production artifact, a repository-owned basis registry and
  independent admission pair must prove each axis identity and semantics. The
  current sealed input contains only the seven domain-separated SI identities,
  so this is activation debt rather than a live selector.
- Species and index-domain schemas still need repository-owned admitted field,
  operator, state, sector, validity, constraint-law, excitation, bound-state,
  mass-uncertainty, and massless-law roots, plus a production registry authority
  and replay binding. The four live floor-derived roots are coordinate
  capabilities only. The production adapter now lets the physical registry
  consume them only after both final verifiers independently bind artifact
  identity, floor authority, paired result, canonical bytes, admission receipt,
  and membership-neutral scope. The conditional
  physical pair checks content identity, exact dimensions and mass, dependency
  closure, and typed capacity refusals when synthetic roots are supplied, but
  synthetic agreement is not evidence that an unfamiliar system is physically
  admitted. Derived chart and time-reference handling, convergence control,
  conditioned support, and joint-measure applicability remain open.
- The retained state-dependent carrier-density fold still calls
  `Fixed::sqrt`. The complete-expression plasma constant is independently
  certified, but this later dynamic kernel remains under the blocked
  deterministic-kernel authority claim and cannot enter canonical state until
  its domain and CPU or GPU parity are independently certified.
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
   correlation-carrying joint measure. Before activating any unfamiliar
   dimension axis, close its semantic identity and algebra through a
   repository-owned basis registry and independent admission pair.
2. Preserve the four membership-neutral floor-coordinate capabilities and the
   claim-scoped `eps_0` relation without treating either as species authority.
   Then derive or fully admit the repository-owned field, operator, state,
   sector, validity, constraint-law, excitation, bound-state, species
   mass-uncertainty, and massless-law roots consumed by the landed physical
   registry pair. Every non-root admission needs a claim-specific capability
   mint backed by its own independent authority pair. Bind the resulting
   complete registry to a separate production authority and replay identity,
   then close conditioned joint support. Only then activate the landed exact
   weighted reducer.
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

## 2026-07-26 first derived-premise capability audit

The first active non-root law-premise pair reconstructs the sealed
`eps_0 = e^2 / (2 * alpha * h * c)` execution coordinate. It consumes no
recorded `eps_0` decimal and no calibration target. Its output remains a
claim-scoped execution relation, not an electromagnetic field, operator,
sector, excitation, or species.

One bounded five-lens panel passed admit-the-alien and familiar-outcome review
and failed confirmation-bias, derive-vs-admit, and observer review. Source
verification confirmed eight repair classes:

1. the supplied output scale was not independently authorized;
2. the pair receipt omitted implementation identities, canary counts, and the
   explicit agreement decision;
3. accepted canaries did not bind complete checker outputs;
4. the advertised output-dimension canary changed relation dimension instead;
5. output scale had no mutation canary;
6. the active semantic closure omitted transitive adapters and primitives;
7. viewer wording conflated a species-support value with the adjacent derived
   coordinate and omitted claim-scope evidence;
8. production and rendering repeated sealed authority work that had already
   been checked.

The repaired v2 pair independently derives the claim-local scale under the
Q32.32 significand floor and signed-i128 capacity bound. Producer and watchdog
each execute 12 mutations, bind each complete mutant preimage and complete
observed result, and expose separate implementation, result, transcript,
case-count, and digest identities. The pair receipt binds the distinct
producer and watchdog transcript domains plus those fields and `agreed`
before capability minting. The generic selector is a non-authorizing post-mint
validation. The active inventory closure now names core fixed-point,
ledger-admission, planet-catalog, units-computation, dimensional, floor
admission, and source-evidence dependencies.

Stage 1 analysis v6 under watchdog v7 carries every claim, role, content,
upstream, applicability, validity, ancestry, checker, canary, pair, and
capability digest. The viewer exposes the same immutable evidence and labels
the false payload bit as
`species_support_value_payload_present`. Rendering is now a pure projection of
the private construction-time-validated artifact. The production frontier
resolves once, and both adapters use the cached sealed SI candidate view while
retaining independent semantic checks.

Five focused derived-relation tests, nine Stage 1 authority tests, the viewer
premise test, planet-boundary gate, and both authority-inventory checks pass.
The frozen repair audit found and then confirmed closure of the shared
canary-transcript identity defect. Two direct runner replays from the
explicitly rebuilt binary each exit `2`, write no stderr, and produce
byte-identical 393,806-byte stdout receipts at SHA-256
`f6814b0036217723d7e763b38f993a6479cc6fd6c816c54ac6e42d30af6159a4`.
The physical registry remains at four roots,
the vocabulary partition remains `0 / 4 / 0`, species membership remains zero,
and Stage 1 still refuses with `no_admitted_species_derivation_rules`.

## 2026-07-26 first primitive-profile hostile audit

The first attempted primitive profile assembled a compact rank-one unbroken
abelian field, operator, state, sector, validity domain, constraint, exact-zero
law, and one proposed excitation member. Before publication, one bounded
admit-the-alien lens source-checked the complete authority path.

Three material findings were confirmed:

1. Producer and watchdog reconstructed the same shared authored theory
   constants. Different traversal and byte framing did not provide independent
   semantic authority.
2. The proposed derive-first, Buckingham-Pi, Gap, Chaos, Residual, and unique
   slot receipts were hashes of conclusion strings. The extractor checked
   schema identities and distinct nonzero digests, but no executable
   derivation attempts, dimensional analysis, gap evidence, residual
   minimization, or collision search existed.
3. The exact-zero object bound a subject, excluded-term label, sector,
   applicability digest, and two differently framed receipts. Neither path
   evaluated a symmetry transformation or proved that the candidate mass
   operator was forbidden in the claimed domain.

The documentary source `pdg_2024_higgs_boson_review` was separately verified.
It supports only the scoped Standard Model statement recorded in the source
registry. It explicitly supplies no project profile, exact-masslessness law,
member, or admission authority. This audit makes no adverse claim about that
source or its authors.

The recommendations were recorded before repair: do not add hashes or repeat
the same checker loop; demote the profile; preserve only general structural
substrate; restore the live scientific refusal; and make executable symmetry
exclusion plus real irreducible-route evidence the next bounded slice.

The repair follows those recommendations. The profile module is compiled only
for tests and its protocol statuses are `assertion_only_not_admitted`. It
retains raw candidate artifacts only and defines no registry capability or
`AdmittedArtifact` conversion. The live physical registry again receives only
four membership-neutral coordinate roots and returns
`no_admitted_species_derivation_rules` with zero members, no coverage claim,
and no authority effect.

The reusable structural improvement remains in registry v5, proof graph v4,
producer v7, and watchdog v6. An exact-zero artifact must now carry a nonempty
excluded term, distinct subject and symmetry identities present in its
requirements, an applicability receipt, and distinct producer and watchdog
exclusion receipts. Tests reject missing scope, collapsed identities, duplicate
receipts, and empty excluded-term content. Exact-massless payloads use
`civsim.physical-species.artifact.v4`; unchanged artifact variants retain v3
identities. These checks prove proof-object shape only. They do not establish
physical truth.

The final bounded smoke pass found four additional seams after demotion:

1. the exact-massless artifact identity domain had not advanced with its new
   proof fields;
2. test builds still let the rejected fixture mint a capability accepted by
   both physical validators;
3. both executable global-vocabulary coverage refusals had been removed to
   permit that fixture; and
4. one contract sentence retained the old root-refusal receipt version.

The recommendations were again recorded before repair. The exact-massless
identity domain is now versioned without changing existing root identities.
The rejected capability and every physical and vocabulary validator match arm
for it are deleted. Both global-coverage checks are restored, and a mixed live
root plus synthetic-rule test reaches
`physical_vocabulary_coverage_incomplete` through both validators. The
contract now records root refusal receipt v5.

Focused validation passes 68 physical-registry tests, nine authority-analysis
tests, five viewer-frontier tests, all eight canonical CLI tests,
warnings-denied focused Clippy, both authority-inventory self-tests and live
agreement gates, the source gate, and generated-source freshness. The repaired
packet passes its exact bounded smoke confirmation. `check-fast` passes in
91.0 seconds. The complete PR route passes in 1,298.7 seconds, including every
declared gate, all canonical tests, deterministic CPU/GPU parity checks,
private-item documentation, and doctests. Two direct runner executions each
exit `2`, write no stderr, and produce byte-identical 393,806-byte receipt v13
output at SHA-256
`3da00e63a98cb78c09c2c2acb6fe212e6660580b249dd4780d27ccd0558bd3b5`.
