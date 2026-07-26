# Independent Authority Rule

Status: active standing rule for the canonical abiotic runpath.

## Rule

A mechanical conclusion cannot supply its own authority. If a checker can mint
a value-bearing capability, close a proof leaf, select a causal branch, change
a provenance or tier classification, or authorize a completed canonical
snapshot, the authority watchdog requires an independently implemented
cross-checker over the same canonical input bytes.

The producer and cross-checker may share canonical serialization, hashing, and
small checked integer primitives. They may not share semantic expected outputs,
budgets, classifications, membership lists, coverage lists, decision helpers,
or acceptance booleans. A self-consistency test may remain useful, but it is
recorded as non-authoritative.

Active agreement produces a claim-scoped receipt. The receipt binds the exact
input digest, claim identity, producer implementation identity, cross-checker
implementation identity, both results, exercised mutation canaries, and the
agreement or refusal. A receipt for one claim cannot authorize another claim.

An authority-bearing mechanism without an independent pair stays blocked. It
may emit a structured diagnostic or refusal, but it cannot mint a proof token,
admit a magnitude, choose a physical branch, or construct a completed snapshot.
The machine-readable status is `scripts/authority_watchdog.toml`.

That inventory is closed by an independently implemented structural pair.
`scripts/authority_watchdog_gate.py` validates schema first, while
`scripts/authority_registry_watchdog.py` starts from a separately coded,
ordered mechanism profile. Each implementation independently parses the exact
TOML bytes, pins every known mechanism's kind, domain, and status, enforces the
fields permitted by that profile, checks repository paths and distinct
implementation identities, prohibits shared semantic helpers, and verifies
the required canaries. Each implementation also carries a separate SHA-256 pin
over every mechanism's complete canonical profile. The pin covers the exact
claim or observation, implementation identities, receipt or diagnostic schema,
canaries, disclosed shared primitives or material, owner boundary, activation
guard and requirements, and ordered semantic closure. Free-text substitution,
claim broadening, and list substitution therefore fail on both sides. Removing
a row, adding an unreviewed row, changing an active or blocked status, or
relabeling an authority mechanism as a diagnostic also fails on both sides.

Each side emits the same canonical receipt over the exact registry bytes and
the digest of every file in every profile's explicit `semantic_closure`. A
closure includes the producer or diagnostic, checker or refusal, and every
reviewed adapter, shared primitive implementation, and orchestrator that can
change the scoped meaning. The primary gate invokes the watchdog in a separate
process and requires byte-for-byte receipt agreement. Truncation, authority
reclassification, diagnostic minting, exact input mutation, receipt mutation,
claim broadening, implementation, schema, and canary substitution, and closure
adapter or orchestrator mutation and omission are mandatory self-test canaries
in both implementations.

The structural pair is enrolled as `governance.authority-inventory`. Its own
paths, implementation identities, and receipt schema are pinned in both
implementations. This is the structural bottom of the recursion: neither
implementation accepts its own observation alone, and no stored receipt is
fed back as an expected answer. Agreement proves completeness against the two
reviewed closed pins and binds the files that implemented the observation. It
cannot discover an authority-bearing mechanism that was never enrolled, so
source review and the owner boundary remain necessary.

Execution of that pair has a separate bootstrap. `scripts/gate_runner.py`
contains an exact independent profile for the authority-inventory gate and
each active claim-specific bootstrap gate. It refuses removal or changes to
any gate field, including identity, order, description, tiers, phase, command,
self-test, timeout, cache policy, input closure, or path triggers.
Stone 0 independently pins the complete authority-inventory gate block before
it invokes the runner. It then directly executes the authority-inventory,
CODATA source-evidence, build-wiring, fixed-math, and external-claim gates as a
second path that the runner cannot suppress. A synthetic integration canary
records all six invocations and proves that a direct-command failure remains
fail-closed. Ordinary provenance policy detections remain eligible for the
owner's one-command emergency override. Direct mandatory-authority semantic
detections and operational failures enter a separate hard-failure channel and
cannot be overridden. Mutation canaries
remove and alter each exact field at both layers. This does not create an
infinite tower: the recursion ends at two diverse executable paths plus the
human CODEOWNERS boundary. Neither executable path decides scientific truth.

