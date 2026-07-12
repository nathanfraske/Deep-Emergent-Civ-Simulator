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

//! Derived resting metabolism and body-to-medium thermal exchange (design Part 15, Part 20, Part 35,
//! Part 41; R-METABOLIZE; Principles 3, 9, 11). The substrate that frees the authored
//! `base_metabolic_drain`, `exertion_drain_coupling`, and `field.body_exchange` scalars: the drain a
//! body pays at rest and the rate its core temperature couples to a medium DERIVE from the body's mass
//! and tissue against the physics floor, never a per-axis authored number.
//!
//! What is authored is physics. The resting metabolic power is Kleiber's law `P = a * m^(3/4)`
//! ([`civsim_physics::laws::basal_metabolic_rate`]): the 3/4 exponent is a universal physics affordance
//! (West, Brown, and Enquist's fractal-network derivation, holding across taxa; Principle 9 permits
//! authored physics), evaluated by the exact two-square-root fixed-point identity so no exp/ln is
//! touched. The thermoregulatory replacement is the resolved convective and radiant heat-loss over the
//! body's exposed surface ([`civsim_physics::laws::resting_heat_loss`]). The body-to-medium coupling is
//! `h * A / (m * c)`, the discrete Newton-cooling rate from the medium coefficient, the surface area,
//! and the body's thermal mass.
//!
//! What is not authored is the outcome. A being's exchange area, thermal mass, and reserve energy are
//! composition-derived reads over its organs (the same development-weighted-sum shape the metabolic
//! reserves and the respiratory surface use, [`crate::homeostasis`], [`crate::medium`]), so two bodies
//! diverge in their derived drain and coupling from their composition, mass, medium, and temperature
//! alone. Nothing here reads a race identity: a hot-set-point body in a cold medium and its mirror
//! differ because their temperatures differ, not because of a label (Principle 9).
//!
//! Everything is integer, fixed-point, and draws no randomness (Principle 3). The owner anchors (the
//! Kleiber coefficient `a`, the normalized-body-mass-to-kilograms bridge, and the Stefan-Boltzmann
//! constant) are reserved with their basis and are the owner's to set ([`MetabolicAnchors::from_manifest`]);
//! the values in [`MetabolicAnchors::dev_fixture`] are labelled development fixtures, never owner canon. Two
//! surface properties the radiant and convective terms once carried as global anchors now read from the
//! being's own data rather than one scalar everywhere (derive-vs-author, Principle 6), with DIFFERENT honest
//! statuses. The radiant-surface emissivity fully DERIVES: it is a material property of the being's covering,
//! read from its `opt.emissivity` axis ([`covering_emissivity`]). The medium convective coefficient `h` is a
//! partial step: it is READ from the being's occupied medium (`fluid.convective_coefficient`, at the being's
//! cell, [`crate::medium::MediumField::convective_at`]), retiring the global scalar in favour of per-medium
//! DATA, but `h` is NOT an irreducible medium property, it is flow-dependent (`h = Nu*k/L`, the Nusselt
//! boundary-layer relation), so the per-medium value is a LUMPED interim and the full derive of `h` from the
//! medium's k/rho/c and the flow past the being's surface is DEFERRED (the reviewer-approved interim). HONEST
//! LIMIT (the two `h` consumers are not yet spatially consistent): the resting thermoregulatory DRAIN reads
//! `h` at the being's CURRENT cell every tick ([`base_drain_from`] via [`being_derived_drains`]), while the
//! body-temperature Newton-cooling exchange RATE caches `h` at the being's SPAWN cell
//! ([`crate::runner::walker_exchange_rate`], stored once per being). Under a spatially-uniform medium (the
//! current dev fixtures carry a uniform `h`) they agree; on a world with spatially-varying `h` a being that
//! moves between media has a current-cell drain and a stale spawn-cell exchange rate until the rate is
//! recomputed, which is deferred with the same per-cell-flow work. The caps below are representability bounds
//! forced by Q32.32 (the engine-mechanics exemption the law kernels and `medium.rs` take), not owner realism
//! values.
//!
//! Two honest limits stand. First, the exact reconciliation of the reserve's stored energy (the biology
//! floor's `bio.energy_density` in kJ/g, the reserve capacity, and the body mass) to joules comparable
//! with a watt-tick spend is the R-UNITS-PIN owner units bridge: the mechanism derives the drain, the
//! absolute scale is the owner's anchors and the floor's units. Second, and this is a genuine
//! cross-tier difference rather than a defect, the base drain is NONLINEAR in mass (`m^(3/4)`) and in
//! temperature (`T^4` through the radiant loss), so the drain of a pool over a size-and-temperature
//! distribution is NOT the drain of the mean size (a Jensen gap): a coarse aggregate tier that reads
//! the mean body loses the convexity. This is the honest cross-tier difference `docs/design.md:2803`
//! already declares for a nonlinear law output over additive quantities. Do NOT silently substitute the
//! mean; the R-TIER-CONSIST reconciliation (carry a size-distribution moment into the pool-tier drain,
//! or accept and document the gap) is the named follow-on, not resolved here.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::Fixed;
use civsim_physics::laws;

use crate::anatomy::{BodyPlan, BodyPlanRegistry};
use crate::calibration::{CalibrationError, CalibrationManifest};

/// The biology-floor axis a tissue carries its body-to-medium exchange surface on
/// (`crates/physics/data/biology_floor.toml`), the area the heat-loss and coupling laws integrate over.
/// A tissue with none of it presents no exchange surface (the absence convention).
pub const CONVECTIVE_SURFACE: &str = "bio.convective_surface";
/// The mechanical-floor axis a tissue carries its density on (`mat.density`, kg/m^3), reused from
/// [`crate::medium`].
pub const TISSUE_DENSITY: &str = "mat.density";
/// The mechanical-floor axis a tissue carries its specific heat on (`therm.specific_heat`, J/(kg*K)),
/// the per-unit-mass heat capacity the body's thermal mass reads.
pub const TISSUE_SPECIFIC_HEAT: &str = "therm.specific_heat";
/// The biology-floor axis a tissue carries its gross energy density on (`bio.energy_density`), the
/// reserve's per-unit specific energy.
pub const ENERGY_DENSITY: &str = "bio.energy_density";
/// The biology-floor axis a tissue (or a standing fluid supply) carries its water content on
/// (`bio.water_fraction`), the hydration reserve's backing class. Named here as a shared constant so
/// the environmental water-source writer and the metabolic water reserve agree on the class id; it is
/// a data label like [`ENERGY_DENSITY`], not a special case (a world's alien fluid is another class).
pub const WATER_FRACTION: &str = "bio.water_fraction";
/// The biology-floor TOXIN class a cell carries its salinity dose on (`bio.salinity`), the class the
/// environmental salinity field doses a cell with and a being's heritable salt tolerance is read
/// against (base-level liveliness step 4). A data label like [`ENERGY_DENSITY`], not a special case: a
/// world's dust, alkalinity, or arcane taint is another toxin class the same harm path reads.
pub const SALINITY: &str = "bio.salinity";
/// The mechanical-floor axis a tissue carries its material strength on (`mat.fracture_strength`, the
/// same axis the individual-tier [`crate::body::Body::strength`] reads, design Part 35), the strength
/// per unit of the tissue the whole-body work force integrates over. A tissue with none of it provides
/// no muscle force (the absence convention).
pub const MUSCLE_STRENGTH: &str = "mat.fracture_strength";
/// The chem/optics-floor axis a covering carries its SURFACE emissivity on (`opt.emissivity`, the
/// radiant-exchange fraction, dimensionless in [0, 1]; `crates/physics/data/chem_optics_floor.toml`).
/// The radiant thermoregulatory term reads the being's covering value on this axis
/// ([`covering_emissivity`]) rather than a duplicate global scalar; a covering with none of it radiates
/// nothing (the absence convention, so an alien body converges on no hidden terran default; Principle 9).
pub const OPT_EMISSIVITY: &str = "opt.emissivity";

/// A representability cap for the basal metabolic rate (W). Engine-mechanics bound, not an owner value.
const RATE_MAX: Fixed = Fixed::from_int(1_000_000_000);
/// A representability cap for the thermoregulatory heat-loss flux (W). Engine-mechanics bound.
const FLUX_MAX: Fixed = Fixed::from_int(1_000_000_000);
/// A representability cap for the mechanical work power (W, the `laws::power_watts` scale, matching the
/// watt-scale basal rate the exertion coupling is summed with). Engine-mechanics.
const POWER_MAX: Fixed = Fixed::from_int(1_000_000_000);
/// The drain-fraction cap: a reserve cannot lose more than its whole capacity in one tick, so the
/// derived fraction is bounded to one. A physical bound, not an owner value.
const FRAC_MAX: Fixed = Fixed::ONE;

/// The reserved owner anchors the derived metabolism needs, surfaced with their basis and fail-loud in
/// the manifest, never fabricated (Principle 11). The kernels are fixed Rust; these are the owner's to
/// set. Read on a canonical run through [`MetabolicAnchors::from_manifest`]; the dev fixture is a
/// labelled test stand-in.
// @derives[metabolic_rate]: a being's metabolic rate, energy drain, and heat loss <- Kleiber's law P = a * m^(3/4) over the body's own mass (kleiber_a and body_mass_kg_scale are per-race anchors, sigma is a universal constant); the rate is NOT authored, it derives from the being's body. Water loss derives from 1/L_vap (latent heat of vaporization) x metabolic power (physiology water-loss coupling, landed with the Mirror water arc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetabolicAnchors {
    /// The Kleiber coefficient `a` in `P = a * m^(3/4)` (W per kg^(3/4)). RESERVED owner anchor.
    pub kleiber_a: Fixed,
    /// The kilograms a body carries at `body_mass = 1` (the normalized-trait-to-kilograms bridge). An
    /// R-UNITS-PIN bridge, NOT derivable. RESERVED owner anchor.
    pub body_mass_kg_scale: Fixed,
    /// The Stefan-Boltzmann constant sigma (W/(m^2*K^4)), a universal physical constant. DERIVED from the
    /// CODATA fundamentals ([`derived_stefan_boltzmann`]), not an authored decimal and not a reserved
    /// manifest value: the same in every profile, computed once at load.
    pub sigma: Fixed,
    /// The same Stefan-Boltzmann sigma at its FULL derived scale (the fine `(bits, scale)`), which the Tier-2
    /// radiant heat-loss lift consumes so sigma enters at full precision rather than the Q32.32 truncation
    /// [`sigma`](Self::sigma) carries. Derived from the same fundamentals, never authored.
    pub sigma_fine_bits: i64,
    pub sigma_fine_scale: u32,
}

