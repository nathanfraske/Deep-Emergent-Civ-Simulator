# Q1 Stone 1: the sealed `Fixed` constructor, grounded design opener

This is the design-first opener for Q1 Stone 1 (the seal-the-constructor mini-arc of `Q1_ARCHITECTURE_BUILD_PLAN.md`), the owner's directed next architecture arc. Its purpose is to convert derive-do-not-author from a discipline the builder must remember into a rule the compiler and gates enforce, so a forgetful builder cannot author a world-content value inline where a floor read belongs. No code lands until the gate gates this design; this document grounds the current `Fixed` construction reality at source, surfaces the migration scope (which is much larger than the plan assumed), and puts the one enforcement-model question to the gate.

## What the plan says Stone 1 is

Ship-order item (1) of the build plan: "add the sealed `Fixed` constructor plus the constants quarantine; add a LINT flagging raw construction outside quarantine (advisory); inventory the violation set; migrate site by site with each byte-diff reviewed; FLIP the seal to hard once the count hits zero." Fork-independent (it works under either base-dimension option, and Fable's round-3 consult already resolved that fork by build-time codegen, so it does not gate Stone 1). The determinism grep gate, ship-order item (0), is the cheap total-coverage win to land first or in parallel.

## Grounding: the `Fixed` constructors at source (`crates/core/src/fixed.rs`)

`Fixed` is Q32.32. The vectors that fabricate a `Fixed` from a bare number:
- `from_bits(i64)` (line 63): the raw internal representation. Used inside `fixed.rs` itself for the pinned transcendental constants (`LN2`, `PI_BITS`, the CORDIC tables), and almost nowhere else legitimately.
- `from_int(i32)` (line 75): a whole number. The overwhelmingly common vector.
- `from_ratio(num, den)` (line 87): a rational. The common vector for a fractional value.
- `from_decimal_str(s)` (line 96): parses a decimal string to fixed-point by integer arithmetic. The vector the floor and manifest loaders use to read cited data.
- `from_bits_i128` (line 254) and `sum_bits`: internal wide-accumulator plumbing.

## The construction-site inventory (the finding that reshapes the plan)

Counts of each vector in the determinism-critical crate sources (`src/`, which includes inline `#[cfg(test)]` modules):

| crate | from_int | from_ratio | from_decimal_str | from_bits |
| --- | --- | --- | --- | --- |
| core | 35 | 12 | 0 | 44 |
| physics | 337 | 148 | 16 | 7 |
| units | 1 | 10 | 16 | 3 |
| sim | 1458 | 1218 | 17 | 50 |
| world | 15 | 9 | 0 | 0 |

The plan's Stone 1 assumed the "@derives" markers were "a handful" and implied the violation set would be small. The inventory says otherwise: the `from_int` plus `from_ratio` population in the sealed crates is in the low thousands. The decisive finding, verified by sampling the sites: the vast majority are NOT authored world-content values, they are the documented Principle-11 ENGINE-MECHANICS EXEMPTION. `crates/physics/src/laws.rs` states it in its own header ("The only module constants are engine mechanics, not owner realism values: the unit-bridge ratios fixed by the pinned canonical scales, the overflow-safe saturation ratios forced by Q32.32, and the mathematical constants"), and the sampled sites bear it out: `from_ratio(38, 100)` is the Watson corresponding-states exponent, `from_ratio(2, 3)` and `from_ratio(1, 3)` are cube-root exponents, `from_ratio(1312, 1000)` is the universal Tee-Gotoh-Stewart constant, `from_ratio(3, 4)` is the near-critical accuracy boundary, `from_int(5)` and `from_int(2)` are divisors and coefficients in a kernel. These are engine mechanics, exempt by Principle 11, not defects.

So the naive seal (make the bare constructor `pub(crate)`, force all construction through a quarantine, migrate the violation set to zero, flip to hard) collides with reality: there is no single quarantine that a divisor-by-two inside a kernel can move to, and there are thousands of such legitimate sites. The constructor ALONE cannot distinguish an authored world-content value from a legitimate engine-mechanics constant, because both are `Fixed::from_ratio(a, b)`. That distinction is the whole problem, and it is semantic, not syntactic.

## The enforceable boundary: what the seal must separate

The rule to mechanize is Principle 11 as sharpened by Prime Directive 6: a value in the PATH OF WORLD CONTENT must be DERIVED from the floor and the situation, or READ as world data (the manifest, the physics registry, the periodic table), never authored inline; the ONLY authored place is the physics floor. Engine mechanics (a divisor, an exponent, a unit-bridge ratio, an overflow cap, a math constant) are exempt and legitimate anywhere.

The seal must therefore separate two populations that share one constructor:
- LEGITIMATE: engine-mechanics constants (the Principle-11 exemption), and the floor and manifest loaders that parse cited data (`from_decimal_str` inside `units`, the physics registry loader, the periodic-table loader), and `fixed.rs`'s own pinned transcendental table.
- DEFECT: a reserved or world-content value (a threshold, a rate, a weight, a temperature) authored inline in a sealed crate where a floor read or a derivation belongs.

Because the split is semantic, no purely syntactic seal separates them exactly. This is the design question below.

## The design question for the gate: which enforcement model

Three models, in increasing strength and cost.

- Model D (scoped seal, tractable now): seal HARD only the vectors that are never legitimate outside a known quarantine. `from_bits` outside `core` (the raw representation; legitimate only for `fixed.rs`'s pinned transcendentals) and `from_decimal_str` outside the whitelisted loaders (`units`, the physics registry, the periodic table; the only places that parse cited floor data) are both clean, small, high-signal seals. Leave `from_int` and `from_ratio` to an ADVISORY lint (they are dominated by legitimate mechanics). Cost: catches the clearest inline-authoring vectors and the raw-bits and stray-decimal-parse defects; does NOT catch an inline `from_ratio(1, 10)` reserved value, so it is a partial guarantee, plainly labelled.

- Model C (typed mechanics-vs-value distinction, the plan's intent, large): introduce a distinct, self-documenting constructor for engine mechanics (a `Fixed::mechanics(n)` or a reviewed marker) and require every legitimate bare construction in a sealed crate to use it, so a bare `from_int` / `from_ratio` becomes the sealed defect vector that a compile error or a hard lint forbids; a world-content value must then come from a floor read. The migration is mechanical (each engine-mechanics site gains the marker, each is byte-neutral) but LARGE (the low thousands of sites), and it is the strongest form: it makes authoring-inline unrepresentable, "the derivation-hunter as a type rule". Cost: a multi-slice migration touching thousands of sites, each reviewed, before the flip.

- Model H (hybrid, my recommendation): land Model D's scoped hard seals first (small, byte-neutral, immediate), land the determinism grep gate (item 0) beside it (cheap, total, protects the reproducibility baseline every byte-neutral claim rests on), and stand up Model C's `mechanics` marker plus an advisory lint over `from_int` / `from_ratio` in the sealed crates, migrating leaf-first and flipping to hard only when a crate's count reaches zero. This delivers a real guarantee immediately (the clearest vectors sealed, determinism locked) and sequences the large typed-distinction migration as reviewed advisory-then-blocking work, exactly the plan's own advisory-first shape, now scoped to the population the inventory found.

## The one owner scope decision this surfaces

The typed mechanics-vs-value migration (Model C, inside the hybrid) is a low-thousands-of-sites arc. That is the owner's to size: the strong compile-time guarantee is worth a large mechanical migration if he wants derive-do-not-author machine-enforced end to end, or the scoped seal (Model D) plus advisory lint may be the right stopping point if the clearest vectors sealed plus a reviewer's advisory signal is enough. I surface the scope so the choice is his, and recommend the hybrid so the immediate wins land regardless of where he draws the line on the full migration.

## What Stone 1 builds, on the gate's ruling

On the hybrid (subject to the gate):
1. The determinism grep gate (item 0): a CI check and hook that fail on `Instant::now`, `SystemTime`, thread-id reads, and unordered-container iteration in `core` / `physics` / `sim` / `world`. Confirm `core::rng` is already counter-based and order-free. A red test proves an intentional `Instant::now` in `sim` fails. Byte-neutral (a gate, no code path change).
2. The scoped hard seals (Model D): `from_bits` sealed outside `core`; `from_decimal_str` sealed outside the whitelisted loaders; a red test per seal proving a violation fails to compile or fails the gate. Byte-neutral (the legitimate sites are already inside the quarantine).
3. The `Fixed::mechanics` marker plus the advisory lint over bare `from_int` / `from_ratio` in the sealed crates, publishing the current per-crate violation distribution, with the leaf-first migrate-then-flip sequence scoped by the gate's ruling on Model C.

Each step compiles the tree, keeps the pins bit-exact where it is byte-neutral, and is independently valuable. A step that LIGHTS UP a latent nondeterminism (item 1) is the gate working, and its fix is a deliberate reviewed byte change, budgeted per the plan's relabel note.

## Discipline

Design-first: no code until the gate gates the enforcement model. The seal itself is byte-neutral by construction (it moves no value, it only forbids a future one); the determinism gate is byte-neutral unless it finds a real bug, in which case the fix is a reviewed byte change and a success. Section-9 once by me per slice (the cost directive). This is the machine form of the rule this project has corrected most often, and the volatile-curve arc just spent three rounds proving why the guarantee earns its place.
