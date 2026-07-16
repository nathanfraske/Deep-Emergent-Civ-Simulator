// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The PER-WORLD DISK COMPOSITION: the star's own elemental abundance pattern, the initial condition the
//! condensation sequence reads. This is the datum that retires the solar-bias defect where every world's crust
//! condensed from the Sun's composition. The condensation kernel (`surface_composition::derive_surface_composition`
//! and the viewer's `derive_uncompressed_bulk_density`) reads WHATEVER pattern this datum carries, so each world's
//! crust and density derive from that world's own star, not a universal solar table.
//!
//! THE PATTERN (a per-world initial-condition datum, the `OrbitalElements::dev_earth` and `DiurnalSky::mirror`
//! model). A world's disk composition is a physical property of its star, not an Earth or Sun constant. The pattern
//! lives here as a labelled per-world datum: one option among many, surfaced for the owner or for worldgen to set,
//! never a silent global default. Physics may be an authored cultural input (Principle 9): the composition a world
//! forms from is such an input, read as data per world. Nothing in this datum authors a cultural or emergent
//! outcome; it seeds the physics, and the crust that follows emerges from the condensation.
//!
//! ADMIT THE ALIEN (Prime Directive 7). The datum keys on the star's own abundance pattern, so an alien star is a
//! data row, never a rewrite. A carbon star with `C/O > 1` is a different pattern that will drive the condensation
//! toward carbides and graphite rather than silicates (once the carbide gas species land in the JANAF budget); a
//! metal-poor (low-`Z`) or metal-rich star is a pattern with the rock-formers scaled against hydrogen. The
//! [`DiskComposition::from_pattern`] constructor takes any such pattern; [`DiskComposition::mirror`] is the one
//! labelled fixture (the Sun's composition, Mirror's initial condition), the same standing as `dev_earth`.
//!
//! THE HELD-OUT REFERENCE, NOT THE INPUT. The condensation reads THIS per-world datum. The AGSS09 solar table
//! (`civsim_physics::solar_abundances`) and the Lodders 2003 condensation fronts
//! (`civsim_physics::condensation`) are HELD-OUT CROSS-CHECKERS: they validate the DERIVED output (the condensation
//! front, the mineralogy, the density), on a DIFFERENT quantity than the abundance input, never fed into the
//! reasoning. The cross-check in this module's tests mirrors the Lodders gate in
//! `equilibrium_condensation`: it runs the Mirror datum through the condensation and proves the derived iron front
//! reproduces the INDEPENDENTLY-computed Lodders front within an inter-dataset band. That the Mirror datum's
//! numbers coincide with the Sun's measured composition is expected (Mirror mirrors the Sun, so the Sun's real
//! pattern is Mirror's initial condition, Directive-blessed); the non-circularity is that the validation compares a
//! derived condensation temperature against an independent thermochemistry, not "abundances in, abundances out".
//!
//! NORTH STAR (flagged, not built). The deep version derives the composition ITSELF from stellar nucleosynthesis
//! and galactic chemical evolution, so even the pattern is an output of an upstream substrate and the AGSS09 table
//! becomes a pure check with no input role anywhere. That is a large arc (a nucleosynthetic-yield substrate keyed
//! on stellar mass, generation, and metallicity history); this datum is the initial-condition rung beneath it, the
//! seam where such a derivation would plug in. Until it lands, the pattern is read as per-world data.

use civsim_core::gauss::{gaussian, GaussApprox};
use civsim_core::{splitmix64, Fixed, Rng};
use civsim_physics::solar_abundances::{SolarAbundanceError, SolarAbundances};

/// A world's DISK COMPOSITION: the elemental abundance pattern its star and disk formed from, the initial condition
/// the condensation sequence reads. The `label` is provenance (which world's pattern this is), not canonical state;
/// the `abundances` carry the per-element pattern in the [`SolarAbundances`] container (the abundance-pattern type,
/// reused for its shape, not because the pattern must be solar). The condensation reads [`DiskComposition::pattern`]
/// exactly where it used to read `SolarAbundances::standard()`, so the pipeline changes not at all; only the SOURCE
/// of the pattern moves from a universal solar table to this per-world datum.
#[derive(Debug, Clone)]
pub struct DiskComposition {
    label: String,
    abundances: SolarAbundances,
    /// The star model's metallicity ratio `Z / Z_sun` for this datum, when the datum DECLARES one: [`Fixed::ONE`] for
    /// the solar/Mirror instance, `10^[Fe/H]` for a DRAWN composition (the generator), and `None` for a bare
    /// [`DiskComposition::from_pattern`] pattern that carries no declared metallicity (the honest gap the plumbing
    /// commit held open, now filled by the draw for drawn worlds). [`DiskComposition::metallicity_ratio_to_solar`]
    /// returns it directly, so a drawn `Z` is a stored datum rather than an inference off the abundance metadata.
    metallicity_ratio: Option<Fixed>,
}

impl DiskComposition {
    /// Build a disk composition from an explicit per-world abundance pattern. The general constructor: a worldgen
    /// stage, an owner-set scenario, or (the north star) a nucleosynthesis substrate hands the pattern in, and the
    /// condensation reads it. `label` names the world's pattern for provenance. This is the seam the alien enters
    /// as data: a carbon-star or metal-poor pattern is a different `abundances`, never a code change.
    pub fn from_pattern(label: impl Into<String>, abundances: SolarAbundances) -> Self {
        DiskComposition {
            label: label.into(),
            abundances,
            // A bare pattern declares no metallicity ratio: the honest `None` the plumbing commit held open. A world
            // that wants a declared `Z / Z_sun` uses [`DiskComposition::mirror`] (unity) or [`DiskComposition::draw`]
            // (the generator's `10^[Fe/H]`).
            metallicity_ratio: None,
        }
    }

    /// The MIRROR world's disk composition: the Sun's own measured elemental pattern as Mirror's initial condition,
    /// the labelled fixture in the standing of `OrbitalElements::dev_earth` and `DiurnalSky::mirror`. Mirror mirrors
    /// the real Sun, so its formation pattern IS the Sun's real composition (the AGSS09 measurement), read here as
    /// Mirror's per-world datum. An alien world sets a different pattern through [`DiskComposition::from_pattern`];
    /// this is one option among many, not a global default. Fails loud if the solar pattern does not load (the
    /// fail-loud-while-reserved discipline: no silent fallback).
    pub fn mirror() -> Result<Self, SolarAbundanceError> {
        Ok(DiskComposition {
            label: "mirror-solar".to_string(),
            abundances: SolarAbundances::standard()?,
            // The solar instance: Z == Z_sun, so the ratio is unity BY CONSTRUCTION, no value chosen. This equals the
            // value the pinned chain produces ([`Environment::local_disk_solar_pin`] draws [Fe/H] = 0, so 10^0 = 1),
            // so `mirror()` and the pinned draw are byte-identical (proven by test).
            metallicity_ratio: Some(Fixed::ONE),
        })
    }

