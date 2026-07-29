# The mid-band arc: audit packet (implementation verbatim, anomalies enumerated)

Prepared for audit and review at the owner's request. Branch `claude/seam4-deeptime`, HEAD `1c511ba`. Both byte pins hold on EVERY commit reported here: default `40fe8a7269ee4da8974eb1787338c3a0`, living `be94e3100b9db82f7c1aea1d8091956d`.

Section A is what was built, with the load-bearing code VERBATIM rather than described, per the packet-fidelity rule (a paraphrase makes a panel find phantoms). Section B is every anomaly, including the ones in my own work, which are listed first because they are the ones a self-report is most likely to omit.

---

## Section A: the implementation

### A1. `laws::convective_strain_rate` (slice 2, commit `15fff5f`, mine)

The claim: `convective_stress` computes `tau = eta * |v| / L` and returns only the stress. For a Newtonian fluid `tau = eta * eps_dot`, so `eps_dot = |v| / L` is a quantity that law forms on every call and discards. The lid's DRIVING STRESS and the lid's STRENGTH must be evaluated against ONE strain rate or they are two carriers of one physical fact.

The code, verbatim:

```rust
pub fn convective_strain_rate(velocity: Fixed, length_scale: Fixed) -> Option<Fixed> {
    if length_scale <= ZERO {
        return None;
    }
    sat_abs(velocity).checked_div(length_scale)
}
```

Two deliberate asymmetries with its sibling, each stated at the site:

1. FAIL-LOUD where `convective_stress` CLAMPS. A stress past the representable range reads as overwhelmingly strong; a saturated STRAIN RATE does not read as "very fast" to its consumer, which takes its logarithm and sets the result beside an Arrhenius exponential, where it multiplies through an `exp` and returns a confident wrong strength.
2. `[direct]` in the floor registry (no `[[law]]` block), matching the `thermal_boundary_layer` EXTRACTION precedent rather than its declared sibling. The difference is honest: `convective_stress` has a live caller; this is dormant until the yield-envelope consumer lands.

### A2. The binding test, and its DERIVED residue (the coherence protocol's step one)

`convective_stress` keeps its own association `(eta * |v|) / L` UNTOUCHED, so no bytes move. The two are held together by a test whose tolerance is derived from the representation, never chosen:

Both fixed-point ops truncate (`checked_mul` through `>> FRAC_BITS`, `checked_div` through integer division), so with every error in `[0, u)` and `u = Fixed::EPSILON`:

```
path A (the stress's own order):  fl(fl(E*V)/L) = T - d1/L - d2
path B (this law, then scaled):   fl(E*fl(V/L)) = T - E*e1 - e2
                     therefore:   |A - B| < u * (E + 1/L + 2)
```

