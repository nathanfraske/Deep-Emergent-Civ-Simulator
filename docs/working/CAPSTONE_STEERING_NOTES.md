# Capstone steering notes (researcher, pre-panel, 2026-07-16)

Five notes from the researcher's steering pass, distilled to what binds the build. A post-panel steer will follow.

## 1. The Delta unit collision (a standing rule, shared fault)

Eq. (83)'s spacing variable Delta is the FRACTIONAL orbital separation `(a2 - a1)/a1`, while the earlier B-ruling quoted Obertas and Weiss in MUTUAL HILL RADII. The unit convention collided silently across two ruling documents (researcher and manager both). STANDING RULE: no coefficient row ships without its COORDINATE DEFINITION inline, the variable, its normalization, and the source equation number, every time. Pre-flag for the Eq. (82) general (uneven-mass) upgrade: there `delta` is Petit's generalized spacing (Eq. 45, HALF the fractional separation in the equal-spacing case) and `eta` is the resonance locator (Eqs. 21/22, NOT a spacing). REGRESSION TO WIRE before the general form lands: evaluate Eq. (82) with `eps_M = 1.22*eps` and `eta = 1/2` and it must reproduce Eq. (83)'s intercept -6.51 exactly (`-log10(0.61) = 0.215`, `-6.72 + 0.215 = -6.51`).

## 2. The 41-planet count is verified, and it constrains what the fix may touch

At fixed Hill-unit spacing the packable count scales as `m^(-1/3)`. Over [1,30] AU (`ln 30 ~ 3.40`) at 13.7 mutual Hill radii: 0.1 M_earth survivors give a step of 0.080/planet, N ~ 42; Earth-mass survivors give 0.173, N ~ 20. So the 41 IS the ten-times embryo mass deficit read back through the cube root, to the planet: the generator is a MEASUREMENT INSTRUMENT of its own input error (the strongest mechanism validation). Steers:
- CALIBRATION LANDS UPSTREAM ONLY. Embryo mass traces through the isolation-mass law to gate-G's Sigma normalization and `b`; `M_iso ~ Sigma^(3/2)`, so the ten-times embryo deficit is roughly a FACTOR 4 to 5 in Sigma_solid. Touching the assembly to move the count would AUTHOR the outcome (forbidden by the C-ruling). The assembly propagated its input faithfully and STAYS UNTOUCHED.
- ZONE-SCOPE the regression target. The 3.6 +/- 0.8 multiplicity row is a [0.5, 1.5] AU standard-disk ensemble. A [1, 30] AU run WITHOUT the #73 giants branch should NOT read four, and with corrected masses it legitimately reads NEAR TWENTY until giants exist to eat the outer zone and secularly sculpt the inner. Calibrating embryo masses against "four over [1,30]" is tuning to a malformed target.
- The Kepler-peak comparison (13.7 mean vs observed ~20) runs THROUGH the detection forward model (the firewall rule; the observed peak is a detected distribution, not the physical one).
- BOOKKEEPING (request now): mass conserved to the bit is HALF the double-entry. Confirm each merge POSTS its binding-energy release to the HEAT LEDGER, because that posted series is what the R-YOUNG impact-list fold-in consumes later. Dropped energy today is a broken reversibility key tomorrow.

## 3. The R-YOUNG 2% relief discriminator tree (aim, do not sweep)

- STEP 1: pin the rendered epoch. A world just after rheological lockup is LEGITIMATELY smooth; the bumps chain builds relief as the lid thickens and provinces refreeze across their own solidus. Low relief at `t ~ t_lockup` may be correct physics, not a defect.
- STEP 2: if the epoch has a thick lid, compute the world's own SUPPORTABLE-RELIEF BOUND (the `strength / (rho*g)` form with the derived yield stress, lid thickness, and gravity), km-class for an Earth-like operating point. That is the expected amplitude.
- STEP 3: if the derived field says km but the viewer shows 2%, the defect is a UNITS or NORMALIZATION drop at the relief-to-render boundary (meters versus normalized height), the same class as the Delta error. (Sent to the R-YOUNG hardening agent as its aiming steer.)

## 4. Kill the class, not the instances (the systemic fix)

The Delta error and the render-amplitude ambiguity are one class: a BARE NUMBER crossing a module boundary stripped of its unit, surfaced twice in one wave. The codebase already uses typestate to make sub-resolution verdicts unreadable; extend the same philosophy to DIMENSIONS. No bare float crosses a module boundary: NEWTYPE the cross-boundary quantities (spacing carries its normalization, height carries meters, energy carries its ledger destination) so the compiler rejects the next Delta and the next render amplitude before an auditor sees them. A one-time cost, permanent retirement of the failure class. Flagged as its own arc (R-DIMENSIONS).

## 5. Sequencing