    /// DRAW a per-world disk composition from the composition-draw CHAIN, conditioned on the birth `environment` and
    /// keyed on the `world_seed`. This is the GENERATOR (Stage-0, path 1): two unpinned seeds draw DIFFERENT
    /// compositions for a DERIVED reason (their `[Fe/H]` differs), so worlds stop looking the same the moment they stop
    /// being fed the one authored solar pattern.
    ///
    /// The chain is ordered by nucleosynthetic causality (never independent marginals). This builds the three OUTERMOST
    /// links: LINK 0 the birth ENVIRONMENT (the `environment` argument, the local disk the tagged default), LINK 1 the
    /// metallicity `[Fe/H]` drawn from that environment's selection-corrected MDF ([`Environment::draw_fe_h`]), and
    /// LINK 2 the alpha enhancement `[alpha/Fe]` drawn from the two-branch knee conditioned on `[Fe/H]`
    /// ([`Environment::draw_alpha_fe`]). The draw becomes a composition by the definitional conversion
    /// `Z / Z_sun = 10^[Fe/H]` (stored as the datum's metallicity ratio), by scaling the solar pattern's metals by
    /// `[Fe/H]` ([`SolarAbundances::scaled_metals_by_dex`], the AMOUNT), and by lifting the alpha rock-formers by
    /// `[alpha/Fe]` ([`SolarAbundances::scaled_alpha_by_dex`], the KIND), so the condensation, the accretion mass, the
    /// bulk density, and now the metal-core mass fraction all read the drawn pattern. The later links (C/O, s/r)
    /// differentiate further elements on top. `[Fe/H]` moves the amount (density fixed); `[alpha/Fe]` moves the
    /// rock-former-to-iron ratio (the first DENSITY lever).
    ///
    /// THE MIRROR PINS THROUGH THIS SAME PATH: [`Environment::local_disk_solar_pin`] pins `[Fe/H] = 0` (so
    /// `Z / Z_sun = 10^0 = 1` exactly, [`ten_pow`] guarantees the identity, and the metal shift is `+0`) AND pins
    /// `[alpha/Fe] = 0` (so the alpha shift is `+0`), both byte-identical to the solar pattern. The solar instance is
    /// thus the chain evaluated at its pins, not a bypass. `None` (error) only if the solar anchor fails to load.
    pub fn draw(environment: &Environment, world_seed: u64) -> Result<Self, SolarAbundanceError> {
        let fe_h = environment.draw_fe_h(world_seed);
        // Z / Z_sun = 10^[Fe/H], the definitional conversion; exactly ONE when [Fe/H] pins to 0.
        let z_ratio = ten_pow(fe_h);
        // LINK 2, [alpha/Fe] CONDITIONED ON [Fe/H]: the two-branch alpha knee (the thick-disk plateau and the
        // thin-disk solar level), drawn from its OWN named slot, pinned to 0 for the Mirror. This is the first
        // KIND-changing link: it lifts the alpha rock-formers relative to iron, which moves the metal-core mass
        // fraction and so the derived DENSITY (where [Fe/H] moved only the amount).
        let alpha_fe = environment.draw_alpha_fe(world_seed, fe_h);
        // Scale the solar pattern's metals by [Fe/H] (the amount-scaling), THEN lift the alpha elements by [alpha/Fe]
        // (the ratio-changing enhancement, keyed on the alpha-element identity). Both +0 shifts for the pin are
        // byte-identical to the solar pattern.
        let pattern = SolarAbundances::standard()?
            .scaled_metals_by_dex(fe_h)
            .scaled_alpha_by_dex(alpha_fe, ALPHA_ELEMENTS);
        Ok(DiskComposition {
            // Provenance only (non-canonical): the environment and the world seed that produced this draw.
            label: format!("{}-seed-{world_seed:016x}", environment.label()),
            abundances: pattern,
            metallicity_ratio: Some(z_ratio),
        })
    }

    /// The per-world abundance pattern the condensation reads, handed to
    /// `surface_composition::derive_surface_composition` in place of `SolarAbundances::standard()`.
    pub fn pattern(&self) -> &SolarAbundances {
        &self.abundances
    }

    /// The provenance label naming which world's pattern this is (not canonical state).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The star model's metallicity ratio `Z / Z_sun` for this datum, the composition axis `derive_planet`
    /// reads, sourced from THIS per-world datum rather than a hardcoded `Fixed::ONE` at the call site. The
    /// SOLAR INSTANCE ([`DiskComposition::mirror`]) returns exactly [`Fixed::ONE`]: its heavy-element mass
    /// fraction IS the pinned solar anchor's (`Z == Z_sun`), so the ratio is UNITY BY CONSTRUCTION, no value
    /// chosen and none fabricated. A DRAWN composition ([`DiskComposition::draw`]) returns `10^[Fe/H]`, the
    /// drawn metallicity ratio (the generator, this commit). A bare [`DiskComposition::from_pattern`] pattern
    /// declares no ratio and returns `None` (the honest gap, unchanged). The value is a STORED datum, read
    /// straight from the field rather than inferred from the abundance metadata.
    pub fn metallicity_ratio_to_solar(&self) -> Option<Fixed> {
        self.metallicity_ratio
    }
}

// ── The composition-draw chain: LINK 0 (environment), LINK 1 ([Fe/H]), LINK 2 ([alpha/Fe]) ───────────────────────

/// The `[Fe/H]` MDF PEAK (dex) for the LOCAL Milky Way thin-plus-thick disk (the tagged default environment). FETCHED:
/// the recalibrated Geneva-Copenhagen survey MDF (the largest kinematically-unbiased solar-neighbourhood sample) peaks
/// NEAR THE SOLAR value, and the SEGUE selection-corrected reference MDF concurs (Casagrande et al. 2011, A&A 530 A138,
/// arXiv 1103.4651; Schlesinger et al. 2012, ApJ 761 160). On the `[Fe/H]` scale the solar value is 0 by definition, so
/// the near-solar peak IS `[Fe/H] = 0`. The Sun sits at the mode of its own neighbourhood's distribution, which is why
/// the Mirror pin (`[Fe/H] = 0`) coincides with the peak. This is the population machinery's one calibrated instance
/// (the MMSN-pattern discipline), not a per-world knob.
const LOCAL_DISK_FE_H_PEAK_DEX: Fixed = Fixed::ZERO;

/// The intrinsic 1-sigma SCATTER (dex) of the local `[Fe/H]` MDF, ~0.20 dex, real and present at all ages (Casagrande
/// et al. 2011; the SEGUE selection-corrected reference concurs, Schlesinger et al. 2012). This is the band the draw
/// carries; the metal-poor and metal-rich tails ship with it, never clipped. `0.20 = 20/100`.
const LOCAL_DISK_FE_H_SIGMA_DEX: Fixed = Fixed::from_int(20).div(Fixed::from_int(100));

/// The name of the `[Fe/H]` link, hashed to key its OWN draw slot (steer 2, the per-link named sub-slot).
const LINK_FE_H: &str = "metallicity_fe_h";

/// The Gaussian approximation the `[Fe/H]` draw uses: the project's stamped canonical method (design 25.10), the sum of
/// 12 counter-keyed unit draws (unit variance, mean zero, +-6 sigma bound). This is the sampler's numerical METHOD (a
/// project identity, the same `k = 12` the genome spine uses), NOT a distribution parameter; the distribution's mean
/// and scatter are the fetched MDF values above.
const FE_H_GAUSS_METHOD: GaussApprox = GaussApprox::SumOfUniforms { k: 12 };

/// `10^x` in fixed point, with the exact identity `10^0 = 1` guaranteed so the Mirror pin (`[Fe/H] = 0`) yields exactly
/// [`Fixed::ONE`] (byte-identity of the pinned globe and the run pins). For a nonzero draw it is the deterministic
/// [`Fixed::powf`] (`Fixed::powf` already returns exactly ONE at 0, but the guard states the identity and removes any
/// dependence on the series edge).
fn ten_pow(x: Fixed) -> Fixed {
    if x == Fixed::ZERO {
        return Fixed::ONE;
    }
    Fixed::from_int(10).powf(x)
}