The bound in bits, verbatim (the ceiling is the derivation's own: a bound must not round down to below itself, and it is the only rounding in the expression):

```rust
let sum_bits = eta.to_bits() as i128 + one_over_l.to_bits() as i128;
let one_bit: i128 = 1 << Fixed::FRAC_BITS;
let bound = u * (((sum_bits + one_bit - 1) >> Fixed::FRAC_BITS) + 2);
let gap = (tau.to_bits() as i128 - scaled.to_bits() as i128).abs();
assert!(gap <= bound, ...);
```

MUTATION RESULTS: four killed (wrong operator `v*L`, dead return, dropped magnitude, 2x scale). ONE SURVIVES BY CONSTRUCTION and is stated in the blindness set rather than chased: a 1-ULP error is indistinguishable from the reassociation the test licenses, because THE BOUND IS THE RESIDUE. That is the price of the byte-neutral door and the argument for step two.

### A3. The `@provides` gate (commits `5f4ab3f`, `147b16f`, mine)

The defect it closes, in the gate's own prior words: `law_providers` found "a DERIVING provider of the quantity ITS NAME STATES". THE NAME WAS THE QUANTITY, so two providers under two names were invisible BY CONSTRUCTION. That is the blindness the gate's scorecard already confessed ("BLIND to DIFFERENT-NAMED PROVIDERS ... covered by the census-before-build habit, NOT mechanically").

The annotation binds to the NEXT `fn`; any non-doc line between them breaks the binding, because an annotation that could drift onto a function it does not describe would be worse than none. Inside `laws.rs` an UNANNOTATED `pub fn` still provides the quantity its name states (the gate's original and only inference); OUTSIDE `laws.rs` silence means silence, because there that guess would be noise.

What it finds today, mechanically, having been told only where to look:

```
UNARBITRATED TWIN PROVIDERS: `log_sum_exp`
     +-- [DERIVED]  logsumexp_canonical()  crates/materials/src/creep.rs:212
     +-- [DERIVED]  log_sum_exp()          crates/physics/src/saha.rs:117
```

That is the diamond a human census found by hand while grounding the ductile composite. It STAYS FIRING, correctly: it is a real unarbitrated diamond scheduled for re-pin window one, not registered away.

REMAINING BLINDNESS, stated: undeclared twins stay invisible, since the annotation is hand-authored. This MECHANIZES the census habit; it does not replace it.

### A4. The hook fail-open (commit `17ebad3`, mine)

Three hooks passed the tool payload to Python in an ENVIRONMENT VARIABLE. An env var is capped at MAX_ARG_STRLEN (131072 bytes), so any payload past ~128 KB failed the exec with E2BIG before a line of Python ran, exiting 126. A PreToolUse hook blocks ONLY on exit 2, so a non-2 exit is a NON-BLOCKING error: the guard vanished.

Measured on `customs-guard.sh`, which documents itself as the hard guard that "blocks even under bypass mode":

| payload | verdict before | verdict after |
| --- | --- | --- |
| 147 bytes + em dash | exit 2, blocked | exit 2, blocked |
| 25 KB + em dash | exit 2, blocked | exit 2, blocked |
| 200 KB + em dash | exit 126, FAILED OPEN | exit 2, blocked |
| 1 MB + em dash | (not reached) | exit 2, blocked |
| 1 MB clean | (not reached) | exit 0, no false block |

The env var was never needed. The original rationale, in the guard's own header, was that "piping the JSON straight into a heredoc would feed the script to Python's stdin instead of the payload". TRUE OF A HEREDOC, false of `-c`: with `-c` the script rides in argv and stdin was free the whole time. The reasoning was right about the mechanism it named and applied to one it did not.

HONEST SEVERITY: the guard reads only `docs/design.md` and `docs/audit.md` (line 44), so the fail-open needed a >128 KB single edit, to one of those two files, carrying a violation. `verify.sh` covers it downstream. Both documents are clean. THE GUARD WAS NEVER BEATEN.

### A5. Slice 3, `moment_equivalence.rs` (commit `1c511ba`, built by an isolated agent, gated by me)

Canonical output is `D_eq`, NOT `T_e`, which executes the rigidity ruling further than it was written. A deflection constrains a RIGIDITY; nothing in a deflection knows what `E` the plate has. `T_e` is that `D` re-expressed through an ASSUMED pair (the primary's Table 1 prints E=80 GPa, nu=0.25 under "Assumed values"), and since `T_e ~ (1/E)^(1/3)`, an engine deriving its own modulus and comparing `T_e` to a published `T_e` compares its plate against a FICTITIOUS 80-GPa one.

Tests pinned from OUTSIDE (the primaries' own printed numbers): the purely elastic plate returns `T_e = H = 40 km` exactly, which is the only test pinning the plane-strain `1/(1-nu^2)` whose loss is a 2 percent error that reads as quadrature noise; the elastic-plastic illustration returns 36.79 km against their printed "less than 37 km"; the Mohr-Coulomb resolution reproduces their printed eqs. 7 and 8 and EXCLUDES the 50 MPa cohesion by 100x the tolerance. Mutation: 12/12 killed, two of them real gaps closed rather than explained.

---

## Section B: the anomalies

### B0. In my own work (listed first, because a self-report omits these)

**B0.1 An AUTHORED CONSTANT inside a "derived" bound.** The pushed binding test computed `floor(E) + floor(1/L) + 4`: flooring understates, so I had added a hand-chosen `+2` to cover it. A fudge factor inside a bound whose entire claim is that it is derived. Pins, clippy, stone0, and the tests all passed it green. NOTHING MECHANICAL WOULD HAVE CAUGHT IT: the number was in a test, where no registry or provenance gate looks. Caught only while writing it up for audit. Fixed by ceiling the exact sum (`dd4e234`); the tightened bound kills every mutant the loose one did.

**B0.2 A vacuous binding test.** Mutation showed it SURVIVED a mutant that dropped the magnitude and returned a SIGNED rate, because every fixture I chose had a rising flow. The two paths agree for `v > 0` whether or not the abs is there, so the binding was blind to exactly the convention it exists to bind, and a signed rate breaks `tau = eta * eps_dot` for every SINKING parcel, which is half the convection cells in any world.

**B0.3 Three narrow-grep failures, mine.** (a) I reported "no convective velocity exists" while `laws::stokes_velocity` SAT IN MY OWN GREP OUTPUT, named for its physics rather than its role. (b) I counted 149 broken doc links where the lint emits 151, missing the two findings whose wording was unusual, which is the population a grep is least able to enumerate. (c) I "verified" a registry regeneration with `git diff | grep -E "^[+-]" | grep -v "^[+-][+-]"`; the registry is a MARKDOWN LIST, so every changed line diffs as `-- \`foo\``, and my filter stripped every changed line, reporting a clean no-op over a real change. My "fixed" filter `^(\+|-)[^+-]` FAILED IDENTICALLY. `git diff --exit-code` said 1 the entire time.

**B0.4 I damaged the enforced reference while wiring the gate that protects it.** The floor registry takes a kernel's FIRST doc line as its excerpt; I placed `@provides` first, so the derive-vs-author reference stopped describing two kernels and recited its own annotation back. Caught by the stop gate.

**B0.5 Two lying counts and a lying render, in the cross-crate sweep.** My regex loosened `^pub fn` to allow leading whitespace and optional `pub`, sweeping test functions and impl methods into the kernel count (116 quantities became 223 from two annotations). The render hardcoded `laws::`, printing `laws::logsumexp_canonical()` for a function in `crates/materials`. The headline printed `len(laws)` under the label "laws.rs kernels" once the sweep went cross-crate. Two of the three would have INFLATED the gate's apparent reach, which is the direction that flatters. All caught by reading the numbers rather than the happy path.

**B0.6 A conflict resolution that silently un-ran two tests.** Merging #194 over #192, my reconstruction dropped `#[test]` from two of #194's tests. A function-set comparison against the union of both parents caught it independently of clippy: 36 expected, 36 present, exactly two attributes lost. A silently un-run test passes every check that is not looking for it.

### B1. The phantom E_coh ladder (refuted three ways, any one sufficient)

The registered arbitration claimed "only Rose's EOS route provides E_coh". Rose never provided it. `MetallicRoute::cohesive_energy` has READ the measured atomization column since its birth commit (its body is an anchor-PRESENCE check whose value is discarded, then `table.element(symbol).atomization_enthalpy`), and `metallic.rs`'s own doc says so: "the metallic cohesive energy is the banked atomization enthalpy", and decisively, "at its equilibrium volume the Rose EOS reproduces E_coh BY CONSTRUCTION (the depth of the binding well)". Rose CONSUMES E_coh.

Second: the declared overlap set (elemental metals) does not exist as phases in `phase_registry.toml`, which carries only oxides and silicates. Third: an element's `delta_f_H` is zero in its reference state, so the Hess route returns the column ITSELF for an element, and the sentinel would have compared the column against its own reflection and agreed to the bit forever.

It was billed as "the fetch's own referee ... so the fetch is refereed THE DAY IT ARRIVES". It could not have caught the one defect the fetch had: the Fe row cited to a CODATA table containing no iron row, caught by a human census.

### B2. `ductile_strength_mpa` did not exist

`creep_rows.rs` listed "THE FIVE CONDITIONS, each realized here" and condition 4 named `ductile_strength_mpa` as its realization. One grep hit across all of `crates/`: that doc line. A rustdoc link to a function that was never written.

### B3. The logsumexp diamond

Two implementations of one primitive, two names, two crates, BOTH invoking this project's determinism discipline by name (saha's "the canonical-logsumexp determinism rule", creep's "rider 1c, the fixed-topology-reduction discipline"). They cannot agree bit-for-bit: a fold of pairwise ops rounds at every step; a sorted n-ary reduction rounds once. Now mechanically visible (A3), still unarbitrated, scheduled for window one.

### B4. Two of the owner's construction specifics, voted down by the primaries

Dual-channel from McNutt and Menard 1982 and Watts and Burov 2003, which reproduce each other. The CORE verified verbatim. Two specifics did not:

- `T_mech` as ruled (the brittle-ductile crossing) is not the literature's `T_mech` (the depth where yield strength falls below 50 MPa, "corresponds to Q/RT = 60", an authored threshold the paper states outright). The crossing is the SEISMOGENIC base and lies shallower. RULED: the engine ships no `T_mech`; the crossing becomes `z_BDT`.
- Curvature reads at the FIRST ZERO CROSSING, not the peak: `w(x_0) = 0` kills the axial term, and the two plate models AGREE there while trench-wall curvature differs between them by a FACTOR OF TWO. It is the one point insensitive to the rheology being assumed, which a construction deriving `T_e` FROM rheology cannot afford its input to depend on.

Trophy for the dual-channel mandate: OCR rendered the flexural rigidity as `D = E T2/12(1- v')`, destroying BOTH the exponent 3 and the `nu^2`. Only the visual channel had the truth, in the arc's central equation.

### B5. THE SELF-TRUNCATION PREMISE IS FALSE (the newest, and the ruling's own physical claim)

The ruling: "the moment integral needs no hard floor: ductile strength decays EXPONENTIALLY with depth through the geotherm, so the integrand's tail is bounded and the integration self-truncates."

A creep envelope does not decay exponentially. A POWER-LAW ROW HAS A STRENGTH FLOOR: `sigma -> (eps_dot/A)^(1/n)` as T rises, about 2 Pa for the banked dry-olivine row at 1e-15/s. Never zero. The lever arm grows LINEARLY, so the integrand tends to a linearly GROWING function and THE INTEGRAL DIVERGES. Under `halfspace_geotherm`, whose T saturates at the interior, the `P*V*` term makes deep material STRONGER with depth: the integrand turns and CLIMBS, with ~13 percent of M in the 200-300 km tail and rising. McNutt and Menard's own integral converges only because THEIR GEOTHERM IS LINEAR, and therefore unphysically hot at depth.

The slice's answer: truncate where the integrand DOES die, and REPORT `self_truncated=false` where it does not, making the moment's dependence on the caller's declared lid VISIBLE rather than silently absorbed. It did NOT reach for `laws::thermal_boundary_layer`, which derives a lid one call away, because "reaching for the nearest available depth is the defect that evicted the nearest available rate".

CONSEQUENCE FOR THE ARC'S CLAIM: "nothing in the arc authors a scalar" now carries an asterisk. Where the integrand does not die, the moment depends on a DECLARED LID, and that dependence is reported rather than hidden. This is the owner's to rule.

### B6. The arc's own primary hindcast target is unserved

The CIRCULAR LOAD IS REFUSED, not faked: `M = -DK` is the line-load form, the axisymmetric case carries `nu/r` against `1/r`, and its fibre state is biaxial (it needs a 2-D yield surface). The arc's stated primary hindcast row is OCEANIC INTRAPLATE VOLCANIC LOADS, that is SEAMOUNTS, which are axisymmetric. Named rather than approximated.

### B7. The banked `V*` chords start at 0.3 GPa

About 9 km on Earth, so a `LithosphereEnvelope` cannot be sampled FROM THE SURFACE without a determination whose chord reaches it. This blocks the full-column solve on a real envelope.

### B8. A sentence pointing the next builder at the forbidden plumbing

`creep_rows.rs` read: "THE RATE IS THE LOAD'S OWN (condition 4) ... so this is derived from THE WORLD'S CONVECTIVE TIMESCALE by the caller." The right rule and the wrong rate, in ONE sentence, in the doc that forbids it. Found by the slice that CONSUMES the function, which is the reader most able to be misled by it. Corrected at `1c511ba`.

### B9. Byerlee gains a third reading

McNutt and Menard cite "Byerlee 1968, 1978" for a law whose cohesion is 80 MPa, where Byerlee 1978's high-stress branch, verified from the primary in `GEOTHERM_FETCHES` section 4.3, is 50 MPa. The friction 0.6 agrees; the cohesion is 60 percent higher and unexplained. Independently corroborated by slice 3's Mohr-Coulomb referee, which excludes 50 MPa by 100x its tolerance against the primary's own printed equations. Queued: Byerlee 1968, with the COORDINATE-FRAME hypothesis named for the fetch to TEST rather than assume.

### B10. The customs guard never watched the board

`customs-guard.sh:44` reads only `docs/design.md` and `docs/audit.md`, so `CONSENSUS_ROADMAP.md`, the document that tracks this project's discipline, has never been mechanically guarded. Eight banned adverbs had accumulated in it. Fixed at `5544b44`, each rewritten to keep its sentence's claim.

---

## Section C: what is owed, and by whom

OWNER: whether "nothing in the arc authors a scalar" survives B5's asterisk; the seamount path (B6: build the axisymmetric form with its 2-D yield surface, or re-target the hindcast); the `V*` chord that reaches the surface (B7). Re-pin window one's signature when slice 5 lands.

WORK, NOT RULINGS: the axisymmetric load; the hindcast rows in rigidity space with their (E, nu) and load-class fields; slice 5's flexure wiring; the logsumexp binding test before window one; the cross-firewall tooling name check; the Miedema silicate reach behind the atomization-column referee.

FETCHES QUEUED: Byerlee 1968 (frame hypothesis to test); the Hartmann or Manara rate-versus-age locus for Agent C's `t_mature`.