Authority subprocess execution follows the same separation rule. A
caller-controlled `PATH` cannot select the interpreter that reports the gate's
success. On the supported Linux and WSL route, Stone 0 resolves only fixed
absolute Python candidates, canonicalizes the target, and requires the
executable plus its complete directory ancestry to be root-owned and neither
group- nor world-writable. Set-id interpreters refuse. Python starts with
isolated, environment-ignoring, no-user-site, no-system-site, and
no-bytecode flags. Python import controls and Unix dynamic-loader injection
variables are removed. The nested Rust tool path comes from the Cargo process
that launched Stone 0, then binds an external canonical `cargo` and sibling
`rustc` pair under owner- or root-controlled, non-world-writable directories.
Native Windows has no equivalent trust root yet and fails closed; the Windows
development route runs the gate inside WSL.

Every successful Python authority call returns one invocation-bound wrapper
completion receipt after the target script exits successfully. A silent
exit-zero shim, a shim that replays the disclosed receipt, a poisoned import,
and dynamic-loader injection are executable canaries. The replay canary also
writes a sentinel and requires it to remain absent, so `Clean` cannot mask
shim execution. These checks prove the named wrapper crossed the declared
operating-system trust boundary and completed. They do not prove mutable
repository gate code is correct or turn an authority result into scientific
truth. The root-controlled Python standard library, dynamic libraries, and WSL
root filesystem, plus the launching Rust toolchain, remain explicit system
trust assumptions.

Authority rows have `kind = "authority"` and `status = "active"` or
`status = "blocked"`. An active row names a distinct producer and checker,
their implementation identities, the scoped receipt schema, shared low-level
primitives, mutation canaries, and exact semantic closure. A blocked row names
the live refusal path, the activation guard, exact activation requirements, and
its semantic closure. Governance authority follows the same pair rule as
scientific authority and is labeled by its separate domain.

Diagnostic rows have `kind = "diagnostic"`, `status = "diagnostic"`, and
`authority_effect = "none"`. Their closed schema has no authority-receipt
field. A diagnostic may use a second implementation to catch drift, but shared
authored expectations keep that agreement non-authoritative. It cannot be
promoted by changing its status; promotion requires an explicit reviewed
schema and gate change.

## Why the rule is general

The rule is the common mechanical form of several existing project laws:

- Buckingham Pi: the declared variable matrix is canonical input. Rank and
  nullity need distinct exact algorithms. Algebra does not prove that the
  chosen variable universe is physically complete.
- Gap Law and Residual Law: the proposer cannot declare its own search complete
  or its own remaining slot unique. Mechanical schema checks need an
  independent receipt check; scientific completeness remains an owner-reviewed
  evidence boundary.
- Chaos Protocol: exact replay checks implementation determinism. It does not
  prove that a stationary measure, regime partition, or transition law is the
  correct physical one. Those claims need independent evidence and falsifiers.
- Diamond Gate: a provider inventory cannot prove the physical correctness of
  the selected provider. Arbitration and provider semantics are separate
  claims.
- Source custody: mirrors, editions, shared samples, shared apparatus, and
  inherited datasets are correlated evidence. They count by independent
  lineage, not URL count.
- Blind panels: repeated runs with the same model, prompt, and supplied frame
  are one correlated voice. A panel result is evidence, not authority by vote.
- Stone 0 and structural gates: canaries and sealed execution prove that a gate
  ran over named bytes. They do not turn a structural pass into scientific
  truth.

## Watchdog classes

Authority-bearing scientific mechanisms require an independent pair. Pure
format, transport, dependency, generated-file, and observer-boundary checks are
registered as structural authority elsewhere. They require integrity canaries
and fail-closed execution, but do not recursively require a second physics
oracle because they are forbidden from satisfying scientific proof leaves.