/// A CONTENT-HASH key for a chain link's draw slot: an FNV-1a fold of the link NAME, mixed through the SplitMix64
/// finalizer. Each link draws from `Rng::for_coords(world_seed, &[link_slot_key(name)])`, a stream keyed on the world
/// seed AND the link's own name, so the links are mutually independent: adding a new link (a new name, a new key)
/// cannot touch an existing link's realization for an existing seed (steer 2, proven by the per-link-slot invariant
/// test).
fn link_slot_key(name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3); // FNV-1a prime
    }
    splitmix64(h)
}

// ── LINK 2: [alpha/Fe] conditioned on [Fe/H], the two-branch alpha knee ───────────────────────────────────────────

/// The name of the `[alpha/Fe]` link, hashed to key its OWN draw slot (steer 2, the per-link named sub-slot). It must
/// key a slot DISTINCT from `[Fe/H]`, so landing this link never shifts an existing seed's already-drawn `[Fe/H]`; the
/// existing `[Fe/H]` slot-invariant test forward-references this exact name.
const LINK_ALPHA_FE: &str = "alpha_fe";

/// The nucleosynthetic ALPHA-CAPTURE element set the `[alpha/Fe]` enhancement lifts: oxygen, magnesium, silicon,
/// calcium, and titanium, the alpha-process products (even-`Z` nuclei built by successive He-4 capture and released
/// promptly by Type-II supernovae) that `[alpha/Fe]` measures against the delayed Type-Ia iron. This is a DATA-defined
/// membership (Principle 11): the enhancement MECHANISM is fixed Rust ([`SolarAbundances::scaled_alpha_by_dex`]), the
/// SET is the cited nucleosynthetic classification and grows as data (the further alpha products S, Ar, Ne plug in as
/// rows with no code change). Iron and the iron-peak Ni are NOT alpha elements: iron is the ratio's denominator and the
/// Type-Ia species the knee tracks. Keyed on element identity so an alien pattern is a data row (Prime Directive 7).
const ALPHA_ELEMENTS: &[&str] = &["O", "Mg", "Si", "Ca", "Ti"];

/// The `[alpha/Fe]` ALPHA-PLATEAU (dex), the high-alpha value the old thick-disk (and halo) branch holds at sub-solar
/// `[Fe/H]` from fast Type-II enrichment before the Type-Ia iron dilutes it. FETCHED: `[alpha/Fe] ~ +0.3`
/// (Bensby, Feltzing & Oey 2014, A&A 562 A71, arXiv 1309.2631, their `[alpha/Fe]`-`[Fe/H]` plane, section 9.3 of the
/// fetch record). `+0.3 = 3/10`.
const ALPHA_PLATEAU_DEX: Fixed = Fixed::from_int(3).div(Fixed::from_int(10));

/// The `[Fe/H]` metallicity of the alpha KNEE (dex): the turnover where delayed Type-Ia iron enters and `[alpha/Fe]`
/// begins declining from the plateau. FETCHED: `[Fe/H] ~ -0.4` (Bensby et al. 2014; the fetch flags this as read from
/// their figures, the standard reading). Below the knee the alpha branch sits on the plateau; above it the thick track
/// declines. `-0.4 = -4/10`.
const ALPHA_KNEE_FE_H_DEX: Fixed = Fixed::from_int(-4).div(Fixed::from_int(10));

/// The `[alpha/Fe]` of the thick-disk (high-alpha) branch AT solar metallicity (dex), the declined end of its knee
/// track. FETCHED: `[alpha/Fe] ~ +0.1` at `[Fe/H] = 0` (Bensby et al. 2014, "declining toward `[alpha/Fe] ~ +0.1` at
/// solar metallicity"). `+0.1 = 1/10`. (The thick fraction is ~0 at solar, so this contributes only across the knee.)
const ALPHA_THICK_AT_SOLAR_DEX: Fixed = Fixed::from_int(1).div(Fixed::from_int(10));

/// The `[alpha/Fe]` of the thin-disk (low-alpha) branch (dex), the young solar-track level. Solar `[alpha/Fe]` is 0 BY
/// DEFINITION (the scale's zero), which is the low end of the fetched thin-disk band (`[alpha/Fe] ~ 0.0` to `+0.05`,
/// Bensby et al. 2014). Using the definitional 0 makes a thin-disk world carry solar alpha ratios, and it is the value
/// the Mirror pins to. This is the "high-iron ~0" the population reaches once the thin disk dominates.
const ALPHA_THIN_DEX: Fixed = Fixed::ZERO;

// The knee constants above are the LOCAL MILKY WAY DISK instance (the tagged default the generator draws from, the same
// standing as the local MDF), NOT a universal alpha law. Other Galactic populations carry a DIFFERENT alpha structure
// the fetch names as the convicting bodies: the halo holds a constant `[alpha/Fe] ~ +0.3` plateau over a far wider
// low-`[Fe/H]` range with a separate low-alpha declining sequence (Nissen & Schuster 2010/2011), and the bulge puts the
// knee at a higher `[Fe/H]` with high `[alpha/Fe]` coexisting (Bensby 2017). A per-environment alpha knee (these params
// moved onto [`Environment`] beside the MDF, the way `from_mdf` already carries the per-environment `[Fe/H]` mean and
// scatter) is the flagged extension; this slice ships the local-disk knee, and a halo or bulge environment plugs its own
// knee in as data with no change to the mechanism below.

/// The THICK-DISK-BRANCH FRACTION at a metallicity `fe_h`: the probability a drawn world sits on the high-alpha
/// (old, thick-disk) branch rather than the low-alpha (thin-disk) branch. The population transitions from
/// thick-dominated below the alpha knee to thin-dominated at solar, so the fraction is 1 at or below the knee
/// (`[Fe/H] <= -0.4`) and declines to 0 at solar (`[Fe/H] = 0`), a linear interpolation between the two
/// fetched/definitional anchors (the minimal non-fabricated form; the exact population-resolved shape is a flagged
/// refinement pending thin-versus-thick MDFs). It is this changing mixture that turns the population MEAN over at the
/// knee, so the two discrete branch levels reproduce the measured bimodal gap a single Gaussian would erase.
fn thick_disk_fraction(fe_h: Fixed) -> Fixed {
    // clamp(fe_h / knee, 0, 1): the knee is negative, so a more-negative fe_h gives a ratio > 1 (clamped to all-thick),
    // and a solar-or-above fe_h gives <= 0 (clamped to all-thin).
    fe_h.div(ALPHA_KNEE_FE_H_DEX).clamp(Fixed::ZERO, Fixed::ONE)
}

/// The `[alpha/Fe]` of the THICK-DISK (high-alpha) branch at a metallicity `fe_h`: the alpha knee TRACK. It holds the
/// plateau (`+0.3`) at or below the knee (`[Fe/H] <= -0.4`), then declines LINEARLY to the fetched thick-at-solar
/// value (`+0.1`) at `[Fe/H] = 0`, the line through the two fetched anchors `(-0.4, +0.3)` and `(0, +0.1)`. Above
/// solar the fraction is 0, so the track is held at the solar value (it never fires there). The slope is DERIVED from
/// the fetched anchors, nothing invented.
fn thick_branch_alpha(fe_h: Fixed) -> Fixed {
    if fe_h <= ALPHA_KNEE_FE_H_DEX {
        return ALPHA_PLATEAU_DEX;
    }
    // frac = fe_h / knee in [0, 1] as fe_h goes knee -> 0 (clamped to 0 above solar). alpha = thick_at_solar +
    // (plateau - thick_at_solar) * frac, so alpha = plateau at the knee and thick_at_solar at solar.
    let frac = fe_h.div(ALPHA_KNEE_FE_H_DEX).clamp(Fixed::ZERO, Fixed::ONE);
    ALPHA_THICK_AT_SOLAR_DEX + (ALPHA_PLATEAU_DEX - ALPHA_THICK_AT_SOLAR_DEX).mul(frac)
}

