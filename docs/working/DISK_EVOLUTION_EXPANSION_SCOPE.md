# Disk-evolution expansion scope: the chain over non-main-sequence stars

Status: scoped and gate-ruled, held at its own gate after slice 3. This records the spine so the expansion stands ready. The load-bearing piece (the pre-main-sequence contraction luminosity) already jumped this gate and is BUILT in the base arc, because the race's dropped-multiplicand catch gave it a consumer inside the current arc; everything else here waits.

The working principle the whole expansion rests on: EVERY `None` IS A DOOR. The base arc's mechanisms are parametric over state variables and guarded at their fit domains, so the domain guards that refuse today are the dispatch seams tomorrow. The expansion is wires and branches, not rewrites, and every guard keeps its exact semantics while gaining a second exit. Total cost: two wires, one derived branch with a band, two conditioning fields, one named axis, one derived power law (the contraction luminosity, already built), and zero authored scalars.

## Move one: feed the star's own track, and the saturation blindness

Disk-hosting stars are not main-sequence objects: they are pre-main-sequence stars descending the Hayashi track, brighter and fully convective. The main-sequence bias in the current chain lives entirely in what callers feed, so it is a wire, not a rewrite.

- THE `L_bol` WIRE (built). `L_bol` wires to the pre-main-sequence contraction luminosity (`pre_main_sequence_luminosity_lsun`, built in the base arc) instead of a main-sequence instance. This is fully derived from the `n = 3/2` polytrope at the H-minus wall temperature (`L ~ t^(-2/3)`), reading the Hayashi attractor the #77 ruling already built (the same wall serves both directions of travel). The premise correction that produced it: the #77 ruling built the temperature ATTRACTOR (the wall), not the pre-main-sequence LUMINOSITY LAW; the wall was ruled and the law was needed, so the law was derived. `L_bol` precision matters here (unlike `tau_conv`, below), since in saturation `L_X = plateau * L_bol` and a pre-main-sequence star is brighter than its main-sequence instance.
- THE SATURATION BLINDNESS (declared, with its consumer named). During the disk era every star is rotationally saturated: fast rotation against a long convective turnover puts `Ro` far below the saturation knee, so the activity fraction sits on the saturated plateau and the disk arc is nearly insensitive to `tau_conv`'s precision. That is why a main-sequence-calibrated turnover polynomial gives right answers in this window. The full Rossby machinery's precision matters where stars LEAVE saturation, gigayears in, which is the atmosphere-escape arc's domain, not this one's. This blindness is now written into the spin-down doc as the consumer split.

## Move two: the high-mass domain guard becomes a two-branch dispatch (the second branch DERIVED)

The convective-turnover domain guard's high-mass refusal (`convective_turnover_time_days` returning `None` above its fit range) is the dispatch seam. Two branches, keyed on the star's structural state:

- CONVECTIVE-ENVELOPE stars drive winds through the dynamo (X-rays), the branch built.
- RADIATIVE-ENVELOPE stars (Herbig Ae/Be and up) have no dynamo, which the guard correctly refuses today, but they photoevaporate HARDER: a hot photosphere is intrinsically EUV-bright without a corona, so the ionizing luminosity is the spectrum's own high-energy tail, DERIVED from `T_eff` and `L` (both owned by the star module) with an atmosphere-model band. Same race, same gravitational radius (the EUV wind temperature is already a banded caller value), a different wind-rate source.

REFINEMENT (gate-adopted): the dispatch key is the star's own STRUCTURAL STATE from the track (convective versus radiative envelope), not a mass cut, so the `1.4 M_sun` figure is demoted to the main-sequence instance of a structure-keyed line. That structural state is itself a derived quantity (fully convective on the pre-main-sequence, mass-dependent on the main sequence), a small derivation the branch needs, not a field that exists today. The convicting populations write themselves as predictions: Herbig stars clear disks faster, massive stars are diskless within a megayear (their main-sequence arrival overlaps their accretion), and planet occurrence collapses above a few solar masses. The observed disk-lifetime-versus-stellar-mass trend joins the hindcast rows (a flagged fetch target) and referees the branch the day it lands.

## Move three: conditioning fields and one named axis, priced not built

- `XrayWindFit` GAINS A METALLICITY-OF-SAMPLE FIELD. The coefficients were fit at solar composition, and the Sellek 2024 row's molecular cooling is precisely the mechanism by which metallicity moves the rate, so low-metallicity draws widen toward Sellek's edge of the already-declared band rather than extrapolating a solar fit silently. Half the metallicity dependence already propagates for free: the formation root consumes the derived dust column and opacity from the composition wire, so a metal-poor draw automatically runs a cooler, faster-clearing disk on the thermal side.
- BINARITY, named without scope creep. Half of all stars; a companion truncates the disk tidally, capping `R_1` at a derived truncation radius and shortening `tau_disk` through the same machinery untouched. It is a layer-4 axis to price when wanted, entering as a derived cap with zero new free values.

## The domain declaration (the "disk" vocabulary fence)

This arc governs the NATAL GAS DISK, birth to `tau_disk`. White-dwarf debris disks, Be-star decretion disks, and post-AGB circumbinary material are SECOND-GENERATION objects fed by the small-body reservoir and the tidal machinery, other arcs' physics. The word "disk" carries three mechanisms, and this project has a defect class named for exactly that (the same shape as the "wind" trap the base arc already fenced: photoevaporative winds disperse, MHD winds transport), so the arc doc earns this one-line lexical fence keeping them apart.

## Build order at the gate

The spine stands. When the expansion opens (after slice 3), the order is: the structure-keyed dispatch state first (it keys both the wind-branch dispatch and the `L_bol` track selection), then the radiative-envelope wind branch with its atmosphere-model band, then the conditioning fields, then the binarity axis when priced. The mass-keyed gyrochrone that lands the `P_ref` draw (resolving the colour-versus-mass keying) is a sibling follow-on that the whole chain reads. Every piece is confirmed against code by the base-arc pass; none is a rewrite.
