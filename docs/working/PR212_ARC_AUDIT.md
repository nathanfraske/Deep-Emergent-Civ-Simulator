# Adversarial audit of the disk-evolution arc (PR #212)

A `gpt-5.6-sol` pass at maximum reasoning effort over the full diff of
`claude/abiotic-field-registry-lmqbqd` against `origin/main`: 407 lines of `astro.rs`, 549 of
`giants.rs`, the vendored registry entry and the working note. Briefed on seven lenses (fabrication,
derive-first, admit-the-alien, provenance, determinism, representation, checkable claims) and told
explicitly NOT to assume the arc's corrected EUV physics is sound merely because it caught an error in a
delegated fetch agent, but to re-derive it independently.

VERDICT: request changes. Six blockers, four high-severity findings.

The corrected `10^8` normalization IS sound and the audit reproduced all sixteen literals to better than
`5e-5 dex` from independently integrated SEDs. What it found instead are structural defects in the
disk-evolution state and a set of claims in comments and commit messages that are false.

TWO FINDINGS I VERIFIED MYSELF before relaying, because a wrong audit finding is worse than none:

  - THE DOI IS WRONG, and worse than the audit states. The branch cites `10.1086/511268`. Resolved
    against Crossref, that is "Mid-Infrared Galaxy Classification Based on Silicate Obscuration", a
    completely unrelated paper. The correct DOI for Lanz and Hubeny 2007, ApJS 169, 83 is
    `10.1086/511270`. The receipt points at the wrong work.
  - The "nearly linear at +0.14 dex per 1000 K" claim is overstated: adjacent slopes range 0.0537 to
    0.1884. I REPEATED that claim in my own PR comments without checking it, and used it to argue about
    interpolation risk across the two unwitnessed points. That amplification was mine.

The verbatim audit follows.

---

# Adversarial audit verdict

**Request changes.** The corrected BSTAR2006 `10^8` normalization is sound, but the PR is not merge-safe. The largest defects are structural: the “canonical” state does not constrain the giant verdict to the truncated gas reservoir, discards an existing 48× collapse band, and the claimed conservation sink records no transfer at all.

## Blockers

1. **The giant verdict ignores the truncated gas account.**

   [`derive_disk_evolution_state`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1635) computes `state.gas`, but calls `giant_formation_banded` with the original untruncated `disk`. Its runaway branch independently integrates the original local feeding zone at [`giants.rs:1148`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1148).

   Concrete failure: birth radius 30 AU, truncation at 10 AU, embryo at 20 AU. Nothing rejects the embryo or limits its feeding annulus to 10 AU; it can receive gas and become a giant even though `state.gas` says that region was removed. Truncation reaches the verdict only through `t_visc`, not through available mass.

   Therefore the claims that the gas account and verdict read “the SAME capped disc” and “one state” at [`giants.rs:1514`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1514), commit `d507b38`, and the roadmap are false.

2. **The new canonical path discards a banked 48.118× collapse interval.**

   The existing banded path explicitly evaluates both collapse models at [`giants.rs:1457`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1457), whose coefficients differ by `46.915 / 0.975 = 48.11795`. The new state instead selects the single Shu/default member at [`giants.rs:1605`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1605).

   For `γ=1`, the lifetime behaves as roughly

   `τ ∝ (Mdot0/Mdotwind)^(2/3) - 1`.

   At a representative Shu ratio `Mdot0/Mdotwind = 187.5`, retaining the Larson–Penston edge expands the high lifetime by about **13.61×**. Even asymptotically the omitted factor is `48.11795^(2/3) = 13.23`. This can change a terrestrial/giant verdict. It directly regresses the derive-first work documented at [`giants.rs:1421`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1421).

3. **The “named conservation sink” is bookkeeping theatre, not a transfer.**

   The dynamic path creates a local zero ledger, creates `removed`, then immediately destroys it at [`giants.rs:643`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:643). `ConservedLedger` discards both source and sink tags—the arguments are `_source` and `_sink` at [`conservation.rs:203`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/foundation/src/conservation.rs:203)—and stores only net and move count.

   No mass or momentum is added to a companion, circumbinary reservoir, or inward disc. `+removed - removed == 0` merely proves the local tautology the function constructed. `system_gas_budget_earth` is then assigned the claimed total without checking any recipient.

   The following claims are false:

   - “the ledger records the outflow under” the tag at [`giants.rs:547`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:547);
   - “a dropped residual is a compile-flagged leak” at [`giants.rs:582`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:582);
   - “conserved to a named sink” in commit `22707a8`.

   The public ledger is also forgeable. For example, `retained=10`, `removed=-5`, `total=999` is accepted and returns `residual_conserved=true`, `system_gas_budget=999`.