/// LINK 0, the birth ENVIRONMENT: the outermost link of the composition-draw chain, the chemical environment the natal
/// cloud belonged to. It carries the conditioning the `[Fe/H]` link reads: the MDF peak and scatter, and (for the
/// Mirror) the solar pin. This is a REAL AXIS, not a variant with one inhabitant: the local Milky Way disk is the
/// tagged DEFAULT, and the halo (alpha-enhanced, s-poor at low iron), the bulge (high `[Fe/H]` with high
/// `[alpha/Fe]`), and the Magellanic / dwarf-galaxy metallicities (mineralogy-class-flipping) are the other values,
/// each a fetched MDF that CONVICTS a local-only draw (PIPELINE_FETCHES.md section 9.1). This slice ships the local-disk
/// default and its solar pin, plus [`Environment::from_mdf`], a test-input constructor the suite uses to prove the axis
/// re-conditions; the other environments plug in as data (their fetched mean and scatter) with no code change.
///
/// Abundances condition on ENVIRONMENT and EPOCH, never on stellar mass (the natal cloud is upstream of the stellar
/// IMF), so an FGK-measured MDF legally serves a draw for a star of any mass, an M dwarf included. EPOCH is the next
/// conditioner (the birth-environment slot the 26Al draw already uses is where it plugs in); this slice conditions on
/// environment alone and flags epoch as that next link.
#[derive(Debug, Clone)]
pub struct Environment {
    label: String,
    /// The `[Fe/H]` MDF peak (dex) for this environment (fetched).
    fe_h_mean: Fixed,
    /// The `[Fe/H]` MDF 1-sigma scatter (dex) for this environment (fetched); the tail ships with it, never clipped.
    fe_h_sigma: Fixed,
    /// When `Some`, `[Fe/H]` is PINNED to this value and NOT drawn: the solar/Mirror pin routes through the chain at
    /// its pinned value rather than around it.
    fe_h_pin: Option<Fixed>,
    /// When `Some`, `[alpha/Fe]` is PINNED to this value and NOT drawn: the solar/Mirror pin returns exactly this on
    /// the alpha link too, so the pinned pattern is byte-identical to the unshifted solar pattern. Every axis exposes
    /// this same pin interface, so the Mirror pins THROUGH the chain on every link.
    alpha_fe_pin: Option<Fixed>,
}

impl Environment {
    /// The tagged DEFAULT environment: the local Milky Way thin-plus-thick disk (the solar neighbourhood). Its
    /// `[Fe/H]` draws from the fetched selection-corrected MDF (peak near solar, ~0.20 dex scatter). The population
    /// machinery is universal; the solar-neighbourhood values are its one calibrated instance (the MMSN-pattern
    /// discipline).
    pub fn local_disk() -> Self {
        Environment {
            label: "local-disk".to_string(),
            fe_h_mean: LOCAL_DISK_FE_H_PEAK_DEX,
            fe_h_sigma: LOCAL_DISK_FE_H_SIGMA_DEX,
            fe_h_pin: None,
            alpha_fe_pin: None,
        }
    }

    /// The local-disk environment with `[Fe/H]` PINNED to the solar value (0), the Mirror's pin. The chain evaluates at
    /// the pin (`10^0 = 1`, the solar pattern unshifted), so the solar instance is the chain at its pinned value, never
    /// a bypass. Every axis exposes this same pin interface, so the Mirror pins THROUGH the chain on every link.
    pub fn local_disk_solar_pin() -> Self {
        Environment {
            label: "local-disk-solar-pin".to_string(),
            fe_h_mean: LOCAL_DISK_FE_H_PEAK_DEX,
            fe_h_sigma: LOCAL_DISK_FE_H_SIGMA_DEX,
            fe_h_pin: Some(Fixed::ZERO),
            // The solar pin also pins [alpha/Fe] = 0 (solar), so the alpha shift is +0 and the Mirror pattern is
            // byte-identical to the unshifted solar pattern on every link.
            alpha_fe_pin: Some(Fixed::ZERO),
        }
    }

    /// A TEST-INPUT environment with an explicit MDF mean and scatter (authored-and-legal by definition: a test input,
    /// not world content). The suite uses it to prove the `[Fe/H]` link RE-CONDITIONS on the environment (a shifted
    /// mean shifts the ensemble), the difference between a real axis and a comment promising one. It is also the shape
    /// the fetched halo / bulge / Magellanic environments take when they land (their own fetched mean and scatter).
    pub fn from_mdf(label: impl Into<String>, fe_h_mean: Fixed, fe_h_sigma: Fixed) -> Self {
        Environment {
            label: label.into(),
            fe_h_mean,
            fe_h_sigma,
            fe_h_pin: None,
            alpha_fe_pin: None,
        }
    }

    /// This environment's provenance label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// LINK 1: draw (or read the pin for) the metallicity `[Fe/H]` for a world, conditioned on THIS environment and
    /// keyed on the `world_seed`. When the environment pins `[Fe/H]` (the Mirror), the pin is returned and NO draw is
    /// consumed. Otherwise `[Fe/H]` is a Gaussian deviate at the environment's MDF peak and scatter, drawn from THIS
    /// link's own content-hash-keyed slot (the world seed folded with the link-name hash, [`link_slot_key`]), so
    /// landing a later link never shifts an already-drawn `[Fe/H]` for an existing seed. The Gaussian is the project's
    /// canonical deterministic approximation ([`FE_H_GAUSS_METHOD`]): mean-zero, unit-variance, bit-identical on any
    /// machine, tails bounded to +-6 sigma (+-1.2 dex here, far beyond the physical local MDF range, so the physical
    /// metal-poor and metal-rich tails are carried, not clipped). There is NO accept/reject loop: a draw is never
    /// resampled (the guards-hold-never-reroll discipline).
    pub fn draw_fe_h(&self, world_seed: u64) -> Fixed {
        if let Some(pin) = self.fe_h_pin {
            return pin;
        }
        let slot = Rng::for_coords(world_seed, &[link_slot_key(LINK_FE_H)]);
        gaussian(&slot, 0, self.fe_h_mean, self.fe_h_sigma, FE_H_GAUSS_METHOD)
    }