- Gate-G's Sigma CALIBRATION is small and lands BEFORE per-planet world generation, or the terrestrial zone fills with 41 sub-scale geologies.
- #44 (secular) is pure computed math with its inputs now standing, so it runs in PARALLEL immediately; its mode table feeds the per-planet climate work. (Check the fractional-power primitive #45 dependency for the Laplace coefficients.)
- #73 (giants) GATES the outer zone: it precedes outer-zone per-planet worlds, OR those worlds ship SCOPE-FENCED (provenance-keyed provisional, no archives the giants fold-in would contradict), exactly like the bumps interim.
- Carbides and #77 stay orthogonal, any time.

Closing (researcher): the assembly agent verifying the coefficient against the primary source before implementing, and reporting the count instead of gating it, is the founding rule and the surfaced-not-asserted rule running unsupervised two levels below their author. That is what load-bearing looks like.

---

## POST-PANEL STEERING (researcher, after the R-YOUNG five-lens, 2026-07-16)

The core R-YOUNG claim held under audit (the handoff scaling with the world's own derived solidus survived adversarial contact); both panel findings were labeling honesty on sound mechanism, the failure profile you want at this stage. Five steers; 1 and 2 are pre-emptions (each panel fix has a second failure mode behind it), sent to the hardening agent before its first report.

1. DEFINE "VISIBLY BUMPY" BEFORE THE KM NUMBER. Real planetary relief is SUB-1% of radius (Mars ~30 km / 3390 = 0.9%, Earth ~20 km / 6371 = 0.3%), so a correctly-derived world rendered at PHYSICAL scale looks nearly smooth, and the eyeball fails sound physics (round two). Ship THREE separated, tagged quantities: (a) the derived relief AMPLITUDE IN KM (the physics deliverable, gets a hindcast row); (b) a SUPPORT-BOUND check (amplitude <= the world's own `strength/(rho*g)` class bound at its lid state, excess FLAGGED not rendered); (c) a declared VERTICAL-EXAGGERATION factor for the viewer, non-canon display-only (the f_max discipline). The truthful bumpy criterion is the km amplitude inside the hindcast band, never the unexaggerated eyeball. FETCH: exact DEM extremes for Mars and Earth (the hindcast row).

2. THE DEFAULT REGRADE IS MARGINAL-CARRIED, NOT COLD. The melt verdict is jointly decided by formation time, the 26Al0 draw, the retention form over the feedstock size spectrum, AND mass; at Mars-class mass the accretion term alone spans sub-melting to melting across the h form's band. Flipping the branch on the 3-4 Myr formation-time POINT repeats the one-axis mistake on a new axis. The honest verdict is NEAR-DEGENERATE, carried (Gap Law). The empirical anchor forbids the cold landing: real Mars IS differentiated, cored, carries early-dynamo remanence, and Hf/W makes its accretion few-Myr fast. BATTERY ROW: any parameterization leaving a Mars-class world UNMELTED at Mars-like formation times FAILS hindcast. FETCH: the Dauphas-Pourmand Mars accretion timescale.

3. GENERALIZE THE CONTAINMENT SELF-TEST INTO A MECHANISM. The rule is general (no banded input flips a GAPPED verdict); the implementation swept retention only and formation time walked through. Make the GAPPED-tag writer WALK THE PROVENANCE DAG and band-sweep every ancestor tagged interim / [E] / [C] automatically, demoting to MARGINAL at write time on any flip. Standing machinery, not a patch; it catches every axis without a panel.

4. SEMANTIC NEWTYPES, not just SI dimensions (upgrading note 4). The 5% incident is dimensionless-versus-dimensionless (a self-normalized ContrastRatio consumed as a relief magnitude), which no unit system catches. The newtypes must encode MEANING: `ContrastRatio` vs `ReliefFraction` vs `Meters`. Dimensionless is where this class hides. Three instances in one wave (the Delta convention, the ratio-as-magnitude readout, the computed-but-unreported km): the rule (R-DIMENSIONS) has earned its cost.

5. c_p IS A ONE-LINE DERIVATION, not a data project. At magma-ocean temperatures c_p per assemblage is Dulong-Petit on the world's own mean atomic mass, `c_p ~ 3R per atom / <m_atom>`, from the banked Layer-2 columns (iron ~447 J/kg/K, forsterite-class silicate ~1240, both matching measured high-T), with a banded Debye correction below the Debye temperature. [D]-form, zero new rows, an iron world judges itself. The SLR-registry follow-on folds into the existing SLR-family ruling, not a new slot.

The previous memo's sequencing stands unchanged (gate-G Sigma calibration before per-planet worlds; #44 in parallel now; #73 or scope-fencing before outer-zone worlds); the panel verdict touches none of it.

## Confirmed bookkeeping gap (manager check, 2026-07-16)

The assembly merge conserves MASS to the bit but does NOT post the binding-energy release to a heat ledger (`planetary_assembly.rs` has no energy posting): the missing half of the double-entry the researcher requested. This is the reversibility key the R-YOUNG impact-list fold-in consumes later, so post the per-merge binding-energy release to the heat ledger. A conservation-completeness fix (allowed; it does not touch the count), sequenced with the gate-G Sigma calibration.