4. **The BSTAR departure is applied without the atmosphere coordinates that define it.**

   The table is strictly solar metallicity and `log g=4`, but [`windless_herbig_departed_spectrum`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:2787) sees only `T_eff` and a blackbody branch. It cannot inspect metallicity, gravity, mass loss, luminosity, or wind regime.

   Consequently, any 20 kK star—metal-free, twice-solar, `log g=2`, or strong-wind—gets exactly `log10 departure=-2.2815`. BSTAR2006 itself provides six compositions and 13 gravities, demonstrating that these are model coordinates, not irrelevant metadata. [The primary BSTAR2006 record confirms that parameter coverage.](https://arxiv.org/abs/astro-ph/0611891)

   The archive grade records custody only; it does not carry atmosphere applicability. The documented wind selector is absent. “Herbig Be winds are weak, so windless is appropriate” at [`astro.rs:2682`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:2682) is unanchored.

5. **The diffuse/direct EUV branch is not built, and its zero-optical-depth result is backwards.**

   [`euv_dispersal_phase`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:4201) has no non-test consumer. No direct-field rate is implemented; the stored `8.8` ratio is unused. The arc added a dead enum and discriminator, not a working second photoevaporation branch.

   It also returns `None` for `τ=0`, although zero optical depth is the clearest possible direct-field case. The test at [`astro.rs:7351`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:7351) circularly locks in that error.

   The source criterion is not an arbitrary “inner optical depth”: it is the 13.6-eV optical depth **along the disc midplane to `0.2 Rg`**. The untyped scalar admits a vertical, Rosseland, or other-band optical depth. The paper says direct dominates when more than 1% of the flux arrives and defines this specific column criterion. [Alexander, Clarke & Pringle 2006, equations 14–15 and surrounding text.](https://academic.oup.com/mnras/article/369/1/229/1053343)

   The radial `R^-1/2`/`R^1/2` comment at [`astro.rs:4151`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:4151) also cites the wrong equation: Eq. 14 establishes the shared flux/mass scaling and the 8.8 comparison; the radial behavior comes from the earlier direct/diffuse forms.

6. **A representable gas ring fails from an overflowing intermediate, then an existing fallback can erase the entire reservoir.**

   At [`giants.rs:505`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:505), evaluation order is:

   `((2π × r) × Σ) × dr`.

   Triggering ring:

   - `r = 100 AU`
   - `Σ = 4,000,000 kg/m²`
   - `dr = 0.0001 AU`

   `2πrΣ = 2.513274123×10^9`, above Q32.32’s `2.147483648×10^9` ceiling, so the function returns `None`. But the completed product is only `251,327.412`, comfortably representable, corresponding to approximately **941.8 Earth masses** after the AU² conversion.

   Worse, the giant path converts any feeding-zone integration failure into zero gas with `.unwrap_or(Fixed::ZERO)` at [`giants.rs:1151`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1151). One overflowing ring can therefore drop the whole reservoir and return a core-only mass while claiming not to fabricate a value.

## High-severity findings

7. **The “one set of birth conditions” is several independent and potentially contradictory sets.**

   In [`derive_disk_evolution_state`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1564):

   - disc temperature uses `disk.thermal.accretion_rate_msun_myr`;
   - lifetime uses the separately derived stellar `Mdot0`;
   - temperature uses `disk.thermal.star_mass_ratio`;
   - viscous gravity and verdict use `star.mass_ratio`;
   - viscous time uses `disk.mean_molecular_weight`;
   - collapse uses `star.mean_molecular_weight`.

   The bundled test already supplies a concrete mismatch: `mirror_disk` carries `0.3 M☉/Myr`, while the 10 K Shu state derives about `1.5–1.55 M☉/Myr`, approximately **5.2× different**. A disk mass ratio of 2 combined with a star mass ratio of 1 likewise produces a hybrid state without refusal.

8. **The canonical path authors `R1` instead of reading the already-built derivation.**

   The repository explicitly derives birth radius through `R_c=j²/(GM)` and warns that independently drawing `R1` creates “two doors to one fact” at [`astro.rs:903`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:903). The new API nevertheless accepts a bare `birth_r1_au` at [`giants.rs:1568`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1568).

   Its three provenance flags are then hardcoded `true` at [`giants.rs:1666`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1666). They do not inspect whether `R1`, rotation, or core temperature were solar-pinned or derived. The promised later “flip to false” cannot be machine-checked through this API.

9. **The claimed provenance DAG does not exist.**

   `DiskEvolutionState` claims every field inherits the weakest input provenance, but the function accepts no provenance for most inputs and simply authors `DiskGasProvenance::ProxyBounds` at [`giants.rs:1649`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1649). Truncation, wind, collapse, stellar state, and verdict carry no unified lineage.

   The struct also does not contain the claimed surface-density normalization or `tau_disk`; compare the claim at [`giants.rs:1514`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1514) with its actual fields at [`giants.rs:1532`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:1532). `tau_low/high` are calculated and discarded into the verdict.

10. **Truncation quadrature has domain, discontinuity, cast, and quantization defects.**

   At [`giants.rs:485`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:485):

   - `truncation_radius < inner_au` is not rejected, despite the documented “disk-edge miss.” Example: `inner=.1`, `birth=30`, `trunc=.05`, `steps=128` succeeds with zero retained mass.
   - Whole midpoint rings are classified rather than splitting the crossing ring. With `steps=1`, `inner=.1`, `birth=30`, `trunc=12`, the sole midpoint is 15.05 AU, so retained mass is reported as zero even though 39.8% of the radial interval lies inside the cut.
   - At 128 steps, `dr=0.23359375 AU`; the effective cut can move by half a cell, **0.116796875 AU**.
   - `steps as i32` breaks for every `steps ≥ 2^31`. At exactly `2,147,483,648`, the denominator becomes `-2,147,483,648`.
   - Division truncates. For `steps=10^9`, any positive span below `steps × 2^-32 = 0.232830644 AU` gives `dr=0`, losing the entire interval.
   - Each ring is converted to Q32.32 Earth masses before summation. Up to half an ulp per ring can vanish. At 128 rings, as much as `1.49×10^-8 M⊕`, about `8.9×10^16 kg`, can disappear; at `10^9` rings the bound is `0.1164 M⊕`.

## Corrected BSTAR2006 derivation

The corrected `10^8` normalization is valid for the stated 911.28 Å integration.

For SVO’s `Fλ` in `erg cm^-2 s^-1 Å^-1`, the dimensionally correct expression is:

`Nmodel = ∫ Fλ(λÅ) × (λÅ × 10^-8)/(hc) dλÅ`.

Only the wavelength used in the photon energy is converted to centimetres; `dλ` remains in Å because the supplied flux density is per Å. Converting `dλ` as well without first converting `Fλ` to per centimetre produces exactly the erroneous `10^-8` factor.

The matching physical blackbody reference is:

`Nbb = π ∫ Bν(T)/(hν) dν`.

The independent bolometric check gives `∫Fλdλ/(σT⁴) = 0.999443…1.000174`, confirming that the SVO tables contain physical surface flux rather than Eddington flux.

Representative independent results:

| Teff | `Nmodel`, photons cm⁻² s⁻¹ | exact-Planck `Nbb` | log10 departure |
|---:|---:|---:|---:|
| 15,000 K | `1.568445896e18` | `7.667310606e20` | `-2.689173515` |
| 20,000 K | `7.905676311e19` | `1.511548583e22` | `-2.281483082` |
| 25,000 K | `2.754836806e21` | `9.748700009e22` | `-1.548850830` |
| 30,000 K | `7.908796233e22` | `3.565948004e23` | `-0.654064620` |

All 16 code literals agree with the independently integrated SEDs to the expected four-decimal rounding, below `5×10^-5 dex`.

However, the derivation and runtime reference are not actually identical:

- [`BSTAR2006_HERBIG_EUV_DEPARTURE.md:57`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/docs/working/BSTAR2006_HERBIG_EUV_DEPARTURE.md:57) uses 911.28 Å and calls it “the same threshold” as the code.
- The only concrete code fixture uses `Tion=157821 K` at [`astro.rs:6777`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:6777), corresponding to approximately **911.65 Å**.
- The actual hydrogen ionization energy is `13.598434599702 eV`, approximately **911.75 Å**. [NIST hydrogen data.](https://physics.nist.gov/cgi-bin/Elements/elInfo.pl?element=1)
- The note’s statement that `13.6057 eV` corresponds to 911.75 Å is false; it corresponds to about **911.27 Å**.
- The blackbody denominator used for the table is full Planck, while runtime uses a one-term Wien approximation. Exact/Wien differs by `0.00122%` at 15 kK and **0.216%** at 30 kK.
- Combining the threshold and denominator mismatches makes the wired result underestimate a runtime-consistent model rate by approximately **0.25–0.29%**, up to **0.00126 dex**.
- More seriously, `IonizingSpectrumEvaluation` does not carry its threshold at all, so the fixed table can be applied to a blackbody constructed with any caller-selected `Tion`.

The note’s `(π/4)Bν Eddington form` statement at [`BSTAR…md:62`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/docs/working/BSTAR2006_HERBIG_EUV_DEPARTURE.md:62) is also false. `Hν=Bν/4`, while physical flux is `4πHν=πBν`; `(π/4)Bν` is neither the physical flux nor the Eddington flux used by the code.

Finally, “nearly linear at +0.14 dex/kK” overstates the interpolation basis. Adjacent slopes range from **0.0537 to 0.1884 dex/kK**. Log-linear interpolation between grid points is a new authored model with no source-provided interpolation error or band.

## Provenance and validation defects

- The BSTAR2006 DOI is wrong at [`registry.toml:269`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/sources/registry.toml:269) and generated `SOURCES.md`: it says `10.1086/511268`; the primary record says **`10.1086/511270`**. [Official arXiv record.](https://arxiv.org/abs/astro-ph/0611891)
- [`registry.toml:305`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/sources/registry.toml:305) still says “Not yet wired,” although the current diff wires it.
- [`registry.toml:301`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/sources/registry.toml:301) says three bibcodes; the VOTables contain two bibcodes plus a TLUSTY homepage URL.
- The categorical statement that model SED values are “uncopyrightable facts” and safe independently of redistribution at [`registry.toml:300`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/sources/registry.toml:300) has no cited licence/legal evidence. `redistributable=false` is conservative; the broader legal conclusion is not verified provenance.
- Commit `d624dcb` explicitly claims an SVO acknowledgement requirement as a licence reason. A later commit removes it, but the false claim remains part of the audited PR arc.
- The working note’s rough Sternberg check uses `R≈7 R☉` without an anchor, and its single-bin check omits which Teff/FID was checked. Those are sanity checks, not independent validation of the 16-point mapping.
- The table endpoint tests at [`astro.rs:6882`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:6882) duplicate the constructor literals; interpolation tests verify the interpolation rule against itself. The 20 kK spectrum test similarly copies `-2.2815`. They test plumbing, not source recovery.
- Residual tests at [`giants.rs:2942`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:2942) assert the enum field and tautological zero ledger, never a recipient before/after balance.
- The canonical-state test at [`giants.rs:2203`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/giants.rs:2203) proves only that tighter truncation shortens `t_visc`; it does not prove that truncation changes the gas-limited verdict, that one set of inputs was used, or that `tau_disk` is carried.
- “EUV is negligible below 15,000 K” at [`astro.rs:2727`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/crates/sim/src/astro.rs:2727) confuses a dataset boundary with a physical zero.
- The roadmap’s “ARC COMPLETE” claim at [`CONSENSUS_ROADMAP.md:11`](/home/nathan/.claude/jobs/90427dee/tmp/pr212/docs/working/CONSENSUS_ROADMAP.md:11) is false given the missing direct rate/consumer, missing wind selector, discarded collapse band, and uncoupled gas verdict.

## What checked out

The 16 SED identities, sizes, and hashes match the recorded receipts; the 14-witness/2-pending archive grading is honest; the corrected photon integral and four-decimal departure values are sound. The new deterministic paths use fixed-point arithmetic and fixed/input-determined trip counts, and the departure interpolation arithmetic itself stays safely inside Q32.32. `git diff --check` is clean.