An algorithm-diverse test is evidence, not an active authority receipt. A
mechanism remains blocked when its tests are test-only, its receipt does not
bind the live claim and inputs, its checker repeats the producer's authored
answer, or its error bound cannot certify the final selected integer. The
current inventory applies that distinction to the SI execution table, SI
representation policy, deterministic CPU and GPU kernels, and wide-integer
arithmetic. The narrow `units.certified-formula-projection` pair is active only
for one formula, its exact ordered point or closed-interval inputs, and one
separately bound fixed-scale or significance request. Both parsers
independently enforce byte, coordinate, decimal, power, nesting, and
intermediate-rational resource bounds before an oversized request can become
authority. Exponentiation binds more tightly than unary signs, and every
exponent operand must be exactly one unsigned integer literal. Grouped
negative bases, explicit negation, signed multiplicative operands, and chained
signs are paired success canaries; signed, grouped, decimal, and chained
exponent operands are paired refusal canaries. The producer selects
its terminal integer through the exact
rational helper, while the watchdog reconstructs quotient, remainder,
midpoint, parity, sign, signed range, magnitude cell, and significance
exponent from raw rational components. Shared exact integer arithmetic does
not select either terminal decision. Formula, fixed-significance, and
positive-square-root production issuance also requires two separately authored,
nonrecursive canary suites to have completed successfully. Each issued
certificate and its receipt digest bind both cached production attestations by
schema, suite identity, case count, and complete transcript digest. Test-only
canaries without this live receipt binding would not satisfy the active claim.
Its positive-square-root subclaim covers one strictly positive
complete radicand at one fixed scale. The recursive-descent producer selects
the terminal floor cell directly from its exact interval. The shunting-yard
watchdog independently evaluates the interval and proves
`q^2 <= radicand < (q + 1)^2` for the candidate and both interval envelopes.
No rounded fixed-point radicand or generic square-root kernel enters that fold.
An invariant coefficient is opaque and cannot expose its normalized bits
through the public API. It can affect production only when its complete
certified rounding cell selects one terminal integer through both parsers and
two independent outer-binding implementations agree on the factor-to-terminal
receipt. Raw arithmetic coordinates still do not carry physical ancestry, so
the planet boundary gate forbids this API in canonical planet source. This
does not authorize the SI execution table, a later state-dependent square-root
kernel, or the table-wide scale-selection policy. Those broader claims remain
blocked until each has its own independent inventory, ancestry, policy, and
aggregate receipt checker. The deterministic CPU and GPU constant table is
active only
for exact derivation, order, source-level direct integer-literal occurrence
coverage, and source binding within the two bound canonical CPU and GPU
implementation files for Pi, half-Pi, log-two, inverse-log-two, the CORDIC
angle table, and inverse gain. Its v3 receipt binds the ASCII canonical Git LF
form of those text files. The two implementations independently map a CRLF
checkout to that form, reject every bare carriage return, and reject non-ASCII
source bytes, so checkout and Unicode-token transforms cannot become authority.

The producer tokenizes Rust and the watchdog independently builds a masked
source view. Only complete root-level `cfg(test)` modules are excluded. Every
other `cfg` or `cfg_attr` attribute and every production `cfg!` invocation
refuses, including raw-identifier forms. Comments, strings, characters,
attribute arguments, macro token trees, and excluded test bodies are
ineligible authority-role sites. Selected numeric literals inside production
attribute arguments and macro token trees remain visible to the occurrence
scan and therefore refuse as unbound. Required declarations are parsed only
from eligible production syntax, and every selected value must agree by path,
line, value, and multiplicity with the independently parsed role receipt.
Binary, octal, decimal, hexadecimal, underscored, unsuffixed, `i64`-suffixed,
raw-identifier, and ASCII-whitespace variants receive one integer meaning.

GPU coverage excludes only three structurally sealed non-Q32 roles: the sole
`let e` word-width declaration in `fixed_ln`, the ordered quadrant declarations
in `fixed_sin`, and the ordered quadrant declarations in `fixed_cos`. Each
function, declaration inventory, selector, and branch is checked independently;
moving or duplicating the same statement does not preserve the exemption.
Thirty-six mutation and acceptance canaries cover role substitution, lexical
forms, conditional compilation, macro and attribute eligibility, exact
exemption ownership, line endings, and non-code lookalikes. The checked JSON
receipt and its Rust physical-floor consumer each map CRLF to canonical Git LF
and reject bare carriage returns before hashing.
The authority inventory's v3 meta-receipt applies the same text contract to
the registry and every enrolled semantic-closure file. Its schema-first
producer removes CRLF pairs directly; its profile-first watchdog walks bytes
and accepts a carriage return only when the next byte is a line feed. Both
reject bare carriage returns and prove native Windows and Linux receipt-byte
equality. Exact-byte research holdings remain outside this repository-text
contract and retain their declared binary custody receipts.
Whole-domain kernel error, domain edges, rails, iteration semantics, CPU and GPU
equivalence, and vendor execution remain one separately blocked claim. The
floor catalog is active through a separate parser that binds
catalog membership, tier, provenance, receipt structure, and canonical input
bytes. Its v2 mutation transcript binds one of three stable watchdog outcomes:
malformed input, semantic validation refusal, or canonical-input custody
refusal. Mutations that remain structurally valid alternate catalogs are
reported as exact-byte custody checks, not as semantic-checker evidence.
The floor-coordinate projection pair is active only for exact rational
projection of those admitted leaves and positive pure-mass identity
projections marked membership-neutral. It cannot authorize a field, operator,
species identity, uncertainty transport, support, or species rest mass. The
pair uses separate sealed-floor extraction paths and independently declared
semantic and resource limits. Before projection, the producer and watchdog
independently verify every floor-entry identity-to-exhaustion-receipt binding,
then canonicalize nonphysical source arrival order by semantic coordinate
identity before projection. Each path constructs a private candidate manifest
and a separate canonical ancestry manifest. Those manifests are checker
evidence, not an admission surface. Success requires agreement over the
candidates and ancestry manifests followed by paired final-projection replay
through both independent paths.