/// The global significance target and guard for the composite-constant fixed-point scale derivation
/// (R-UNITS-PIN). They set the INTERMEDIATE per-quantity scale a composite is derived at. At the shipped
/// values (30, 1) the value the sim consumes (sigma at Q32.32) is invariant to them, locked by the test
/// `derived_stefan_boltzmann_is_the_expected_q32_bits`; they are fixed-point REPRESENTATION knobs (the family
/// of the canonical `FRAC_BITS`), not world values. The invariance is NOT universal, though: an extreme
/// retune (a large guard, or a significance target that drives the intermediate scale below the canonical
/// bits) can perturb the consumed value, so a retune MUST re-verify the four run_world pins rather than
/// treat these as inconsequential. Surfaced for the owner (reserved-with-basis in the decisions log). Basis:
/// resolve a CODATA composite's ~10 significant figures. NOW LIVE (the previously-flagged follow-on is
/// realized, R-UNITS-PIN slice 4): [`derived_stefan_boltzmann_fine`] consumes sigma at its fine scale in the
/// Tier-2 radiant lift, so these knobs set how many significant bits of sigma reach the radiant term (the
/// fine-scale sigma differs from the old Q32.32 value, ~0.19% at body temperatures). They still sit inside the
/// Principle-11 representation exemption (they set how precisely the exact derived sigma is stored, not WHAT
/// sigma is), and the four pins were re-verified byte-neutral after the lift because that fidelity change
/// stays below the downstream discretization; a retune MUST re-verify the pins, now doubly so.
const COMPOSITE_SIG_TARGET: u32 = 30;
const COMPOSITE_GUARD_BITS: u32 = 1;

/// The Stefan-Boltzmann constant sigma DERIVED from the fundamentals, never an authored decimal: the
/// units-crate composite compute evaluates `2*pi^5*k_B^4/(15*h^3*c^2)` EXACTLY as a rational (pi by a
/// deterministic series, no float) and rounds ONCE to sigma's derived scale, and this projects it to the
/// sim's Q32.32 `Fixed`. A one-time, float-free, deterministic load computation, so it perturbs no canonical
/// result; every sigma anchor reads this one derivation. Memoized because the value is a pure constant.
pub fn derived_stefan_boltzmann() -> Fixed {
    use std::sync::OnceLock;
    static SIGMA: OnceLock<Fixed> = OnceLock::new();
    *SIGMA.get_or_init(|| {
        let (bits, scale) = derived_stefan_boltzmann_fine();
        let q32 = civsim_units::rescale_bits(bits, scale, Fixed::FRAC_BITS)
            .expect("sigma rescale to Q32.32 must not overflow");
        Fixed::from_bits(q32)
    })
}

/// The Stefan-Boltzmann sigma at its FULL derived scale, the fine `(bits, scale)` pair the units composite
/// compute produces BEFORE the Q32.32 projection [`derived_stefan_boltzmann`] applies. This is the value the
/// Tier-2 radiant lift ([`civsim_physics::laws::radiant_emission_tier2`]) consumes, so sigma enters the
/// radiant heat-loss term at its ~31-bit mantissa rather than the roughly eight-bit Q32.32 truncation. A
/// pure, float-free, memoized load constant, the same derivation [`derived_stefan_boltzmann`] reads.
pub fn derived_stefan_boltzmann_fine() -> (i64, u32) {
    use std::sync::OnceLock;
    static SIGMA_FINE: OnceLock<(i64, u32)> = OnceLock::new();
    *SIGMA_FINE.get_or_init(|| {
        let (bits, scale) = civsim_units::compute::derived_composite_bits(
            &civsim_units::fundamentals::STEFAN_BOLTZMANN,
            COMPOSITE_SIG_TARGET,
            COMPOSITE_GUARD_BITS,
            Fixed::FRAC_BITS,
        )
        .expect("the Stefan-Boltzmann sigma must derive from the fundamentals");
        (
            i64::try_from(bits).expect("sigma at its derived scale fits i64"),
            scale,
        )
    })
}

