# Mirror surface-energy-balance: close the hot bias by derivation (arc kickoff)

This is the design-first kickoff for the Mirror surface-energy-balance arc, the gate's next-arc assignment after the role-dimension declaration (#137). It establishes the comms bridge and states the arc's scope and discipline. No code lands until the gate gates the grounded design, which follows on this PR.

## The arc

The calibrated Mirror runs about 14 K too hot at the surface (roughly 302 K where Earth is 288 K), because the diurnal surface balance keeps only the radiative loss and omits the turbulent surface cooling (the latent and sensible fluxes). The fix is DERIVATION, never authoring 288 K: the missing terms close from substrate that already exists, plus one new derived air-temperature reference state. 288 K must EMERGE from a closed surface balance rather than being written into the diurnal path.

## The approach (to be grounded and detailed in the design that follows, not yet verified against source)

The gate's build list, which I will confirm against source before endorsing:

- **Latent cooling** closes from pieces that already exist: `Q_latent = E * L_vap`, where the evaporation mass flux `E` is derived every tick by `laws::evaporation_rate` inside `step_hydrology`, and `L_vap` is the cited `metabolism.water_loss_per_joule = 1/L_vap` that physiology reads. Couple the existing latent-heat flux as surface cooling; no new authored number.
- **Sensible cooling** closes in mechanism but needs one new state: `laws::convective_flux(h, area, T_surf, T_air)` exists and `h = fluid.convective_coefficient` is a floor axis on the air medium, but `T_air` must be an INDEPENDENT reference (a mixed-layer or lapse-rate air temperature), not the surface field itself. Deriving that air-temperature reference state is the one new substrate piece; derive it, never author a gradient.
- **The combined kernel**: `radiative_equilibrium` is closed-form only and cannot absorb the implicit sensible term, so a new implicit or linearized surface-balance kernel solves absorbed shortwave minus radiative emission minus latent minus sensible for `T_surf`.

## Deferred, flagged not faked

- Back-radiation from atmospheric composition needs a greenhouse-gas column substrate that is absent, so it stays the cited per-world `back_radiation` datum for now. Flag it, do not fake it.
- The static `climate.mean_surface_temperature = 288` worldgen anchor is a distinct category-b datum; making it emergent is a larger reconciliation, out of scope here.

## Calibration residual

The Mirror Dalton evaporation coefficients (`hydrology.evaporation_still/wind/cap`) are fixture placeholders. This arc surfaces their Mirror basis reserved-with-basis from real mass-transfer data, computed independently of the target temperature: calibrate to real data, accept the resulting temperature, never tune until it hits 288 K.

## Byte-neutrality

The diurnal path is opt-in, so unarmed runs hold the four canonical pins (`4bbf6b59`, `1db633b3`, `c9d5cc17`, `ad69f2bf`). Arming the new cooling terms re-pins `living`, a stated re-pin whose reason is the added latent and sensible surface cooling.

## Design-first discipline and the substrate-first header

No code until the gate gates the design. The next step grounds the arc against `environ.rs` (`step_hydrology`, `step_productivity`, `DiurnalSky::mirror`), `laws::convective_flux` / `evaporation_rate` / `radiative_equilibrium`, `medium.rs`, and `docs/working/PHYSICS_FLOOR_REGISTRY.md`, then posts the kernel form, the air-state derivation, and the Dalton-coefficient basis for the gate's design-gate.

Substrate-first: before authoring or flagging any value owner-set, the physics floor registry is read and it is proven in writing that no substrate derives it. The latent and sensible terms derive from existing substrate; the air-state derives from a lapse-rate reference; a Dalton coefficient is a per-world datum from real mass-transfer data with basis, never a fabricated number. Where a substrate is missing (the greenhouse column), the honest output is to build the substrate later, not to author the value now.