    /// LINK 2: draw (or read the pin for) the alpha enhancement `[alpha/Fe]` for a world, CONDITIONED on the already
    /// drawn `[Fe/H]` (`fe_h`) and keyed on the `world_seed`. When the environment pins `[alpha/Fe]` (the Mirror), the
    /// pin is returned and NO draw is consumed. Otherwise the value is drawn from the fetched TWO-BRANCH ALPHA KNEE
    /// (Bensby, Feltzing & Oey 2014): a SINGLE Bernoulli picks the high-alpha thick-disk branch with probability
    /// [`thick_disk_fraction`] (near 1 below the knee, 0 at solar) else the low-alpha thin-disk branch, and the value
    /// is that branch's level ([`thick_branch_alpha`], the plateau/knee track, or the solar thin level
    /// [`ALPHA_THIN_DEX`]). This is a CONDITIONAL on `[Fe/H]` with a knee shape and a two-branch (bimodal) band, never a
    /// single Gaussian: the discrete gap between the plateau and the thin level is the measured thick-disk sequence a
    /// Gaussian would erase.
    ///
    /// DETERMINISM AND THE SLOT. The draw is a pure function of `(world_seed, fe_h)`: the branch coin is a single
    /// [`Rng::unit_fixed`] on THIS link's OWN content-hash-keyed slot ([`LINK_ALPHA_FE`], distinct from `[Fe/H]`'s
    /// slot), so landing this link never shifts an already-drawn `[Fe/H]`, and adding a later link never shifts this
    /// `[alpha/Fe]`. There is NO accept/reject loop: the Bernoulli SELECTS a branch, it never resamples a draw (the
    /// guards-hold-never-reroll discipline). The band is the measured inter-branch span (the intra-branch broadening is
    /// the flagged refinement pending a fetched per-branch dispersion).
    pub fn draw_alpha_fe(&self, world_seed: u64, fe_h: Fixed) -> Fixed {
        if let Some(pin) = self.alpha_fe_pin {
            return pin;
        }
        let slot = Rng::for_coords(world_seed, &[link_slot_key(LINK_ALPHA_FE)]);
        let f_thick = thick_disk_fraction(fe_h);
        // ONE Bernoulli branch draw (a uniform in [0, ONE)): the thick (high-alpha) branch with probability f_thick,
        // else the thin (solar-alpha) branch. Not a reroll: a single draw selects a branch.
        let u = slot.unit_fixed(0);
        if u < f_thick {
            thick_branch_alpha(fe_h)
        } else {
            ALPHA_THIN_DEX
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_core::Fixed;

    #[test]
    fn the_mirror_datum_carries_the_suns_pattern_as_its_initial_condition() {
        // The per-world Mirror datum resolves to the Sun's real composition (Mirror mirrors the Sun). The
        // condensation reads THIS datum's pattern, not a universal solar table hardcoded in the pipeline.
        let disk = DiskComposition::mirror().expect("the Mirror disk composition loads");
        assert_eq!(disk.label(), "mirror-solar");
        // The load-bearing rock-formers are present in the pattern (the condensation reads them from here).
        let fe = disk
            .pattern()
            .preferred("Fe")
            .expect("iron is in the Mirror pattern");
        assert!(
            (fe.to_f64_lossy() - 7.50).abs() < 0.05,
            "the Mirror pattern's iron is the solar log-eps 7.50, got {}",
            fe.to_f64_lossy()
        );
        let o = disk
            .pattern()
            .preferred("O")
            .expect("oxygen in the pattern");
        // Oxygen exceeds carbon in the solar pattern (C/O < 1), the silicate-condensing regime.
        let c = disk
            .pattern()
            .preferred("C")
            .expect("carbon in the pattern");
        assert!(
            o.to_f64_lossy() > c.to_f64_lossy(),
            "the solar pattern is oxygen-rich (C/O < 1): O {} > C {}",
            o.to_f64_lossy(),
            c.to_f64_lossy()
        );
    }

    #[test]
    fn the_solar_instance_metallicity_ratio_is_unity() {
        // The per-world metallicity ratio the star model reads, sourced from the datum rather than a bare
        // `Fixed::ONE` literal. The Mirror (solar) datum resolves to Z == Z_sun, so the ratio is exactly ONE
        // (unity by construction, no value chosen). This is the seam that will carry a DRAWN Z / Z_sun once the
        // abundance-draw generator lands.
        let disk = DiskComposition::mirror().expect("the Mirror disk composition loads");
        assert_eq!(
            disk.metallicity_ratio_to_solar(),
            Some(Fixed::ONE),
            "the solar-instance datum's metallicity ratio Z/Z_sun is unity"
        );
    }

    #[test]
    fn a_bare_from_pattern_datum_declares_no_metallicity_ratio() {
        // A BARE `from_pattern` datum declares no metallicity ratio: `None`, the honest gap, never a fabricated ratio.
        // The generator fills the ratio through `DiskComposition::draw` (10^[Fe/H]), NOT through `from_pattern`, so a
        // hand-built pattern that names no Z stays `None`. (Proven still-honest after the generator landed.)
        let no_z_pattern = r#"
[[abundance]]
symbol = "H"
z = 1
log_eps_photosphere = "12.00"
source = "photospheric"

[[abundance]]
symbol = "Fe"
z = 26
log_eps_photosphere = "7.50"
source = "photospheric"
"#;
        let pattern =
            SolarAbundances::from_toml_str(no_z_pattern).expect("the stand-in pattern parses");
        let disk = DiskComposition::from_pattern("drawn-stand-in", pattern);
        assert_eq!(
            disk.metallicity_ratio_to_solar(),
            None,
            "a bare from_pattern datum declares no ratio (the generator's draw supplies one)"
        );
    }

    #[test]
    fn an_alien_carbon_star_is_a_data_row_not_a_rewrite() {
        // ADMIT THE ALIEN. A carbon star with C/O > 1 is a different abundance pattern handed to the SAME datum and
        // the SAME pipeline. The datum carries it; the condensation would read C > O and drive toward carbides and
        // graphite (once the carbide gas species land in the JANAF budget). No pathway assumes an oxygen-rich,
        // silicate world: the star is data.
        let carbon_star = r#"
[[abundance]]
symbol = "H"
z = 1
log_eps_photosphere = "12.00"
source = "photospheric"

[[abundance]]
symbol = "C"
z = 6
log_eps_photosphere = "9.20"
source = "photospheric"

[[abundance]]
symbol = "O"
z = 8
log_eps_photosphere = "8.70"
source = "photospheric"

[[abundance]]
symbol = "Fe"
z = 26
log_eps_photosphere = "7.50"
source = "photospheric"
"#;
        let pattern =
            SolarAbundances::from_toml_str(carbon_star).expect("the alien pattern parses as data");
        let disk = DiskComposition::from_pattern("alien-carbon-star", pattern);
        assert_eq!(disk.label(), "alien-carbon-star");
        let c = disk
            .pattern()
            .preferred("C")
            .expect("carbon in the pattern");
        let o = disk
            .pattern()
            .preferred("O")
            .expect("oxygen in the pattern");
        assert!(
            c.to_f64_lossy() > o.to_f64_lossy(),
            "the alien pattern is carbon-rich (C/O > 1): C {} > O {}, the carbide-condensing regime the datum admits",
            c.to_f64_lossy(),
            o.to_f64_lossy()
        );
    }

    #[test]
    fn the_mirror_datum_reproduces_the_lodders_iron_front_held_out() {
        // THE HELD-OUT CROSS-CHECK, mirroring `equilibrium_condensation::the_iron_condensation_temperature_
        // reproduces_the_lodders_front`, but SOURCING the iron abundance from the PER-WORLD MIRROR DATUM rather
        // than an inline solar fixture. This proves the per-world SOURCE flows into the physics and the DERIVED
        // condensation front reproduces the INDEPENDENTLY-computed Lodders front.
        //
        // WHAT IS THE INPUT: the per-world Mirror abundance pattern (its iron and helium log-eps), read from the
        // datum. WHAT IS HELD OUT: the Lodders 2003 iron T50 (computed by Lodders with her OWN thermochemistry, an
        // independent dataset) and the JANAF floor thermochemistry. WHAT THE CROSS-CHECK COMPARES: the derived
        // 50%-condensation TEMPERATURE against the Lodders temperature, a DIFFERENT quantity than the abundance
        // input, so this is never "abundances in, abundances out". The condensation front is a physics observable,
        // Lodders never touches the derivation, and the derivation never touches the Lodders table.
        let disk = DiskComposition::mirror().expect("the Mirror disk composition loads");

        // The iron partial pressure DERIVES from the Mirror datum's pattern, nothing fabricated. n_Fe/n_H =
        // 10^(log_eps(Fe) - 12); the gas is hydrogen-dominated, so the total particle count per H nucleus is
        // n_H2 + n_He = 0.5 (hydrogen mostly molecular H2) + 10^(log_eps(He) - 12) (helium, from the datum), and
        // x_Fe = (n_Fe/n_H) / that count. P_Fe = x_Fe * P_total at the disk total pressure 1e-4 bar. Every input
        // is read from the per-world datum (Fe and He abundances) or the disk setting.
        let log_eps_fe = disk.pattern().preferred("Fe").expect("iron in the pattern");
        let log_eps_he = disk
            .pattern()
            .preferred("He")
            .expect("helium in the pattern");
        let n_fe_over_n_h = (10.0_f64).powf(log_eps_fe.to_f64_lossy() - 12.0);
        let n_he_over_n_h = (10.0_f64).powf(log_eps_he.to_f64_lossy() - 12.0);
        // 0.5: hydrogen is mostly molecular H2 in the disk gas, so half as many H2 particles as H nuclei. This is a
        // gas-state fact, not a fabricated tuneable; the helium term is read from the datum.
        let particles_per_h = 0.5 + n_he_over_n_h;
        let x_fe = n_fe_over_n_h / particles_per_h;
        let p_total_bar = 1.0e-4_f64;
        let p_fe_bar = x_fe * p_total_bar;

        // f(T) = g(gas) - g(solid) through the JANAF mu-standard floor (the same wire the sibling gate uses).
        let janaf = civsim_physics::janaf::JanafTables::standard().expect("JANAF loads");
        let fe_g = janaf.species("Fe(g)").expect("Fe(g) in JANAF");
        let fe_cr = janaf.species("Fe(cr)").expect("Fe(cr) in JANAF");
        let f = |t: f64| -> f64 {
            let tf = Fixed::from_int(t as i32);
            let g_gas = crate::equilibrium_condensation::janaf_g_over_rt(
                fe_g.delta_f_g_at(tf).unwrap(),
                tf,
            )
            .unwrap();
            let g_sol = crate::equilibrium_condensation::janaf_g_over_rt(
                fe_cr.delta_f_g_at(tf).unwrap(),
                tf,
            )
            .unwrap();
            g_gas.checked_sub(g_sol).unwrap().to_f64_lossy()
        };
        // target = ln(P0/P_Fe) + ln 2 (at T50 half the iron is condensed, so the residual gas pressure is halved).
        let target = (1.0_f64 / p_fe_bar).ln() + 2.0_f64.ln();
        let f_lo = f(1300.0);
        let f_hi = f(1400.0);
        assert!(
            f_lo > target && target > f_hi,
            "the Fe T50 is bracketed by [1300, 1400] K: f(1300)={f_lo:.2} > target={target:.2} > f(1400)={f_hi:.2}"
        );
        let t50 = 1300.0 + 100.0 * (f_lo - target) / (f_lo - f_hi);

        // THE HELD-OUT LODDERS FRONT, consumed only here in validation, never in the derivation above.
        let lodders =
            civsim_physics::condensation::CondensationTable::standard().expect("Lodders loads");
        let lodders_fe = lodders.t50_k("Fe").expect("Fe in Lodders").to_f64_lossy();
        assert!(
            (t50 - lodders_fe).abs() < 30.0,
            "the Mirror-datum-sourced Fe T50 ({t50:.0} K) reproduces the held-out Lodders front ({lodders_fe:.0} K) within the inter-dataset +-30 K band"
        );
    }

    // ── The composition-draw chain: LINK 0 (environment) and LINK 1 ([Fe/H]) ─────────────────────────────────────

    #[test]
    fn the_fe_h_ensemble_reproduces_the_local_disk_mdf_within_band() {
        // STEER 1, the LOAD-BEARING test: acceptance is DISTRIBUTIONAL, not two-seeds-differ. An ensemble of [Fe/H]
        // draws over many seeds must reproduce the FETCHED selection-corrected local MDF (its near-solar mean and
        // ~0.20 dex scatter) WITHIN its band: the generator hindcasting its own source (Casagrande 2011 / Schlesinger
        // 2012). A broken sampler with the wrong dispersion fails HERE even though it would pass a smoke test.
        let env = Environment::local_disk();
        let n = 20_000u64;
        let mut sum = 0.0f64;
        let mut sumsq = 0.0f64;
        for seed in 0..n {
            let fe_h = env.draw_fe_h(seed).to_f64_lossy();
            sum += fe_h;
            sumsq += fe_h * fe_h;
        }
        let nf = n as f64;
        let mean = sum / nf;
        let sigma = (sumsq / nf - mean * mean).sqrt();
        assert!(
            mean.abs() < 0.02,
            "the ensemble [Fe/H] mean reconstructs the near-solar MDF peak (0.0): got {mean:.4}"
        );
        assert!(
            (sigma - 0.20).abs() < 0.02,
            "the ensemble [Fe/H] scatter reconstructs the fetched ~0.20 dex intrinsic scatter: got {sigma:.4}"
        );
    }

    #[test]
    fn adding_a_later_link_never_shifts_the_drawn_fe_h_for_a_seed() {
        // STEER 2: per-link NAMED sub-slots, never a shared sequential stream. Adding a chain link (its own
        // name-keyed slot) must NEVER change an already-drawn [Fe/H] for an existing seed. Simulate landing an
        // unrelated later link ([alpha/Fe]) by consuming a draw from ITS slot, and assert [Fe/H] is bit-identical
        // with and without that consumption (it is, because each link is a pure function of the seed and its own
        // name-key, with no shared cursor).
        let env = Environment::local_disk();
        let seed = 0x1234_5678_9abc_def0u64;
        let fe_h_alone = env.draw_fe_h(seed);
        let alpha_slot = Rng::for_coords(seed, &[link_slot_key("alpha_fe")]);
        let _later = gaussian(&alpha_slot, 0, Fixed::ZERO, Fixed::ONE, FE_H_GAUSS_METHOD);
        let fe_h_after = env.draw_fe_h(seed);
        assert_eq!(
            fe_h_alone.to_bits(),
            fe_h_after.to_bits(),
            "the [Fe/H] realization is bit-identical whether or not a later link's slot is consumed"
        );
        assert_ne!(
            link_slot_key(LINK_FE_H),
            link_slot_key("alpha_fe"),
            "each named link keys a distinct slot, so they cannot alias"
        );
    }

    #[test]
    fn the_fe_h_link_reconditions_on_the_environment() {
        // STEER 3: environment is an INTERFACE the tests exercise, not a variant with one inhabitant. A test-input
        // environment with a SHIFTED mean must shift the ENSEMBLE mean of the [Fe/H] draw (the link re-conditions on
        // the environment), the difference between a real axis and a comment promising one. This is the shape the
        // fetched halo/bulge/Magellanic environments take when they land.
        let default_env = Environment::local_disk();
        let shifted = Environment::from_mdf(
            "test-metal-poor",
            Fixed::from_int(-1).div(Fixed::from_int(2)), // mean [Fe/H] = -0.5, a test input (authored-and-legal)
            LOCAL_DISK_FE_H_SIGMA_DEX,
        );
        let n = 8000u64;
        let ens_mean = |env: &Environment| -> f64 {
            let mut s = 0.0f64;
            for seed in 0..n {
                s += env.draw_fe_h(seed).to_f64_lossy();
            }
            s / n as f64
        };
        let m_default = ens_mean(&default_env);
        let m_shifted = ens_mean(&shifted);
        assert!(
            m_default.abs() < 0.02,
            "the default environment centers the ensemble near solar: {m_default:.4}"
        );
        assert!(
            (m_shifted - (-0.5)).abs() < 0.02,
            "the shifted-mean environment re-conditions the ensemble to its own mean -0.5: {m_shifted:.4}"
        );
        assert!(
            m_shifted < m_default - 0.4,
            "the shifted-mean environment moves the ensemble metal-poor (real re-conditioning, not a no-op)"
        );
    }

    #[test]
    fn the_mirror_pins_through_the_chain_byte_identical_to_mirror() {
        // STEER 4: the Mirror pins THROUGH the chain, not around it. Drawing with the solar-pin environment yields
        // exactly the solar pattern and Z/Z_sun = ONE, byte-identical to DiskComposition::mirror() (so the default
        // globe and the run pins stay byte-identical). The seed is irrelevant under the pin (no draw is consumed).
        let pinned = DiskComposition::draw(&Environment::local_disk_solar_pin(), 0xDEAD_BEEF)
            .expect("the pinned draw resolves");
        let mirror = DiskComposition::mirror().expect("mirror loads");
        assert_eq!(
            pinned.metallicity_ratio_to_solar(),
            Some(Fixed::ONE),
            "the pinned draw's Z/Z_sun is exactly ONE (10^0)"
        );
        assert_eq!(mirror.metallicity_ratio_to_solar(), Some(Fixed::ONE));
        // The abundance pattern is byte-identical to the solar pattern on every element and both columns.
        let solar = SolarAbundances::standard().expect("solar loads");
        for sym in solar.elements() {
            assert_eq!(
                pinned
                    .pattern()
                    .log_eps_photosphere(sym)
                    .map(|f| f.to_bits()),
                mirror
                    .pattern()
                    .log_eps_photosphere(sym)
                    .map(|f| f.to_bits()),
                "pinned photospheric log-eps is byte-identical to mirror for {sym}"
            );
            assert_eq!(
                pinned.pattern().log_eps_meteorite(sym).map(|f| f.to_bits()),
                mirror.pattern().log_eps_meteorite(sym).map(|f| f.to_bits()),
                "pinned meteoritic log-eps is byte-identical to mirror for {sym}"
            );
        }
        // The pin is seed-invariant (no draw consumed): a second seed yields the identical pinned ratio.
        let pinned2 = DiskComposition::draw(&Environment::local_disk_solar_pin(), 0x1111_2222)
            .expect("second pinned draw resolves");
        assert_eq!(pinned2.metallicity_ratio_to_solar(), Some(Fixed::ONE));
    }

    #[test]
    fn two_unpinned_seeds_draw_different_compositions() {
        // THE THESIS SMOKE TEST (steer 1 notes this is ONLY the smoke test; the distributional test above is the
        // load-bearing one). Two UNPINNED world seeds draw DIFFERENT [Fe/H], hence different Z/Z_sun and different
        // scaled patterns, for a DERIVED reason (the MDF draw), not an authored one.
        let env = Environment::local_disk();
        let a = DiskComposition::draw(&env, 1).expect("seed 1 draws");
        let b = DiskComposition::draw(&env, 2).expect("seed 2 draws");
        let za = a.metallicity_ratio_to_solar().expect("seed 1 has a ratio");
        let zb = b.metallicity_ratio_to_solar().expect("seed 2 has a ratio");
        assert_ne!(
            za.to_bits(),
            zb.to_bits(),
            "two unpinned seeds draw different Z/Z_sun (the derived reason worlds diverge)"
        );
        // The scaled patterns differ too: a metal's abundance moved by the draw.
        let fe_a = a.pattern().preferred("Fe").expect("Fe in seed 1");
        let fe_b = b.pattern().preferred("Fe").expect("Fe in seed 2");
        assert_ne!(
            fe_a.to_bits(),
            fe_b.to_bits(),
            "the drawn iron abundance differs between the two seeds"
        );
    }

    #[test]
    fn the_local_disk_mdf_matches_its_verify_on_pull_fingerprint() {
        // VERIFY-ON-PULL: the loaded MDF numbers are FETCHED (Casagrande et al. 2011, A&A 530 A138; Schlesinger et al.
        // 2012, ApJ 761 160): a near-solar peak and ~0.20 dex intrinsic scatter. This fingerprint catches any SILENT
        // edit to those cited numbers: change a constant and the recorded hash breaks, forcing a re-citation. The peak
        // is [Fe/H] = 0 (solar is 0 by definition, so the near-solar peak sits at 0); the scatter is 0.20 dex.
        assert_eq!(
            LOCAL_DISK_FE_H_PEAK_DEX.to_bits(),
            0,
            "the near-solar MDF peak is [Fe/H] = 0"
        );
        let fingerprint = Rng::for_coords(
            0x004D_4446, // "MDF"
            &[
                LOCAL_DISK_FE_H_PEAK_DEX.to_bits() as u64,
                LOCAL_DISK_FE_H_SIGMA_DEX.to_bits() as u64,
            ],
        )
        .key();
        assert_eq!(
            fingerprint, 0x0fec_40cc_bc94_7e30,
            "the fetched MDF numbers match their recorded verify-on-pull fingerprint"
        );
        // The local-disk environment carries the fetched numbers, not a stale copy.
        let env = Environment::local_disk();
        assert_eq!(env.fe_h_mean.to_bits(), LOCAL_DISK_FE_H_PEAK_DEX.to_bits());
        assert_eq!(
            env.fe_h_sigma.to_bits(),
            LOCAL_DISK_FE_H_SIGMA_DEX.to_bits()
        );
    }

    // ── LINK 2: the [alpha/Fe] two-branch alpha knee, conditioned on [Fe/H] ──────────────────────────────────────

    #[test]
    fn the_alpha_ensemble_reproduces_the_two_branch_knee_within_band() {
        // STEER 1, the LOAD-BEARING test: acceptance is DISTRIBUTIONAL. An ensemble of [alpha/Fe] draws conditioned on
        // a LOW [Fe/H] must reproduce the fetched alpha-PLATEAU (~+0.3), and on a HIGH [Fe/H] the DECLINED value (~0),
        // the two ends of the fetched Bensby knee. And at an INTERMEDIATE [Fe/H] the draw is BIMODAL (every draw is one
        // of the two discrete branch levels, never a value in the gap between them): the thick-disk sequence a single
        // Gaussian would erase. A single-Gaussian sampler centred on the mean would fail the bimodality assertion.
        let env = Environment::local_disk();
        let n = 20_000u64;
        let ens_mean = |fe_h: Fixed| -> f64 {
            let mut s = 0.0f64;
            for seed in 0..n {
                s += env.draw_alpha_fe(seed, fe_h).to_f64_lossy();
            }
            s / n as f64
        };
        // Low [Fe/H] (below the knee): all-thick, reproduces the plateau +0.3.
        let low = ens_mean(Fixed::from_ratio(-6, 10));
        assert!(
            (low - 0.30).abs() < 0.01,
            "the low-[Fe/H] ensemble reproduces the fetched alpha plateau +0.3: got {low:.4}"
        );
        // High [Fe/H] (solar): all-thin, the declined value ~0.
        let high = ens_mean(Fixed::ZERO);
        assert!(
            high.abs() < 0.01,
            "the solar-[Fe/H] ensemble reproduces the declined high-iron value ~0: got {high:.4}"
        );
        // Intermediate [Fe/H] = -0.2: BIMODAL. Every draw is either the thick track value or the thin (0) level, and
        // BOTH appear. The thick value at -0.2 is 0.1 + 0.2 * (0.2/0.4) = 0.2.
        let fe_h_mid = Fixed::from_ratio(-2, 10);
        let thick_here = thick_branch_alpha(fe_h_mid).to_f64_lossy();
        let (mut saw_thick, mut saw_thin) = (false, false);
        for seed in 0..n {
            let a = env.draw_alpha_fe(seed, fe_h_mid).to_f64_lossy();
            let is_thick = (a - thick_here).abs() < 1e-9;
            let is_thin = a.abs() < 1e-9;
            assert!(
                is_thick || is_thin,
                "a bimodal draw is one of the two branch levels ({thick_here:.4} or 0), got {a:.6} (a Gaussian would land in the gap)"
            );
            saw_thick |= is_thick;
            saw_thin |= is_thin;
        }
        assert!(
            saw_thick && saw_thin,
            "both branches appear at intermediate [Fe/H] (the measured bimodal gap, not a single spread)"
        );
    }

    #[test]
    fn the_alpha_knee_reconditions_on_fe_h() {
        // STEER 3: the knee is REAL and conditioned, not a constant. The drawn [alpha/Fe] ensemble mean must be HIGH at
        // low [Fe/H] and LOW at high [Fe/H] (it turns over through the knee). If [alpha/Fe] ignored [Fe/H] the two
        // ensemble means would coincide; the assertion that low >> high is what proves the conditioning.
        let env = Environment::local_disk();
        let n = 12_000u64;
        let ens_mean = |fe_h: Fixed| -> f64 {
            let mut s = 0.0f64;
            for seed in 0..n {
                s += env.draw_alpha_fe(seed, fe_h).to_f64_lossy();
            }
            s / n as f64
        };
        let low = ens_mean(Fixed::from_ratio(-6, 10)); // below the knee: alpha-enhanced
        let high = ens_mean(Fixed::from_ratio(-1, 20)); // -0.05, near solar: alpha-poor
        assert!(
            low > high + 0.2,
            "the [alpha/Fe] ensemble mean is HIGH at low [Fe/H] ({low:.4}) and LOW near solar ({high:.4}): the knee re-conditions"
        );
    }

    #[test]
    fn adding_a_later_link_never_shifts_the_drawn_alpha_fe_for_a_seed() {
        // STEER 2 (the reverse invariant): a seed's [alpha/Fe] is STABLE when a still-later link (its own named slot)
        // is added. Simulate landing an unrelated later link (C/O) by consuming a draw from ITS slot, and assert
        // [alpha/Fe] is bit-identical with and without that consumption, because each link is a pure function of the
        // seed and its own name-key with no shared cursor. Also assert the alpha slot keys distinctly from [Fe/H] and
        // C/O, so no link can alias another.
        let env = Environment::local_disk();
        let seed = 0x0fed_cba9_8765_4321u64;
        let fe_h = env.draw_fe_h(seed);
        let alpha_alone = env.draw_alpha_fe(seed, fe_h);
        let co_slot = Rng::for_coords(seed, &[link_slot_key("c_over_o")]);
        let _later = gaussian(&co_slot, 0, Fixed::ZERO, Fixed::ONE, FE_H_GAUSS_METHOD);
        let alpha_after = env.draw_alpha_fe(seed, fe_h);
        assert_eq!(
            alpha_alone.to_bits(),
            alpha_after.to_bits(),
            "the [alpha/Fe] realization is bit-identical whether or not a later link's slot is consumed"
        );
        assert_ne!(
            link_slot_key(LINK_ALPHA_FE),
            link_slot_key(LINK_FE_H),
            "the alpha slot keys distinctly from the [Fe/H] slot"
        );
        assert_ne!(
            link_slot_key(LINK_ALPHA_FE),
            link_slot_key("c_over_o"),
            "the alpha slot keys distinctly from the C/O slot"
        );
    }

    #[test]
    fn the_mirror_pins_the_alpha_link_to_solar_zero_byte_identical() {
        // STEER 4: the Mirror pins the alpha link too. The solar-pin environment returns [alpha/Fe] = 0 exactly (no
        // draw consumed, seed-invariant), so the alpha scaling is +0 and the drawn pattern is byte-identical to the
        // unshifted solar pattern on every element. The already-present chain-pin test proves the whole datum is
        // byte-identical to mirror(); this isolates the alpha link's pin.
        let pin = Environment::local_disk_solar_pin();
        assert_eq!(
            pin.draw_alpha_fe(0xDEAD_BEEF, Fixed::ZERO).to_bits(),
            0,
            "the solar pin returns [alpha/Fe] = 0 exactly"
        );
        assert_eq!(
            pin.draw_alpha_fe(0x1234_5678, Fixed::from_ratio(-6, 10))
                .to_bits(),
            0,
            "the pin is seed- and [Fe/H]-invariant (no draw consumed): still exactly 0"
        );
        // The pinned draw's pattern is byte-identical to the unshifted solar pattern on every element (the +0 alpha
        // shift is exact), the alpha-link half of the full byte-identity the run pins prove end to end.
        let pinned = DiskComposition::draw(&pin, 0xABCD).expect("the pinned draw resolves");
        let solar = SolarAbundances::standard().expect("solar loads");
        for sym in solar.elements() {
            assert_eq!(
                pinned
                    .pattern()
                    .log_eps_photosphere(sym)
                    .map(|f| f.to_bits()),
                solar.log_eps_photosphere(sym).map(|f| f.to_bits()),
                "pinned photospheric log-eps is byte-identical to solar for {sym} (alpha shift +0)"
            );
        }
    }

    #[test]
    fn the_alpha_knee_matches_its_verify_on_pull_fingerprint() {
        // VERIFY-ON-PULL: the loaded alpha-knee numbers are FETCHED (Bensby, Feltzing & Oey 2014, A&A 562 A71,
        // arXiv 1309.2631): the plateau +0.3, the knee metallicity -0.4, the thick-at-solar +0.1, and the thin/solar
        // level 0. This fingerprint catches any SILENT edit to those cited numbers: change a constant and the recorded
        // hash breaks, forcing a re-citation.
        assert_eq!(
            ALPHA_THIN_DEX.to_bits(),
            0,
            "the thin/solar alpha level is 0"
        );
        let fingerprint = Rng::for_coords(
            0x414C_5041, // "ALPA"
            &[
                ALPHA_PLATEAU_DEX.to_bits() as u64,
                ALPHA_KNEE_FE_H_DEX.to_bits() as u64,
                ALPHA_THICK_AT_SOLAR_DEX.to_bits() as u64,
                ALPHA_THIN_DEX.to_bits() as u64,
            ],
        )
        .key();
        assert_eq!(
            fingerprint, 0xe8ac_feb8_792c_8ace,
            "the fetched alpha-knee numbers match their recorded verify-on-pull fingerprint"
        );
    }
}