impl MetabolicAnchors {
    /// The anchors read from the calibration manifest, fail-loud if any is still reserved (Principle 11,
    /// the reserved-value discipline). This is the sanctioned way to obtain the anchors on a canonical
    /// run; there is no default, so an unset value refuses to run rather than fabricating a number. Sigma is
    /// no longer read here: it DERIVES from the fundamentals ([`derived_stefan_boltzmann`]), the same in
    /// every profile, so it is never a reserved manifest value.
    pub fn from_manifest(
        manifest: &CalibrationManifest,
    ) -> Result<MetabolicAnchors, CalibrationError> {
        let (sigma_fine_bits, sigma_fine_scale) = derived_stefan_boltzmann_fine();
        Ok(MetabolicAnchors {
            kleiber_a: manifest.require_fixed("metabolism.kleiber_coefficient")?,
            body_mass_kg_scale: manifest.require_fixed("metabolism.body_mass_kg_scale")?,
            sigma: derived_stefan_boltzmann(),
            sigma_fine_bits,
            sigma_fine_scale,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE, not owner canon: a plausible temperate-mammal Kleiber coefficient,
    /// a mid-size kilogram bridge, and the CODATA Stefan-Boltzmann constant. Two surface properties are no
    /// longer anchors: the radiant term's emissivity derives from the being's covering
    /// ([`covering_emissivity`]) and the medium convective coefficient `h` from the being's medium
    /// ([`crate::medium::MediumField::convective_at`]). For tests and examples only; a canonical run reads
    /// [`MetabolicAnchors::from_manifest`].
    pub fn dev_fixture() -> MetabolicAnchors {
        let (sigma_fine_bits, sigma_fine_scale) = derived_stefan_boltzmann_fine();
        MetabolicAnchors {
            kleiber_a: Fixed::from_ratio(1, 100),
            body_mass_kg_scale: Fixed::from_int(100),
            sigma: derived_stefan_boltzmann(),
            sigma_fine_bits,
            sigma_fine_scale,
        }
    }
}

/// A being's whole-body convective exchange area: the development-weighted sum over its organs of their
/// `bio.convective_surface` composition. The same composition-derived shape [`crate::medium::exchange_area`]
/// uses for the respiratory surface, so a body's ability to shed heat follows its anatomy: a body with no
/// exchange-surface tissue presents zero area and couples to no medium convectively. The sum is the
/// order-independent [`Fixed::saturating_add`], so it is invariant to organ order.
pub fn whole_body_surface(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut sum = Fixed::ZERO;
    for organ in &plan.organs {
        let surface = organs
            .organ_composition(organ.kind)
            .map(|c| c.component(CONVECTIVE_SURFACE))
            .unwrap_or(Fixed::ZERO);
        let area = organ
            .development
            .checked_mul(surface)
            .unwrap_or(Fixed::ZERO);
        sum = sum.saturating_add(area);
    }
    sum
}

/// A being's whole-body muscle work force (design Part 35, real-world unification step 5): the
/// development-weighted sum over its organs of their `mat.fracture_strength` composition, times the
/// body's mass in kilograms, mirroring the individual-tier [`crate::body::Body::strength`] (muscle mass
/// times material strength) at the body-plan tier so the two tiers stay dimensionally consistent (owner
/// ruling 2026-07-04). This replaces the raw `body_mass` proxy the exertion coupling read: a body's
/// exertion drain now follows its actual muscle endowment scaled by its size, so two bodies of equal mass
/// but different muscle composition exert different force, and two bodies of equal muscle composition but
/// different mass exert different force too (the mass scaling the raw proxy carried is kept, not dropped).
/// It reads the composition axis, never a specific tissue-material id or a race label (Principle 9): a
/// body whose tissue declares no strength reads ZERO (the absence convention its siblings use), not a
/// mass-sized default, so the exertion coupling falls to its no-force branch rather than a hidden proxy.
/// The sum is the order-independent [`Fixed::saturating_add`], so it is invariant to organ order, and the
/// mass bridge is the reserved kilogram scale the metabolic derivations already read. HONEST LIMIT (the
/// shared Q32.32 convention, surfaced by the blind audit): the final mass multiply routes an overflow to
/// ZERO (`checked_mul(..).unwrap_or(ZERO)`), the same degenerate-input convention [`body_mass_kg`] and the
/// individual-tier [`crate::body::Body::strength`] use, so a body whose Pa-scale muscle strength times a
/// giant body mass exceeds the representable range reads zero force rather than saturating. This never
/// binds at the dev-fixture magnitudes; when the owner sets real fracture strengths it is a calibration
/// concern to keep the product in range, and any move to a saturating convention must change both tiers
/// together (the ruling's tier-consistency), not this one alone.
pub fn whole_body_muscle_force(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let mut sum = Fixed::ZERO;
    for organ in &plan.organs {
        let strength = organs
            .organ_composition(organ.kind)
            .map(|c| c.component(MUSCLE_STRENGTH))
            .unwrap_or(Fixed::ZERO);
        let force = organ
            .development
            .checked_mul(strength)
            .unwrap_or(Fixed::ZERO);
        sum = sum.saturating_add(force);
    }
    sum.checked_mul(body_mass_kg(plan, anchors))
        .unwrap_or(Fixed::ZERO)
}

/// A being's whole-body specific heat (J/(kg*K)): the development-weighted average over its organs of
/// their `therm.specific_heat` composition, or ZERO if no organ declares one (the absence convention
/// its siblings [`whole_body_surface`] and [`whole_body_energy_density`] use). The same
/// composition-average shape [`crate::medium::body_density`] uses, so the body's thermal mass follows
/// its tissue rather than a hidden terran-water default: a body whose tissue carries no specific heat
/// has no defined thermal mass, and the body-to-medium coupling then falls to its own
/// no-thermal-mass branch ([`derive_body_exchange_rate`]) rather than converging on the specific heat
/// of water (Principle 9: no terran constant on the content path). Order-independent (saturating
/// sums, one checked division).
pub fn whole_body_specific_heat(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut weighted = Fixed::ZERO;
    let mut total_dev = Fixed::ZERO;
    for organ in &plan.organs {
        let c = organs
            .organ_composition(organ.kind)
            .map(|comp| comp.component(TISSUE_SPECIFIC_HEAT))
            .unwrap_or(Fixed::ZERO);
        if c > Fixed::ZERO {
            let contribution = organ.development.checked_mul(c).unwrap_or(Fixed::ZERO);
            weighted = weighted.saturating_add(contribution);
            total_dev = total_dev.saturating_add(organ.development);
        }
    }
    if total_dev <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    weighted.checked_div(total_dev).unwrap_or(Fixed::ZERO)
}

/// A being's whole-body energy density: the development-weighted average over its organs of their
/// `bio.energy_density` composition, the reserve's per-unit specific energy the drain bridge multiplies
/// by the reserve capacity to reach the stored joules. A body with no energy-dense tissue reads zero
/// (no stored energy, so the resting demand drains its reserve fully, the no-energy-organ death the
/// physiology already models). Order-independent.
pub fn whole_body_energy_density(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut weighted = Fixed::ZERO;
    let mut total_dev = Fixed::ZERO;
    for organ in &plan.organs {
        let d = organs
            .organ_composition(organ.kind)
            .map(|comp| comp.component(ENERGY_DENSITY))
            .unwrap_or(Fixed::ZERO);
        if d > Fixed::ZERO {
            let contribution = organ.development.checked_mul(d).unwrap_or(Fixed::ZERO);
            weighted = weighted.saturating_add(contribution);
            total_dev = total_dev.saturating_add(organ.development);
        }
    }
    if total_dev <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    weighted.checked_div(total_dev).unwrap_or(Fixed::ZERO)
}

/// A being's whole-body water density: the development-weighted average over its organs of their
/// `bio.water_fraction` composition, the hydration reserve's per-unit water content the water-loss bridge
/// multiplies by the reserve capacity to reach the stored water mass. The water sibling of
/// [`whole_body_energy_density`], read on the same floor axis ([`WATER_FRACTION`]); a body with no
/// water-bearing tissue reads zero. Order-independent.
pub fn whole_body_water_density(plan: &BodyPlan, organs: &BodyPlanRegistry) -> Fixed {
    let mut weighted = Fixed::ZERO;
    let mut total_dev = Fixed::ZERO;
    for organ in &plan.organs {
        let d = organs
            .organ_composition(organ.kind)
            .map(|comp| comp.component(WATER_FRACTION))
            .unwrap_or(Fixed::ZERO);
        if d > Fixed::ZERO {
            let contribution = organ.development.checked_mul(d).unwrap_or(Fixed::ZERO);
            weighted = weighted.saturating_add(contribution);
            total_dev = total_dev.saturating_add(organ.development);
        }
    }
    if total_dev <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    weighted.checked_div(total_dev).unwrap_or(Fixed::ZERO)
}

/// A being's whole-body COMPOSITION VECTOR: its value on the UNION of every floor axis any of its parts
/// declares, each a development-weighted mean over the parts that carry it, generalizing
/// [`whole_body_energy_density`] and [`crate::medium::body_density`] from one named axis to all of them. This
/// is the physics of the matter a body is made of: the vector a corpse deposits into the tissue field
/// ([`crate::material::TissueField`]) so the world can forage, work, and decompose an organism's remains by
/// the SAME axes and mechanisms as any other matter, with no minted per-species substance and no authored
/// species-to-substance map (Principle 8). The organs read their [`crate::anatomy::TissueComposition`] via
/// `organ_composition`; the covering and each weapon read their `KindDef::material` DIRECTLY, because
/// `organ_composition` searches only the organ list and a covering or weapon kind id would otherwise alias
/// onto an unrelated organ sharing that numeric id. Locomotion (a bare kind-id vector with no development
/// scalar, `BodyPlan::locomotion`) is excluded here; its weighting is a reserved design choice. On an
/// organs-only body its `mat.density` component equals [`crate::medium::body_density`] exactly (the
/// special-case the generalization subsumes). Order-independent, no RNG; an axis no part carries is absent
/// (the substrate's zero-for-absent convention), so a consumer that needs a floor for an axis (as
/// `body_density` applies a water baseline) applies its own.
pub fn whole_body_composition_vector(
    plan: &BodyPlan,
    registry: &BodyPlanRegistry,
) -> BTreeMap<String, Fixed> {
    // Each part's (development weight, its own axis map), gathered so the axis union and the per-axis
    // weighted mean read the SAME source. Organs read their tissue composition; the covering and weapons
    // read their material map directly (organ_composition would alias their kind id onto an organ). The
    // CONTRIBUTOR SET is deliberately organs, covering, and weapons only: senses and locomotion are NOT
    // contributors. This is load-bearing for what the corpse deposits: the senses carry optical axes
    // (`opt.refractive_index`, `crate::anatomy` sense kinds), and were they added as contributors those
    // axes would enter this vector. The `opt.*` axis-union skip below already keeps optical axes out of the
    // deposited matter, so senses contribute nothing even if added, but a future change to the contributor
    // set (or a new non-optical axis on a sense) must be conscious of this coupling rather than break the
    // matter vector silently.
    let mut contributors: Vec<(Fixed, &BTreeMap<String, Fixed>)> = Vec::new();
    for organ in &plan.organs {
        if let Some(comp) = registry.organ_composition(organ.kind) {
            contributors.push((organ.development, &comp.components));
        }
    }
    if let Some(cov) = registry
        .coverings
        .iter()
        .find(|k| k.id == plan.covering.kind)
    {
        contributors.push((plan.covering.development, &cov.material));
    }
    for weapon in &plan.weapons {
        if let Some(kd) = registry.weapons.iter().find(|k| k.id == weapon.kind) {
            contributors.push((weapon.development, &kd.material));
        }
    }
    // The axis union over every contributor. The deposited vector is the body's MATTER (the mechanical,
    // thermal, and biological composition the world forages, works, and decomposes); an optical SURFACE
    // coefficient (`opt.*`, such as a covering's `opt.emissivity`, read at the radiating surface by the
    // metabolism, not depositable bulk matter) is excluded, keeping the matter vector free of the optical
    // axes senses and coverings carry, the same way senses are excluded as contributors above.
    let mut axes: BTreeSet<&str> = BTreeSet::new();
    for (_, map) in &contributors {
        for key in map.keys() {
            // Exclude an optical SURFACE coefficient (`opt.*`, a covering's emissivity) as before, AND
            // `mat.fracture_energy`: it is a material RESISTANCE PROPERTY (energy per crack area) the wound
            // law reads off a body's OWN outermost material (the covering) for its Griffith tolerance
            // (predation-integration slice), not a depositable bulk-matter quantity the world forages or
            // decomposes, so it belongs to the wound read and not the corpse's matter vector, exactly as a
            // surface coefficient does. No existing contributor carries it (weapons carry
            // `mat.indentation_hardness`, organs carry `bio.*`), so this skip is byte-identical for every
            // existing body; it keeps the covering's new fracture-energy out of the deposited matter. (Whether
            // the other mechanical `mat.*` resistances, e.g. `mat.indentation_hardness`, should likewise be
            // excluded is a pre-existing consistency question flagged, not changed here.)
            if key.starts_with("opt.") || key.as_str() == "mat.fracture_energy" {
                continue;
            }
            axes.insert(key.as_str());
        }
    }
    // Per axis, the development-weighted mean over the contributors that carry it (value > 0), the same
    // discipline body_density and whole_body_energy_density use; an axis no contributor carries is absent.
    let mut vector: BTreeMap<String, Fixed> = BTreeMap::new();
    for axis in axes {
        let mut weighted = Fixed::ZERO;
        let mut total_dev = Fixed::ZERO;
        for (dev, map) in &contributors {
            let v = map.get(axis).copied().unwrap_or(Fixed::ZERO);
            if v > Fixed::ZERO {
                let contribution = dev.checked_mul(v).unwrap_or(Fixed::ZERO);
                weighted = weighted.saturating_add(contribution);
                total_dev = total_dev.saturating_add(*dev);
            }
        }
        if total_dev > Fixed::ZERO {
            let mean = weighted.checked_div(total_dev).unwrap_or(Fixed::ZERO);
            if mean > Fixed::ZERO {
                vector.insert(axis.to_string(), mean);
            }
        }
    }
    vector
}

/// The body's mass in kilograms: the normalized `body_mass` trait times the reserved kilogram bridge.
/// An overflowing product routes to zero (an unrepresentably huge body has no meaningful metabolism
/// here), matching the law kernels' degenerate-input convention.
pub fn body_mass_kg(plan: &BodyPlan, anchors: &MetabolicAnchors) -> Fixed {
    plan.body_mass
        .checked_mul(anchors.body_mass_kg_scale)
        .unwrap_or(Fixed::ZERO)
}

/// A being's radiating-surface value on ONE optical floor axis: the value its covering material declares on
/// the axis `axis` (`opt.emissivity`, `opt.refractive_index`, `opt.albedo`, whatever the chem/optics floor
/// declares), or ZERO if the covering (or the being) carries none (the substrate absence convention). This is
/// the general surface-keyed DIRECT read of a SINGLE floor optical axis: it reads exactly one axis, never a
/// composite of several (a composite would be an authored value in the world-content path, the value-authoring
/// line; combination across axes belongs to selection over per-axis weights, never a fold here), and it keys on
/// the being's OWN surface material (its covering, resolved by `plan.covering.kind` against the registry
/// coverings, the same way [`covering_emissivity`] and [`whole_body_composition_vector`] do), so a being whose
/// covering declares no value for the axis, or has no covering at all, reads ZERO and simply carries no feature
/// on that axis rather than a synthesized default (admit-the-alien: the alien is a data row, Principle 9). The
/// perceivable-feature substrate ([`crate::perceivable_feature::PerceivableFeatureRegistry`]) reads this per
/// declared channel. Pure and RNG-free.
pub fn surface_optical_axis(plan: &BodyPlan, registry: &BodyPlanRegistry, axis: &str) -> Fixed {
    registry
        .coverings
        .iter()
        .find(|k| k.id == plan.covering.kind)
        .map(|cov| cov.mat(axis))
        .unwrap_or(Fixed::ZERO)
}

/// A being's radiating-surface emissivity: its surface value on the `opt.emissivity` optical floor axis
/// ([`OPT_EMISSIVITY`]), the [`surface_optical_axis`] read on that one axis, or ZERO if the covering carries
/// none (the substrate absence convention, so an alien body whose covering declares no emissivity radiates
/// nothing rather than converging on a hidden terran default; Principle 9). The radiant thermoregulatory term
/// reads THIS rather than a global manifest scalar, so the emissivity is the being's OWN covering-material datum,
/// per-race differentiable and read from the same covering the corpse deposit carries, not a duplicate constant
/// (derive-vs-author, Principle 6; the retired `metabolism.surface_emissivity` duplicated this floor axis).
pub fn covering_emissivity(plan: &BodyPlan, registry: &BodyPlanRegistry) -> Fixed {
    surface_optical_axis(plan, registry, OPT_EMISSIVITY)
}

/// The perceptible optical SIGNAL power a being emits from its own body heat (the being-percept keystone,
/// step 6, the emitter side): its blackbody radiant flux off its body temperature
/// ([`civsim_physics::laws::radiant_emission`], the same Stefan-Boltzmann law the thermoregulation term
/// reads) scaled by a reserved per-being emission `coefficient`. Derived from the being's OWN body
/// temperature, never an authored per-species signature: a warmer body emits a stronger thermal signature
/// its perceivers sense, a body at absolute zero emits nothing, and a cold or photosynthetic alien differs
/// by its own temperature, not by kingdom (Principle 9, admit-the-alien). `t_cold` is zero because the
/// SIGNAL is the being's ABSOLUTE radiance (what a perceiver's sensorium transduces), unlike the net thermal
/// balance against ambient the resting heat-loss path ([`base_drain_from`]) computes.
///
/// The `coefficient` is RESERVED (the being-percept feature's own calibration, surfaced with basis, never
/// fabricated): its basis is the body's covering emissivity times its radiating area, folded into one lever
/// until a per-body material-and-area vector exists on the run path to split it into emissivity times area
/// (the gate-ruled follow-on). It enters as the emissivity-times-area term of `radiant_emission`, so the
/// [`FLUX_MAX`] representability cap applies to the final emission, matching the ruled
/// `radiant_emission(body_temp) * coefficient` (emissivity is that law's linear scale, so folding the
/// coefficient into it is that product with the cap correctly on the result). Pure and RNG-free.
pub fn being_signal_emission(
    body_temp: Fixed,
    coefficient: Fixed,
    sigma_bits: i64,
    sigma_scale: u32,
) -> Fixed {
    // Sigma at its full derived scale (the Tier-2 lift, R-UNITS-PIN slice 4): the perceived thermal signal
    // `sigma * body_temp^4 * coefficient` now carries sigma at full precision instead of the Q32.32 truncation,
    // its `sigma * body_temp^4` term computed in one wide accumulator and rounded once. `t_cold` is zero (the
    // absolute radiance), so this signal is non-zero for any warm body and IS surfaced on the pinned paths.
    laws::radiant_emission_tier2(
        coefficient,
        Fixed::ONE,
        body_temp,
        Fixed::ZERO,
        sigma_bits,
        sigma_scale,
        FLUX_MAX,
    )
}

/// The derived resting drain FRACTION of the energy reserve per tick, composing the physics laws: the
/// Kleiber basal rate over the body mass plus the thermoregulatory heat loss over the whole-body surface
/// (the body held at its resting set point against the ambient medium), bridged to a fraction of the
/// reserve's stored energy. This replaces the authored `base_metabolic_drain`: two bodies diverge from
/// mass, tissue, medium, and temperature alone. `energy_capacity` is the being's energy-reserve capacity
/// (the caller passes `homeostasis.capacity(ENERGY)`); `tick` is the tick length in seconds. The radiant
/// term's emissivity is read from the being's covering ([`covering_emissivity`]).
#[allow(clippy::too_many_arguments)]
pub fn derive_base_drain(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    energy_capacity: Fixed,
    energy_density: Fixed,
    ambient_temp: Fixed,
    setpoint: Fixed,
    medium_h: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    base_drain_from(
        plan,
        energy_capacity,
        energy_density,
        whole_body_surface(plan, organs),
        covering_emissivity(plan, organs),
        ambient_temp,
        setpoint,
        medium_h,
        tick,
        anchors,
    )
}

/// The base drain over EXPLICIT composition scalars (the exposed surface, the per-mass energy density, and
/// the radiating-surface emissivity), supplied by the caller from either a catalog organ set
/// ([`derive_base_drain`]) or a GROWN body's grown tissue ([`crate::morphogen::Structure::composition_sum`] /
/// `whole_body_energy_density`), so a fully grown body pays its thermoregulatory and basal drain off its own
/// tissue rather than the empty digest's zeros (emergent-anatomy Step 3, the derived-physiology grow). The
/// `emissivity` is the being's covering datum ([`covering_emissivity`]), passed explicitly like `surface`.
/// The math is identical; only the source of the surface, energy density, and emissivity differs.
#[allow(clippy::too_many_arguments)]
pub fn base_drain_from(
    plan: &BodyPlan,
    energy_capacity: Fixed,
    energy_density: Fixed,
    surface: Fixed,
    emissivity: Fixed,
    ambient_temp: Fixed,
    setpoint: Fixed,
    medium_h: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let mass_kg = body_mass_kg(plan, anchors);
    let basal = laws::basal_metabolic_rate(mass_kg, anchors.kleiber_a, RATE_MAX);
    // At rest the body holds its set point; the thermoregulatory demand is the heat shed to the medium
    // at that core temperature over the body's exposed surface.
    let heat_loss = laws::resting_heat_loss(
        medium_h,
        surface,
        setpoint,
        ambient_temp,
        emissivity,
        anchors.sigma_fine_bits,
        anchors.sigma_fine_scale,
        FLUX_MAX,
    );
    // The reserve's energy-storing mass: the anatomy-derived reserve capacity scaled to the body's
    // physical mass, so the bridge to stored joules scales with size (a larger body stores proportionally
    // more absolute energy). The exact kJ/g-to-joule reconciliation of the reserve units is the
    // R-UNITS-PIN owner calibration (the honest units limit); the mechanism derives, the absolute scale
    // is the owner's anchors and the floor's energy-density units.
    let reserve_mass = energy_capacity.checked_mul(mass_kg).unwrap_or(Fixed::ZERO);
    laws::metabolic_drain_fraction(
        basal,
        heat_loss,
        reserve_mass,
        energy_density,
        tick,
        FRAC_MAX,
    )
}

/// The ALIEN-CLEAN physical intake (the R-PHYS-BIO edibility measure): the reserve-amount a being gains by
/// eating a food's content on a reserve's OWN backing class, and the content it eats to do so. This is the
/// intake counterpart to [`derive_base_drain`] and uses the SAME size-scaled reserve bridge: one unit of
/// reserve amount is worth `body_mass * body_storage_density` of physical content (for the energy reserve,
/// `mass_kg * bio.energy_density` joules), so eating `content` of the class, assimilated (`assim`) and passed
/// at the trophic efficiency (`eta`), raises the reserve by `content * assim * eta / (body_mass *
/// body_storage_density)`. The being eats only enough to fill its `room` (bounded by what is `available`), so
/// no bite overflows the reserve, and the reserve fills by the food's PHYSICAL content, never a made-up
/// biomass number. Keyed on NO axis identity: the same mechanism fills a chemical-energy reserve from an
/// energy-dense seed and a thaumic reserve from a mana-bearing plant (Principle 9), the class and the storage
/// density being the being's own data. A being whose body stores none of the class (`body_storage_density <=
/// 0`), a zero body mass, a non-digester (`assim <= 0`), or a full reserve (`room <= 0`) eats nothing.
/// Returns `(content_to_eat, reserve_gain)`, the gain bounded by `room`.
pub fn physical_intake(
    available: Fixed,
    assim: Fixed,
    eta: Fixed,
    body_mass: Fixed,
    body_storage_density: Fixed,
    room: Fixed,
) -> (Fixed, Fixed) {
    let num = assim.checked_mul(eta).unwrap_or(Fixed::ZERO); // assimilated, trophic-passed content -> reserve
    let denom = body_mass
        .checked_mul(body_storage_density)
        .unwrap_or(Fixed::ZERO); // reserve-unit content worth
    if available <= Fixed::ZERO || room <= Fixed::ZERO || num <= Fixed::ZERO || denom <= Fixed::ZERO
    {
        return (Fixed::ZERO, Fixed::ZERO);
    }
    // The content that would exactly fill the room: room * denom / num. An overflow means the room is
    // effectively unbounded relative to the content, so eat everything available.
    let content_to_fill = room
        .checked_mul(denom)
        .and_then(|x| x.checked_div(num))
        .unwrap_or(Fixed::MAX);
    let eaten = available.min(content_to_fill);
    // The reserve gain from the eaten content, capped at the room (the division can round up by a fixed-point
    // ulp, so the min keeps the reserve from a one-tick overfill).
    let gain = eaten
        .checked_mul(num)
        .and_then(|x| x.checked_div(denom))
        .unwrap_or(room)
        .min(room);
    (eaten, gain)
}

/// A being's summarized FIRST-HAND FELT EXPERIENCE over a window: its own reserves' movement folded into an
/// intensity (how much its total reserve health changed) and a signed valence (whether it improved or
/// worsened). See [`felt_salience`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FeltExperience {
    /// How intense the felt change was: the absolute net movement of the being's total reserve health over
    /// the window, in reserve-fraction units. A calm window (reserves roughly held) reads near zero.
    pub intensity: Fixed,
    /// The signed valence of the change: `+1` if the being's total reserve health ROSE over the window, `-1`
    /// if it FELL (a falling reserve is negative, the floor sign of hardship), `0` if unchanged. This is a
    /// floor fact about the being's own body, carrying NO axis and NO pole: which conviction the experience
    /// bears on, and in which direction, is a per-being LEARNED coupling ABOVE this primitive, never authored
    /// here (the blind framing panel's ruling, `docs/working/OWNER_DECISIONS_LOG.md` R2).
    pub valence: Fixed,
}

/// The ALIEN-CLEAN felt-experience measure: fold a being's own signed reserve level-changes (the floor's
/// interoceptive deltas, [`crate::homeostasis::ReserveMemory::delta`]) into one [`FeltExperience`]. This is
/// the floor input the learned experience-to-conviction coupling reads: the intensity is the absolute net
/// change in the being's total reserve health, the valence its sign. Keyed on NO axis identity: it sums the
/// signed level-changes over WHATEVER reserves the being's physiology declares, so a photosynthetic being's
/// light-charge, a silicon body's heat store, and a grazer's energy and water fold through the same call and
/// the alien is a data row (Principle 9). It authors nothing above the floor: it emits only how much the
/// being's reserves moved and in which net direction, never which belief that bears on, so the coupling that
/// routes this felt signal to a conviction (and gives it a direction on that conviction) stays a learned,
/// per-being fact the layer above discovers rather than a mapping this primitive stamps (OWNER_DECISIONS_LOG
/// R2, the corrected framing the blind framing panel converged on). A pure function; the net saturates rather
/// than wrapping, so an extreme swing cannot panic under the release overflow checks.
pub fn felt_salience(reserve_deltas: impl IntoIterator<Item = Fixed>) -> FeltExperience {
    let mut net = Fixed::ZERO;
    for d in reserve_deltas {
        net = net.saturating_add(d);
    }
    let valence = match net.cmp(&Fixed::ZERO) {
        std::cmp::Ordering::Greater => Fixed::ONE,
        std::cmp::Ordering::Less => Fixed::ZERO - Fixed::ONE,
        std::cmp::Ordering::Equal => Fixed::ZERO,
    };
    FeltExperience {
        intensity: net.abs(),
        valence,
    }
}

/// The derived exertion drain coupling: the added fraction of the energy reserve drained per tick per
/// unit of exertion, from the mechanical work power a full-exertion body sustains (`force * velocity`),
/// bridged to a reserve fraction. This replaces the authored `exertion_drain_coupling`;
/// [`crate::homeostasis::Homeostasis::metabolize_derived`] scales it by the being's exertion signal
/// and ADDS it to the base drain, so the two must share one power scale. It reads the work power on
/// the WATT scale ([`civsim_physics::laws::power_watts`]), the same scale the basal rate and the
/// `metabolic_drain_fraction` bridge use, so the base and exertion fractions are commensurate rather
/// than off by the kilowatt factor (the earlier `laws::power` returned kilowatts, making the summed
/// exertion term a thousand times too small).
pub fn derive_exertion_coupling(
    plan: &BodyPlan,
    energy_capacity: Fixed,
    energy_density: Fixed,
    force: Fixed,
    velocity: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let work_power = laws::power_watts(force, velocity, POWER_MAX);
    // The same size-scaled reserve-energy bridge as the base drain (see derive_base_drain). The per-mass
    // energy density is supplied by the caller, so a grown body reads its own grown tissue.
    let reserve_mass = energy_capacity
        .checked_mul(body_mass_kg(plan, anchors))
        .unwrap_or(Fixed::ZERO);
    laws::metabolic_drain_fraction(
        work_power,
        Fixed::ZERO,
        reserve_mass,
        energy_density,
        tick,
        FRAC_MAX,
    )
}

/// The DERIVED physiological WATER-loss drain, retiring the authored flat `base_drain` on the water axis
/// (the R-METABOLIZE water sibling of [`base_drain_from`] / [`derive_exertion_coupling`]). Real endotherm
/// water loss is dominated by RESPIRATORY loss (water vapour in exhaled breath, whose rate is set by
/// ventilation, itself proportional to metabolic rate) and by EVAPORATIVE thermoregulatory loss (sweating
/// or panting to shed the thermoregulatory heat load, evaporating water to carry off that heat). Both scale
/// with metabolic POWER: the respiratory term with the whole resting power (ventilation tracks metabolism),
/// the evaporative term with the heat that must be shed. So the water loss DERIVES as `water_flux =
/// water_per_power * metabolic_power`, the resting power `basal + thermoregulatory_heat_loss` for the base
/// term and the work power for the exertion term, each bridged to a fraction of the hydration reserve by the
/// SAME size-scaled reserve bridge the energy drain uses (the reserve's stored water is `water_capacity *
/// body_mass * water_density`). Keyed on the body's own mass, tissue, surface, and medium, never a race id
/// or a flat rate: a larger or hotter-working body in a drier, hotter medium loses proportionally more
/// water, from the physics alone (Principle 9). `water_per_power` is the reserved owner anchor (grams of
/// water lost per joule of metabolism, the respiratory-plus-evaporative water cost of energy), surfaced with
/// its basis, never fabricated. Returns `(base_fraction, exertion_fraction)` per tick, the derived siblings
/// of the authored [`crate::homeostasis::HomeostaticAxisDef::base_drain`] / `exertion_drain` on the water
/// axis. Pure and RNG-free.
#[allow(clippy::too_many_arguments)]
pub fn derive_water_loss(
    plan: &BodyPlan,
    water_capacity: Fixed,
    water_density: Fixed,
    surface: Fixed,
    emissivity: Fixed,
    ambient_temp: Fixed,
    setpoint: Fixed,
    medium_h: Fixed,
    force: Fixed,
    velocity: Fixed,
    water_per_power: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> (Fixed, Fixed) {
    let mass_kg = body_mass_kg(plan, anchors);
    let basal = laws::basal_metabolic_rate(mass_kg, anchors.kleiber_a, RATE_MAX);
    // At rest the body holds its set point; the thermoregulatory heat it sheds is the evaporative water
    // driver (sweating/panting), so the resting water power is the whole resting metabolic power.
    let heat_loss = laws::resting_heat_loss(
        medium_h,
        surface,
        setpoint,
        ambient_temp,
        emissivity,
        anchors.sigma_fine_bits,
        anchors.sigma_fine_scale,
        FLUX_MAX,
    );
    let resting_power = basal.saturating_add(heat_loss);
    let work_power = laws::power_watts(force, velocity, POWER_MAX);
    // The hydration reserve's water-storing mass, the same size-scaled bridge the energy drain uses: the
    // anatomy-derived water capacity scaled to the body's physical mass, so the bridge to stored water mass
    // scales with size. The water flux (per_power * metabolic power) enters as the `basal` slot of the
    // shared fraction bridge with a zero second term, so `fraction = water_flux * tick / stored_water`.
    let reserve_mass = water_capacity.checked_mul(mass_kg).unwrap_or(Fixed::ZERO);
    let base_flux = resting_power
        .checked_mul(water_per_power)
        .unwrap_or(FLUX_MAX);
    let work_flux = work_power.checked_mul(water_per_power).unwrap_or(FLUX_MAX);
    let base = laws::metabolic_drain_fraction(
        base_flux,
        Fixed::ZERO,
        reserve_mass,
        water_density,
        tick,
        FRAC_MAX,
    );
    let exertion = laws::metabolic_drain_fraction(
        work_flux,
        Fixed::ZERO,
        reserve_mass,
        water_density,
        tick,
        FRAC_MAX,
    );
    (base, exertion)
}

/// The derived body-to-medium thermal coupling rate per tick: `h * A / (m * c)`, the discrete
/// Newton-cooling rate that governs `new_temp = temp + rate * (medium_temp - temp)`. `h` is the medium
/// convective coefficient (a fluids-floor datum), `A` the whole-body convective surface, and `m * c` the
/// body's thermal mass (its mass in kilograms times its whole-body specific heat). This replaces the
/// authored `field.body_exchange`: a high-surface, low-thermal-mass body couples fast; a compact, dense
/// one couples slowly, from the physics alone. Clamped to `[0, 1]` for the explicit scheme's stability
/// (rate 1 is instant equilibration; a rate above 1 would overshoot). A body with no exchange surface
/// (or in a medium with no coupling) reads zero: no surface, no convective exchange.
pub fn derive_body_exchange_rate(
    plan: &BodyPlan,
    organs: &BodyPlanRegistry,
    medium_h: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    body_exchange_rate_from(
        plan,
        whole_body_surface(plan, organs),
        whole_body_specific_heat(plan, organs),
        medium_h,
        tick,
        anchors,
    )
}

/// The body-to-medium thermal exchange rate over EXPLICIT composition scalars (the exposed surface and the
/// specific heat), supplied by the caller from either a catalog organ set ([`derive_body_exchange_rate`]) or
/// a GROWN body's grown tissue ([`crate::morphogen::Structure::composition_sum`] / `composition_mean`), so a
/// fully grown body couples to the medium off its own tissue rather than the empty digest's zeros
/// (emergent-anatomy Step 3, the derived-physiology grow). The math is identical.
pub fn body_exchange_rate_from(
    plan: &BodyPlan,
    surface: Fixed,
    specific_heat: Fixed,
    medium_h: Fixed,
    tick: Fixed,
    anchors: &MetabolicAnchors,
) -> Fixed {
    let ha = match medium_h.checked_mul(surface) {
        Some(x) => x,
        None => return Fixed::ONE,
    };
    if ha <= Fixed::ZERO {
        // No exchange surface (or no medium coupling): no convective exchange, the body holds its heat.
        return Fixed::ZERO;
    }
    let mass_kg = body_mass_kg(plan, anchors);
    let mc = match mass_kg.checked_mul(specific_heat) {
        Some(x) => x,
        // An enormous thermal mass barely responds over one tick.
        None => return Fixed::ZERO,
    };
    if mc <= Fixed::ZERO {
        // A massless (heat-capacity-less) body equilibrates instantly.
        return Fixed::ONE;
    }
    let per_second = match ha.checked_div(mc) {
        Some(x) => x,
        None => return Fixed::ONE,
    };
    per_second
        .checked_mul(tick)
        .unwrap_or(Fixed::ONE)
        .clamp(Fixed::ZERO, Fixed::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{OrganKindDef, Part, Temperament, TissueComposition};
    use crate::homeostasis::{Homeostasis, HomeostaticRegistry, ENERGY};

    #[test]
    fn being_signal_emission_derives_from_body_temperature_not_a_species_label() {
        // The emitter side of the being-percept keystone: a being's perceptible signal is its own thermal
        // self-emission (Stefan-Boltzmann off its body temperature) times a reserved coupling coefficient,
        // so it keys on the being's OWN temperature, never a per-species signature.
        let (sigma_bits, sigma_scale) = derived_stefan_boltzmann_fine();
        let coeff = Fixed::from_ratio(1, 2);

        // A body at absolute zero emits nothing: no thermal signal to perceive (a cold ambusher, a corpse
        // that has cooled to ambient-zero). The signal is the being's own radiance, so zero temperature is
        // zero emission.
        assert_eq!(
            being_signal_emission(Fixed::ZERO, coeff, sigma_bits, sigma_scale),
            Fixed::ZERO,
            "a body at absolute zero emits no thermal signal"
        );

        // A warmer body emits a stronger signal than a cooler one (the T^4 dependence): a warm predator is
        // more perceptible than a cool one, and this divergence is temperature alone, no label.
        let cool = being_signal_emission(Fixed::from_int(280), coeff, sigma_bits, sigma_scale);
        let warm = being_signal_emission(Fixed::from_int(310), coeff, sigma_bits, sigma_scale);
        assert!(
            warm > cool && cool > Fixed::ZERO,
            "a warmer body emits a stronger thermal signal (T^4 dependence)"
        );

        // The reserved coefficient is the emission lever: a larger coupling yields a stronger signal at the
        // same temperature (monotone in the coefficient), so the reserved value means what its basis says.
        // Monotone rather than exact-double because a half-scale fixed-point multiply truncates by up to one
        // ULP; the lever's DIRECTION is the load-bearing property, not bit-exact linearity.
        let quarter = being_signal_emission(
            Fixed::from_int(300),
            Fixed::from_ratio(1, 4),
            sigma_bits,
            sigma_scale,
        );
        let half = being_signal_emission(
            Fixed::from_int(300),
            Fixed::from_ratio(1, 2),
            sigma_bits,
            sigma_scale,
        );
        assert!(
            half > quarter && quarter > Fixed::ZERO,
            "a larger reserved coefficient yields a stronger emission (the coupling lever)"
        );

        // Deterministic: identical inputs give the identical bit-exact emission (Principle 3), the run path's
        // requirement for a reproducible perceive phase.
        assert_eq!(
            being_signal_emission(Fixed::from_int(305), coeff, sigma_bits, sigma_scale),
            being_signal_emission(Fixed::from_int(305), coeff, sigma_bits, sigma_scale),
            "the emission is a pure deterministic read of temperature and the coefficient"
        );
    }

    #[test]
    fn felt_salience_folds_reserve_deltas_alien_clean_with_no_axis_or_pole() {
        // The felt-experience primitive keys on NO axis identity: it folds an iterator of signed reserve
        // level-changes into (intensity, valence), so the SAME call summarizes a grazer's energy+water and a
        // thaumic being's mana+heat identically when their deltas match. A falling total is negative (the
        // floor sign of hardship); a rising total positive; a net-calm window near zero.
        let quarter = Fixed::from_ratio(1, 4);
        let eighth = Fixed::from_ratio(1, 8);

        // A grazer whose energy fell 1/4 and water fell 1/8: total reserve health fell 3/8, valence negative.
        let hardship = felt_salience([Fixed::ZERO - quarter, Fixed::ZERO - eighth]);
        assert_eq!(
            hardship.valence,
            Fixed::ZERO - Fixed::ONE,
            "falling reserves feel negative"
        );
        assert_eq!(
            hardship.intensity,
            Fixed::from_ratio(3, 8),
            "intensity is the absolute net fall"
        );

        // An ALIEN with the IDENTICAL delta magnitudes on entirely different reserves (a mana pool and a heat
        // store), supplied through an owned Vec rather than an array, folds to the identical felt experience: no
        // axis identity and no container shape is read (Principle 9).
        let alien_deltas: Vec<Fixed> = vec![Fixed::ZERO - quarter, Fixed::ZERO - eighth];
        let alien = felt_salience(alien_deltas);
        assert_eq!(
            alien, hardship,
            "the same deltas give the same felt experience whatever the reserves mean"
        );

        // A rising total feels positive; a net-calm window (equal-and-opposite swings) feels nothing at all,
        // carrying no valence to hand any conviction, so the layer above has nothing to route until real net
        // change is felt.
        assert_eq!(
            felt_salience([quarter, eighth]).valence,
            Fixed::ONE,
            "rising reserves feel positive"
        );
        let calm = felt_salience([quarter, Fixed::ZERO - quarter]);
        assert_eq!(
            calm.valence,
            Fixed::ZERO,
            "an equal-and-opposite window has no net valence"
        );
        assert_eq!(calm.intensity, Fixed::ZERO, "and no net intensity");

        // An extreme swing saturates rather than panicking under the release overflow checks.
        let _ = felt_salience([Fixed::MAX, Fixed::MAX]);
    }

    #[test]
    fn physical_intake_is_alien_clean_and_fills_by_physical_content_not_a_biomass_number() {
        // The R-PHYS-BIO edibility measure keys on NO axis identity: the reserve fills by the food's physical
        // content through the being's own storage bridge, so the SAME call fills a chemical-energy reserve
        // from an energy-dense food and a thaumic reserve from a mana-bearing food, given the same physical
        // quantities. Proof: two callers with identical numbers (one thinking "joules", one "mana") get the
        // identical (eaten, gain); the mechanism never reads bio.energy_density or any Earth axis.
        let assim = Fixed::from_ratio(8, 10);
        let eta = Fixed::from_ratio(1, 2);
        let body_mass = Fixed::from_int(60);
        let storage_density = Fixed::from_int(5); // reserve-unit content worth = 60 * 5 = 300
                                                  // A large room and abundant food: the being eats only what fills the room, and the gain equals room.
        let room = Fixed::from_int(30);
        let plenty = Fixed::from_int(100000);
        let energy_reserve = physical_intake(plenty, assim, eta, body_mass, storage_density, room);
        let mana_reserve = physical_intake(plenty, assim, eta, body_mass, storage_density, room);
        assert_eq!(
            energy_reserve, mana_reserve,
            "the intake is alien-clean: identical physical quantities give an identical result whatever the \
             backing class means (energy or mana)"
        );
        let (eaten, gain) = energy_reserve;
        // Abundant food fills the reserve to its room, never past it (the round-trip through two divisions can
        // lose a fixed-point ulp, so gain is at or just under room, never above).
        let ulp = Fixed::from_ratio(1, 1000);
        assert!(
            gain <= room && gain >= room - ulp,
            "abundant food fills the reserve to its room ({gain:?} ~ {room:?}), never past it"
        );
        // The eaten content is room * (body_mass * storage_density) / (assim * eta) = 30*300/0.4 = 22500.
        assert_eq!(
            eaten,
            room.checked_mul(body_mass.checked_mul(storage_density).unwrap())
                .unwrap()
                .checked_div(assim.checked_mul(eta).unwrap())
                .unwrap(),
            "the content eaten is the physical amount whose assimilated value fills the room"
        );
        // Round-trip: the gain from the eaten content is the physical conversion, not a fabricated number.
        assert_eq!(
            gain,
            eaten
                .checked_mul(assim.checked_mul(eta).unwrap())
                .unwrap()
                .checked_div(body_mass.checked_mul(storage_density).unwrap())
                .unwrap(),
            "gain = eaten * assim * eta / (body_mass * storage_density): the drain's own reserve bridge"
        );

        // Scarce food: the being eats all that is available and gains proportionally less than a full room.
        let scarce = Fixed::from_int(300); // worth 300 * 0.4 / 300 = 0.4 reserve amount
        let (eaten_s, gain_s) =
            physical_intake(scarce, assim, eta, body_mass, storage_density, room);
        assert_eq!(
            eaten_s, scarce,
            "when food is scarce the being eats all of it"
        );
        assert!(
            gain_s < room && gain_s > Fixed::ZERO,
            "a scarce bite fills the reserve partway"
        );

        // A being that stores none of the class (no reserve of that kind) or cannot digest it eats nothing.
        assert_eq!(
            physical_intake(plenty, assim, eta, body_mass, Fixed::ZERO, room),
            (Fixed::ZERO, Fixed::ZERO),
            "a body that stores none of the class has no reserve of that kind and eats none"
        );
        assert_eq!(
            physical_intake(plenty, Fixed::ZERO, eta, body_mass, storage_density, room),
            (Fixed::ZERO, Fixed::ZERO),
            "a non-digester of the class gains nothing"
        );
        assert_eq!(
            physical_intake(plenty, assim, eta, body_mass, storage_density, Fixed::ZERO),
            (Fixed::ZERO, Fixed::ZERO),
            "a full reserve draws nothing"
        );
    }

    fn temperament() -> Temperament {
        Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(1, 2),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        }
    }

    fn organ(kind: u16, dev: (i64, i64)) -> Part {
        Part {
            kind,
            development: Fixed::from_ratio(dev.0, dev.1),
        }
    }

    /// A body of a given mass bearing the given organs (locomotion irrelevant here).
    fn body(mass: (i64, i64), organs: Vec<Part>) -> BodyPlan {
        BodyPlan {
            body_mass: Fixed::from_ratio(mass.0, mass.1),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![],
            locomotion: vec![1],
            organs,
            temperament: temperament(),
        }
    }

    /// A registry whose organ set adds a skin/covering tissue bearing convective surface, a
    /// dense-and-heat-capacious tissue (density and specific heat), and an energy tissue, at known ids,
    /// alongside the default organs. Labelled fixtures.
    fn registry() -> (BodyPlanRegistry, u16, u16, u16) {
        let mut reg = BodyPlanRegistry::dev_default();
        let skin = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: skin,
            name: "skin".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(CONVECTIVE_SURFACE, Fixed::from_int(2))]),
        });
        let flesh = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: flesh,
            name: "flesh".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[
                (TISSUE_DENSITY, Fixed::from_int(1000)),
                (TISSUE_SPECIFIC_HEAT, Fixed::from_int(3500)),
            ]),
        });
        let fat = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: fat,
            name: "energy-store".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(ENERGY_DENSITY, Fixed::ONE)]),
        });
        (reg, skin, flesh, fat)
    }

    #[test]
    fn surface_optical_axis_reads_one_axis_and_is_zero_for_absent() {
        use crate::anatomy::KindDef;
        let (mut reg, _skin, _flesh, _fat) = registry();
        // A covering carrying two optical axes with distinct values (a labelled fixture, not owner canon).
        let cov = reg.coverings.len() as u16;
        let mut material = std::collections::BTreeMap::new();
        material.insert(OPT_EMISSIVITY.to_string(), Fixed::from_ratio(9, 10));
        material.insert("opt.refractive_index".to_string(), Fixed::from_ratio(3, 2));
        reg.coverings.push(KindDef {
            id: cov,
            name: "test-hide".to_string(),
            fantasy: false,
            geometry: std::collections::BTreeMap::new(),
            material,
        });
        let mut plan = body((1, 1), vec![]);
        plan.covering = Part {
            kind: cov,
            development: Fixed::ONE,
        };
        // It reads exactly ONE axis, not a composite: each declared axis returns its own value.
        assert_eq!(
            surface_optical_axis(&plan, &reg, OPT_EMISSIVITY),
            Fixed::from_ratio(9, 10)
        );
        assert_eq!(
            surface_optical_axis(&plan, &reg, "opt.refractive_index"),
            Fixed::from_ratio(3, 2)
        );
        // An axis the covering does not declare reads ZERO (graceful absence, the feature is simply absent).
        assert_eq!(surface_optical_axis(&plan, &reg, "opt.albedo"), Fixed::ZERO);
        // `covering_emissivity` is the same read on the emissivity axis (the delegation is byte-identical).
        assert_eq!(
            covering_emissivity(&plan, &reg),
            surface_optical_axis(&plan, &reg, OPT_EMISSIVITY)
        );
        // A being whose covering kind is not in the registry (an alien with no covering-row) reads ZERO on
        // every axis: it carries no feature rather than a synthesized default, so the alien stays a data row.
        plan.covering = Part {
            kind: 60000,
            development: Fixed::ONE,
        };
        assert_eq!(
            surface_optical_axis(&plan, &reg, OPT_EMISSIVITY),
            Fixed::ZERO
        );
    }

    #[test]
    fn the_food_energy_density_reconciliation_keeps_forage_intake_in_the_survivable_regime() {
        // Regression guard for R-UNITS-PIN (the end-of-arc audit flagged the "world thrives" proof as a manual
        // eyeball of a non-canonical example, with NO test protecting the calibration): the forage
        // reconciliation `food_energy_density` must keep a foraging being's per-tick intake gain a MEANINGFUL
        // fraction of its reserve room, not the near-zero gain the un-reconciled physical bridge gives (the
        // body_mass * storage_density denominator is ~1500x the raw standing-food supply, the mismatch that
        // starved the world before the reconciliation). If the scale regresses out of the survivable regime,
        // this fails, so a silent change to the dev value can no longer pass CI unnoticed. This is a unit-level
        // guard; a scenario-level cohort-survival test is the follow-on (OWNER_DECISIONS_LOG item 6).
        let food_ed = crate::locomotion::LocomotionParams::dev_default().food_energy_density;
        let assim = Fixed::ONE;
        let eta = Fixed::from_ratio(1, 2);
        let body_mass = Fixed::from_int(60); // a representative body mass (kg)
        let storage_density = Fixed::from_int(25); // a representative tissue energy density
        let supply = Fixed::from_ratio(1, 2); // a plausible standing-food supply the forager reads off the field
        let room = Fixed::ONE; // a drained unit reserve
        let content = supply.checked_mul(food_ed).unwrap();
        let (_eaten, gain) = physical_intake(content, assim, eta, body_mass, storage_density, room);
        // The reconciled gain fills at least a quarter of the drained reserve in one forage tick (survivable);
        // at food_ed = 1 (un-reconciled) the gain is ~1e-4 * room and this guard trips.
        assert!(
            gain >= room.checked_div(Fixed::from_int(4)).unwrap(),
            "the reconciled forage intake gain {gain:?} must fill a survivable fraction of the reserve room \
             {room:?}; a regression of food_energy_density out of the survivable regime trips this guard"
        );
    }

    #[test]
    fn whole_body_composition_vector_generalizes_body_density_and_unions_axes() {
        use crate::medium::body_density;
        let (organs, _skin, flesh, fat) = registry();
        // A body of flesh (density 1000, specific heat 3500) and an energy store (energy density 1), each at
        // full development. The composition vector carries the UNION of every axis the parts declare.
        let plan = body((1, 2), vec![organ(flesh, (1, 1)), organ(fat, (1, 1))]);
        let vector = whole_body_composition_vector(&plan, &organs);
        assert_eq!(
            vector.get(TISSUE_DENSITY).copied(),
            Some(Fixed::from_int(1000)),
            "the flesh density enters the vector"
        );
        assert_eq!(
            vector.get(ENERGY_DENSITY).copied(),
            Some(Fixed::ONE),
            "the energy store's energy density enters the vector (axis union, not one axis)"
        );
        assert_eq!(
            vector.get(TISSUE_SPECIFIC_HEAT).copied(),
            Some(Fixed::from_int(3500)),
            "the flesh specific heat enters the vector too"
        );
        // The generalization subsumes the special case: the vector's mat.density EQUALS body_density on the
        // same organs-only body.
        assert_eq!(
            vector.get(TISSUE_DENSITY).copied().unwrap(),
            body_density(&plan, &organs),
            "the vector's mat.density is exactly the special-case body_density"
        );

        // A hand-computed development-weighted mean over TWO density-bearing organs: flesh (density 1000, dev
        // 1) and a bone organ (density 1900, dev 3) give (1*1000 + 3*1900) / (1+3) = 6700/4 = 1675.
        let mut reg = organs;
        let bone = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: bone,
            name: "bone".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(TISSUE_DENSITY, Fixed::from_int(1900))]),
        });
        let mixed = body((1, 2), vec![organ(flesh, (1, 1)), organ(bone, (3, 1))]);
        let mixed_vec = whole_body_composition_vector(&mixed, &reg);
        assert_eq!(
            mixed_vec.get(TISSUE_DENSITY).copied(),
            Some(Fixed::from_int(1675)),
            "the density is the development-weighted mean over both organs"
        );
        assert_eq!(
            mixed_vec.get(TISSUE_DENSITY).copied().unwrap(),
            body_density(&mixed, &reg),
            "and it still equals body_density on the two-organ body"
        );
    }

    #[test]
    fn whole_body_surface_follows_the_convective_tissue_and_is_zero_without_it() {
        let (organs, skin, _flesh, fat) = registry();
        // A body with a full skin presents more surface than one with a quarter skin.
        let big = whole_body_surface(&body((1, 2), vec![organ(skin, (1, 1))]), &organs);
        let small = whole_body_surface(&body((1, 2), vec![organ(skin, (1, 4))]), &organs);
        assert!(big > small, "more convective tissue, more exchange area");
        assert_eq!(
            big,
            Fixed::from_int(2),
            "full skin (dev 1 * surface 2) = 2 m^2"
        );
        // A body with only an energy organ (no convective surface) presents zero area.
        assert_eq!(
            whole_body_surface(&body((1, 2), vec![organ(fat, (1, 1))]), &organs),
            Fixed::ZERO,
            "no convective tissue, no exchange area"
        );
    }

    #[test]
    fn whole_body_muscle_force_follows_the_strength_tissue_and_mass_and_is_zero_without_it() {
        // Real-world unification step 5: the work force a body exerts is its development-weighted muscle
        // strength times its mass, mirroring the individual-tier Body::strength (muscle mass times
        // material strength). A muscle-bearing tissue provides force to the extent of its development; a
        // body with none exerts zero (the absence convention); two equal-mass bodies with different
        // muscle endowment exert different force; and two bodies of equal muscle but different mass exert
        // different force (the mass scaling the earlier raw proxy carried, kept by the ruling).
        let anchors = MetabolicAnchors::dev_fixture(); // body_mass_kg_scale = 100
        let mut reg = BodyPlanRegistry::dev_default();
        let muscle = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: muscle,
            name: "muscle".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(MUSCLE_STRENGTH, Fixed::from_int(4))]),
        });
        let energy = reg.organs.len() as u16;
        reg.organs.push(OrganKindDef {
            id: energy,
            name: "energy".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[(ENERGY_DENSITY, Fixed::ONE)]),
        });
        let big =
            whole_body_muscle_force(&body((1, 2), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        let small =
            whole_body_muscle_force(&body((1, 2), vec![organ(muscle, (1, 4))]), &reg, &anchors);
        assert!(big > small, "more muscle development, more work force");
        assert_eq!(
            big,
            Fixed::from_int(200),
            "full muscle (dev 1 * strength 4) times mass (0.5 * 100 kg) = 200"
        );
        assert_eq!(
            whole_body_muscle_force(&body((1, 2), vec![organ(energy, (1, 1))]), &reg, &anchors),
            Fixed::ZERO,
            "no strength tissue, no work force (not a mass-sized default)"
        );
        // Two bodies of equal normalized mass but different muscle endowment exert different force,
        // which the earlier body-mass proxy could not distinguish.
        let strong =
            whole_body_muscle_force(&body((3, 4), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        let weak =
            whole_body_muscle_force(&body((3, 4), vec![organ(muscle, (1, 8))]), &reg, &anchors);
        assert!(
            strong > weak,
            "equal mass, different muscle, different force"
        );
        // Two bodies of equal muscle endowment but different mass exert different force: the mass scaling
        // the ruling keeps, that the earlier composition-only sum had dropped.
        let heavy =
            whole_body_muscle_force(&body((1, 1), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        let light =
            whole_body_muscle_force(&body((1, 4), vec![organ(muscle, (1, 1))]), &reg, &anchors);
        assert!(
            heavy > light,
            "equal muscle, more mass, more force (the mass factor is present)"
        );
    }

    #[test]
    fn whole_body_specific_heat_averages_the_tissue_and_is_zero_without_it() {
        let (organs, skin, flesh, _fat) = registry();
        assert_eq!(
            whole_body_specific_heat(&body((1, 2), vec![organ(flesh, (1, 1))]), &organs),
            Fixed::from_int(3500),
            "one flesh organ carries its specific heat"
        );
        // No tissue declares specific heat (skin carries only surface): the absence convention reads
        // ZERO, not a hidden terran-water default (audit defect 2, Principle 9).
        assert_eq!(
            whole_body_specific_heat(&body((1, 2), vec![organ(skin, (1, 1))]), &organs),
            Fixed::ZERO,
            "no specific-heat tissue reads zero (the absence convention), never the water constant"
        );
    }

    #[test]
    fn two_specific_heat_free_bodies_do_not_converge_on_the_earth_water_value() {
        // Regression (audit defect 2): two distinct bodies that both declare no specific-heat tissue
        // must not both read the same hidden 4186 water value. Under the absence convention both read
        // ZERO thermal mass, so the body-to-medium coupling takes its own no-thermal-mass branch
        // (rate one, instant equilibration) rather than converging on the terran-water constant.
        let (organs, skin, _flesh, _fat) = registry();
        let anchors = MetabolicAnchors::dev_fixture();
        let a = body((1, 2), vec![organ(skin, (1, 1))]);
        let b = body((1, 4), vec![organ(skin, (1, 2))]);
        assert_eq!(whole_body_specific_heat(&a, &organs), Fixed::ZERO);
        assert_eq!(whole_body_specific_heat(&b, &organs), Fixed::ZERO);
        // The coupling is not authored from a hidden water thermal mass; the no-thermal-mass branch
        // reads rate one for both.
        assert_eq!(
            derive_body_exchange_rate(&a, &organs, Fixed::from_int(10), Fixed::ONE, &anchors),
            Fixed::ONE
        );
        assert_eq!(
            derive_body_exchange_rate(&b, &organs, Fixed::from_int(10), Fixed::ONE, &anchors),
            Fixed::ONE
        );
    }

    #[test]
    fn a_larger_denser_body_drains_a_smaller_fraction_of_its_reserve() {
        // The Kleiber signature: basal power grows as mass^(3/4) while the energy reserve grows linearly
        // with the body's energy tissue, so a larger, denser body spends a SMALLER fraction of its
        // reserve per tick. Thermoneutral (ambient == set point) to isolate the basal term.
        let (organs, _skin, _flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let setpoint = Fixed::from_int(310);
        let tick = Fixed::ONE;
        // Small body: quarter mass, a quarter energy store.
        let small = body((1, 4), vec![organ(fat, (1, 4))]);
        // Large, denser body: full mass, a full energy store (more energy-dense tissue).
        let large = body((1, 1), vec![organ(fat, (1, 1))]);
        let cap_small = Homeostasis::new(&reg, &small, &organs).capacity(ENERGY);
        let cap_large = Homeostasis::new(&reg, &large, &organs).capacity(ENERGY);
        assert!(
            cap_large > cap_small,
            "the larger body holds the larger reserve"
        );
        let drain_small = derive_base_drain(
            &small,
            &organs,
            cap_small,
            whole_body_energy_density(&small, &organs),
            setpoint,
            setpoint,
            Fixed::from_int(10),
            tick,
            &anchors,
        );
        let drain_large = derive_base_drain(
            &large,
            &organs,
            cap_large,
            whole_body_energy_density(&large, &organs),
            setpoint,
            setpoint,
            Fixed::from_int(10),
            tick,
            &anchors,
        );
        assert!(
            drain_small > Fixed::ZERO && drain_large > Fixed::ZERO,
            "both drain"
        );
        assert!(
            drain_large < drain_small,
            "the larger, denser body drains a smaller fraction (Kleiber): large {drain_large:?} < small {drain_small:?}"
        );
    }

    #[test]
    fn a_colder_medium_drains_more_than_a_warm_one() {
        // The thermoregulatory term: the same body in a colder medium sheds more heat and so pays a
        // larger resting drain than in a temperate medium. Physics in (a temperature gradient), no label.
        let (organs, skin, flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let plan = body(
            (1, 1),
            vec![
                organ(skin, (1, 1)),
                organ(flesh, (1, 1)),
                organ(fat, (1, 1)),
            ],
        );
        let cap = Homeostasis::new(&reg, &plan, &organs).capacity(ENERGY);
        let setpoint = Fixed::from_int(310);
        let cold = derive_base_drain(
            &plan,
            &organs,
            cap,
            whole_body_energy_density(&plan, &organs),
            Fixed::from_int(250),
            setpoint,
            Fixed::from_int(10),
            Fixed::ONE,
            &anchors,
        );
        let warm = derive_base_drain(
            &plan,
            &organs,
            cap,
            whole_body_energy_density(&plan, &organs),
            setpoint,
            setpoint,
            Fixed::from_int(10),
            Fixed::ONE,
            &anchors,
        );
        assert!(
            cold > warm,
            "a colder medium exacts a larger thermoregulatory drain"
        );
    }

    #[test]
    fn exertion_coupling_adds_a_drain_that_scales_with_work() {
        let (organs, _skin, _flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let plan = body((1, 1), vec![organ(fat, (1, 1))]);
        let cap = Homeostasis::new(&reg, &plan, &organs).capacity(ENERGY);
        // A modest force on the WATT scale (force*velocity, no kilowatt bridge, matching the
        // watt-scale basal drain it is summed with), kept below the full-drain saturation so the
        // scaling with velocity is visible.
        let force = Fixed::ONE;
        let ed = whole_body_energy_density(&plan, &organs);
        let slow =
            derive_exertion_coupling(&plan, cap, ed, force, Fixed::ONE, Fixed::ONE, &anchors);
        let fast = derive_exertion_coupling(
            &plan,
            cap,
            ed,
            force,
            Fixed::from_int(4),
            Fixed::ONE,
            &anchors,
        );
        assert!(
            fast > slow,
            "faster work at the same force adds a larger exertion drain ({fast:?} > {slow:?})"
        );
        assert!(
            slow > Fixed::ZERO,
            "work exacts a nonzero exertion coupling"
        );
        assert!(
            fast < FRAC_MAX,
            "the exertion coupling stays below full drain here"
        );
    }

    #[test]
    fn a_high_surface_body_couples_to_the_medium_faster_than_a_compact_one() {
        // h*A/(m*c): a high-surface body couples fast, a low-surface one slowly, and a body with no
        // exchange surface does not couple at all.
        let (organs, skin, flesh, _fat) = registry();
        let anchors = MetabolicAnchors::dev_fixture();
        // High surface: a full skin plus modest flesh.
        let high = body((1, 2), vec![organ(skin, (1, 1)), organ(flesh, (1, 4))]);
        // Compact: the same flesh but a quarter skin (less exposed surface).
        let compact = body((1, 2), vec![organ(skin, (1, 4)), organ(flesh, (1, 4))]);
        let rate_high =
            derive_body_exchange_rate(&high, &organs, Fixed::from_int(10), Fixed::ONE, &anchors);
        let rate_compact =
            derive_body_exchange_rate(&compact, &organs, Fixed::from_int(10), Fixed::ONE, &anchors);
        assert!(rate_high > rate_compact, "more surface, faster coupling");
        // No exchange surface: no coupling.
        assert_eq!(
            derive_body_exchange_rate(
                &body((1, 2), vec![organ(flesh, (1, 1))]),
                &organs,
                Fixed::from_int(10),
                Fixed::ONE,
                &anchors,
            ),
            Fixed::ZERO,
            "no convective surface, no coupling"
        );
    }

    #[test]
    fn the_same_body_couples_faster_in_a_higher_h_medium() {
        // The dedup's metabolic point, EXERCISED (the dev fixtures hold h uniform, so this is what proves the
        // differentiation reaches the coupling): the SAME body couples to the medium at h*A/(m*c), so raising
        // the medium's convective coefficient h (still air ~10 to immersion water ~500) raises the coupling.
        // A being in air and one immersed in water therefore couple at their media's own h, the whole reason
        // the per-medium datum replaced the global scalar.
        let (organs, skin, flesh, _fat) = registry();
        let anchors = MetabolicAnchors::dev_fixture();
        let plan = body((1, 2), vec![organ(skin, (1, 1)), organ(flesh, (1, 4))]);
        let in_air =
            derive_body_exchange_rate(&plan, &organs, Fixed::from_int(10), Fixed::ONE, &anchors);
        let in_water =
            derive_body_exchange_rate(&plan, &organs, Fixed::from_int(500), Fixed::ONE, &anchors);
        assert!(
            in_water > in_air,
            "the same body couples faster in the higher-h medium (immersion water over still air), \
             so the per-medium h differentiation reaches the body-to-medium coupling"
        );
    }

    #[test]
    fn anchors_read_from_a_set_manifest_and_fail_loud_when_reserved() {
        // The three owner anchors load from a set manifest, and a reserved one refuses to fabricate. The
        // medium convective coefficient is no longer an anchor: it is read from the being's medium
        // (medium.rs), not this manifest, so it is absent here.
        let set = r#"
[[reserved]]
id = "metabolism.kleiber_coefficient"
basis = "fixture"
status = "set"
value = "3.4"
unit = "w"
source = "test"
[[reserved]]
id = "metabolism.body_mass_kg_scale"
basis = "fixture"
status = "set"
value = "100"
unit = "kg"
source = "test"
"#;
        let m = CalibrationManifest::from_toml_str(set).unwrap();
        let a = MetabolicAnchors::from_manifest(&m).unwrap();
        assert_eq!(a.body_mass_kg_scale, Fixed::from_int(100));
        // Sigma is no longer a manifest key: it DERIVES from the fundamentals regardless of the
        // profile, so from_manifest reads no stefan_boltzmann key and returns the derived value.
        assert_eq!(a.sigma, derived_stefan_boltzmann());
        // The shipped anchors are reserved (empty), so a from_manifest read fails loud rather than
        // fabricating a number.
        let reserved = set.replace(
            "id = \"metabolism.kleiber_coefficient\"\nbasis = \"fixture\"\nstatus = \"set\"\nvalue = \"3.4\"",
            "id = \"metabolism.kleiber_coefficient\"\nbasis = \"fixture\"\nstatus = \"reserved\"\nvalue = \"\"",
        );
        let mr = CalibrationManifest::from_toml_str(&reserved).unwrap();
        assert_eq!(
            MetabolicAnchors::from_manifest(&mr).unwrap_err(),
            CalibrationError::Reserved("metabolism.kleiber_coefficient".to_string()),
        );
    }

    #[test]
    fn derived_stefan_boltzmann_is_the_expected_q32_bits() {
        // Lock the CONSUMED sigma to its Q32.32 bits (244 x 2^-32), the round-half-even nearest of the true
        // CODATA sigma. Sigma folds into the metabolic drain (base_drain_from's radiant term) and thus into
        // the four run_world pins, so any change here (a compute change, or a retune of the representation
        // knobs that perturbs the consumed value) FAILS this test loudly rather than silently moving a pin.
        assert_eq!(derived_stefan_boltzmann(), Fixed::from_bits(244));
    }

    #[test]
    fn derived_metabolism_is_deterministic() {
        let (organs, skin, flesh, fat) = registry();
        let reg = HomeostaticRegistry::dev_default();
        let anchors = MetabolicAnchors::dev_fixture();
        let plan = body(
            (3, 4),
            vec![
                organ(skin, (1, 2)),
                organ(flesh, (3, 4)),
                organ(fat, (1, 2)),
            ],
        );
        let cap = Homeostasis::new(&reg, &plan, &organs).capacity(ENERGY);
        let run = || {
            let base = derive_base_drain(
                &plan,
                &organs,
                cap,
                whole_body_energy_density(&plan, &organs),
                Fixed::from_int(270),
                Fixed::from_int(310),
                Fixed::from_int(10),
                Fixed::ONE,
                &anchors,
            );
            let rate = derive_body_exchange_rate(
                &plan,
                &organs,
                Fixed::from_int(10),
                Fixed::ONE,
                &anchors,
            );
            (base.to_bits(), rate.to_bits())
        };
        assert_eq!(
            run(),
            run(),
            "the same body, medium, and anchors replay bit for bit"
        );
    }
}