The v5 success receipt binds the claim, canonical semantic input, floor and
identity-to-receipt bindings, both result digests, both ancestry-manifest
digests, both semantic and resource contract digests, the paired final
projection, runtime canary digest, checker identities, membership-neutral
counts, and receipt digest. The v5 refusal
receipt binds the refusal claim, checked input, independent outcomes, declared
limits, checker identities, canary evidence, reason code, and receipt digest.
A refusal mints no root capability. Production refusal verification also
replays the current sealed pair and requires the fresh refusal to exact-match;
structural self-digest consistency alone is not current evidence.
Pre-projection runtime canaries send source-order permutations, stale
identity-to-receipt bindings, changed limits, and other packet mutations
through both sealed projectors. Every v6 packet-canary transcript entry binds
the complete mutant preimage and observed checker result. Source-order
permutations preserve the semantic result.
Post-projection runtime canaries send omission, duplication, scope escalation,
canonical-byte mutation, admission-receipt substitution, and coordinated
ancestry substitution through both real final verifiers. A separate paired
transcript binds every mutated projection field, receipt, admission,
capability, and observed verifier result. It excludes only its own digest field
to avoid self-reference. Before accepting a projection, each final verifier independently
re-extracts and reprojects its current sealed packet, exact-matches the private
manifest and canonical bytes, reconnects both receipt resource digests to its
local contract, and recomputes both canary transcripts. Focused tests
separately substitute every success and refusal receipt field, exercise
checker disagreement, prove that same-code mutants produce different
transcripts, and prove that a self-consistent unsealed projection or
self-authored refusal cannot escape. None of these uses a parallel mock
acceptance path. The generic
alien-coordinate projector exists only under `cfg(test)`, returns checker
output only, and grants no admission capability. It tests the admit-the-alien
invariant without creating an authored-coordinate authority. The authority
inventory pins this mechanism's semantic profile as SHA-256
`7963fbd5fb857fb2516e11579e76409c5bdfc3cc53d172b15251016b87d80750`.

The current projection-to-registry boundary now performs that complete paired
verification. An admitted artifact cannot be constructed in production from a
schema token and arbitrary receipt digests. It carries a private opaque
`VerifiedAdmissionCapability`; the repository-root pair is the only production
minter. That capability binds the artifact identity, exact root admission, and
pair-receipt digest. Both physical-registry validators independently reject
identity, admission, capability, or receipt drift. Test-only exact capabilities
exercise unfamiliar, derived, irreducible, and massless graphs without
creating a production mint.

The v5 physical-registry producer and v4 watchdog each own a complete local
production cap table, resource-contract constructor, semantic-work schedule,
and digest encoder. The resource digest covers both the caps and semantic work
profile. Canonical rational bytes, expression nodes and edges, executed
operations, identities, hashes, registry emission, and bounded powers carry
implementation-neutral costs. Complete closure is preflighted as
`2V + 2E + D`, where `V` is rule count, `E` constituent-edge count, and `D`
declared-member count. Producer then uses linear indegree elimination while the
watchdog retains DFS, reverse tracing, and a forward worklist. Algorithm-local
helper visits do not consume the semantic budget. Exact-boundary and
resource-profile substitutions must therefore produce the same acceptance or
refusal on both sides. The current pair computes, validates, and compares those
digests but does not mint a production species-authority receipt. Before any
species registry activates, its production receipt must retain both local
resource digests and bind separately executed full-registry canary transcripts.

