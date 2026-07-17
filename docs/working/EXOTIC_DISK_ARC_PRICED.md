# The exotic-disk arc: priced, parked, and one rung pulled forward

Owner-ruled 2026-07-17. PRICED NOW, BUILT LATER. Nothing here is built until the main-sequence catalog stands, because the capstone does not wait on pulsars. The point of pricing it now is that when someone eventually asks this engine for THE FIRST PLANETS EVER DISCOVERED, the answer is A SUPPLY CHAIN AWAY INSTEAD OF A REWRITE.

## The domain line, which is why this is an arc rather than a rewrite

The NATAL arc governs the gas disk a star is BORN with. A neutron star's planets, where they exist at all, are NOT born from that disk: the supernova destroys or unbinds the first generation, and what a pulsar hosts afterward is a SECOND-GENERATION disk, supernova fallback or a shredded companion. THAT IS A DIFFERENT SUPPLY CHAIN FEEDING THE SAME KIND OF MACHINE.

**THE ARCS ARE SUPPLY CHAINS; THE MECHANISMS ARE ARC-AGNOSTIC.** That sentence is the whole price estimate, and it is the parametric discipline paying its full fare. Every mechanism the disk arc built is CALLER-PARAMETRIC and none of them knows it is serving a T Tauri star: the `Mdot(t)` clock takes any `(Mdot_0, t_visc, gamma)`; the wind-versus-accretion race takes any wind rate; the gravitational radius takes any `(M, T_wind, mu)`. A FALLBACK DISK IS A DIFFERENT SET OF ARGUMENTS, NOT A DIFFERENT FUNCTION: an initial radius of a fraction of an AU, a viscous time of DAYS rather than megayears, and a composition that is essentially pure metals.

## What does NOT transfer, and the guard already knows it

THE `L_X` CHAIN. A neutron star has NO CONVECTIVE ENVELOPE, so the Rossby machinery has nothing to run on and THE `tau_conv` DOMAIN GUARD CORRECTLY REFUSES. That refusal is the design working, not a gap.

A compact object's high-energy luminosity is entirely different physics:
- SPIN-DOWN POWER for pulsars: the magnetic dipole formula, FULLY DERIVED `[D]` from field, spin, and radius. No reserved amplitude, no decay exponent to fetch.
- ACCRETION POWER where there is inflow.

So `L_X` GAINS A SECOND PROVIDER, DISPATCHED ON STELLAR CLASS, REGISTERED THROUGH `@provides` SO THE DIAMOND GATE SEES BOTH PROVIDERS WITH THEIR ARBITRATION FROM BIRTH. This is the PRE-REGISTRATION PATTERN USED DELIBERATELY THIS TIME, and the qualifier is earned: the last pre-registration (the E_coh ladder) was scheduled against a premise nobody checked, and its "second carrier" turned out to be one column and its reflection. Here the second provider is REAL PHYSICS (a dipole against a corona), the two are independent in fact, and the arbitration is a CLASS DISPATCH rather than a ladder. Register it when the second provider is written, never before, and verify the first provider's route at the site the way the correction taught.

## The rungs

1. **The Alfvén radius (magnetospheric truncation).** PULLED FORWARD, see below.
2. **The propeller condition**: the disk is EJECTED when truncation sits outside corotation. For a magnetar that condition mostly answers "NO DISK SURVIVES", AND THAT IS ITSELF THE DERIVED OUTPUT. Magnetars are the same class dispatch at extreme field, not a new branch.
3. **Spin-down luminosity**, `[D]` from the dipole formula over field, spin, radius.
4. **The fallback reservoir**: a WIDE-BANDED SEEDED DRAW. The data is thin, and THE HONESTY IS THE BAND. Never a median (the statistic-with-a-hidden-conditioning-variable class, convicted three times in this project already).
5. **Second-generation condensation**, which routes through machinery being built for other reasons: a fallback disk's chemistry is OXYGEN, SILICON, AND IRON WITH NO HYDROGEN TO SPEAK OF, which is the exotic-condensation roster's EXTREME TEST CASE, the C/O-guard territory pushed to its limit. The s/r-process draw connects too, since the ejecta composition is the supernova's own nucleosynthesis.

## The one rung pulled forward, because the debt already exists

**BUILD THE ALFVEN RADIUS ONCE AND IT SERVES BOTH ARCS**: the natal disk's inner edge TODAY, and the pulsar disk's survival question whenever this arc opens.

THE NATAL ARC ALREADY OWES IT, and the code says so in its own words. `crates/sim/src/planetary_assembly.rs:862`:

```rust
const PLANET_ZONE_INNER_AU: Fixed = Fixed::from_int(1); // planet-zone proxy, NOT the magnetospheric edge
```

with line 856 naming what it stands in for: "The physical inner bound is the magnetospheric truncation (a few stellar radii)". So this is not new work justified by a parked arc; it is EXISTING DEBT that a parked arc happens to share, and that is the only reason it is pulled forward. Verified at the site rather than taken from the ruling.

## The founding hindcast row, and it validates on BOTH existence and scarcity

**PSR B1257+12**: the FIRST EXOPLANETS HUMANITY EVER CONFIRMED, terrestrial-mass planets around a millisecond pulsar.

The engine must be able to PRODUCE that system, second-generation rocky worlds condensed from metal-rich debris. AND IT MUST PRODUCE IT RARELY, because THE OBSERVED RARITY OF PULSAR PLANETS IS THE POPULATION CONSTRAINT. An arc that makes pulsar planets common has failed even while reproducing the system. Both halves are the row.

## The rest of the zoo, split by the same sentence read twice

- **WHITE DWARFS**: already inside the plan. #77 carries stars to that endpoint; survivor planets are tides-and-mass-loss machinery; WD debris disks were scoped as second-generation objects fed by the small-body reservoir, WHICH EXISTS.
- **BLACK HOLES**: the pulsar case MINUS A SURFACE. Accretion-powered irradiation only.
- **EXOTIC STARS AS STARS** (stripped envelopes, Thorne-Zytkow objects, blue stragglers): #77 and binary-evolution territory, NOT disk territory.

EXOTIC DISKS ROUTE THROUGH THIS ARC; EXOTIC STELLAR STRUCTURES ROUTE THROUGH THE EVOLUTION MACHINERY. The split is the same domain sentence read twice.

## Sequencing

PARKED. Build none of it until the main-sequence catalog stands. The Alfven radius is the single exception, and it is an exception because the natal arc already owed it, not because this arc wants it.