A distinct future non-root mint and registry-to-support boundary remain
blocked. Before a species-forming rule may authorize a derived or irreducible
artifact, a claim-specific independent pair must verify its complete ancestry
and mint the matching capability. Both physical registry validators must then
revalidate attribution, closure, and membership scope. The species derivation
frontier is recorded as a non-authoritative diagnostic because its two
traversals share repository-authored frontier material.

The scientific recursion bottoms at explicit authority boundaries. A human
owner decides whether a declared physical variable set is complete, whether
evidence lineages are independent, and whether a Gap or Residual argument is
sufficient. The watchdog can bind that decision and detect drift. It cannot
prove the human judgment that created it. The inventory's structural recursion
bottoms at exact agreement between its schema-first producer and profile-first
watchdog.

The physical floor is the first active irreducible instance of this rule.
Catalog-admission v3 separately binds derivation exhaustion, Buckingham Pi,
Gap Law, the typed Chaos Protocol branch, Residual Law, and the unique residual
slot for every leaf. An identity-keyed owner-admission record commits that
complete tuple, and the independent byte parser reconstructs it before
physical-floor authority v5 can exist. The repository pins are durable,
claim-scoped review capabilities, not cryptographic proof of the reviewer's
identity.

`CODEOWNERS` names the registry, implementations, rule, and gate registry as
one review surface. It becomes an enforced human boundary only when the host
requires pull requests, owner review, authority status checks, and the chosen
signature policy. Repository-local checkers cannot attest their own hosting
configuration.

## Inventory meta-pair

The schema-first producer rejects any top-level or per-profile field outside
the schema, then validates each row against a separately pinned profile map.
The profile-first watchdog walks an ordered tuple of required profiles and
requires each registry row to match at the same position before applying its
own field and path checks. Neither imports code, constants, or acceptance
booleans from the other.

Both receipts bind the registry SHA-256, schema and closed-world marker,
ordered mechanism profiles with their complete-profile SHA-256 pins, exact
active, blocked, and diagnostic counts, and a sorted semantic-closure
path-to-SHA-256 manifest. The current closed profile contains nine active
authorities, eight blocked authorities, and one diagnostic. The agreement
receipt therefore changes when a semantic field changes, when any closure
implementation, adapter, shared primitive, or orchestrator changes, or when a
structural classification changes. Agreement remains structural. It does not
certify the scientific claim made by an enrolled mechanism and does not turn a
diagnostic into authority.

## First enrolled implementation

The floor Pi producer in `crates/units/src/dimensional_analysis.rs` uses exact
rational RREF. Its cross-checker in
`crates/units/src/authority_watchdog.rs` uses fraction-free integer
elimination and a direct integer basis check. Their sealed receipt binds the
ordered SI matrix, phenomenon membership, residual slots, ranks, nullity,
declared budget, producer basis, and both algorithm identities. The physical
floor authority digest includes the watchdog seal.

The dense producer analysis also refuses before its worst-case augmented
null-space basis exceeds 1,048,576 cells. That checked execution-resource
budget does not close the physical basis or forbid unfamiliar coordinates.
A larger lawful census needs a partitioned or sparse authority path with its
own bound rather than an allocator-dependent result.

Mutation canaries cover a changed dimension exponent, raised budget, omitted
variable, duplicate variable, reordered variable set, changed phenomenon, and
duplicate residual slot. Any such change refuses until the independently pinned
authority is reviewed and updated.

## Activation checklist

Before an authority-bearing mechanism can move from `blocked` to `active`:

1. Name one narrow semantic claim.
2. Define canonical input bytes without an expected answer field.
3. Implement the producer and cross-checker through distinct semantic paths.
4. List every shared primitive and prove that no shared semantic helper exists.
5. Bind results, implementation identities, inputs, and canaries in a receipt.
   When the claim relies on executable canaries, bind their
   production-executed transcript identities, case counts, and digests rather
   than citing test names alone.
6. Add adversarial canaries that would fool one side or change the claim.
7. Record the human authority boundary and remaining falsifiers.
8. List every implementation, adapter, shared primitive implementation, and
   orchestrator that can change the scoped meaning in `semantic_closure`.
9. Enroll the pair in `scripts/authority_watchdog.toml`.

The rule is informed by independent verification and validation practice, but
the repository applies it at mechanism granularity. Organizational, financial,
and personnel independence are outside what one repository can manufacture.
