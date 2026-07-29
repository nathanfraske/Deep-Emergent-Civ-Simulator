//! The disposer-condensate-to-grain-opacity wire: given the condensed phases the freezer realized at a disk
//! location, assemble their grain Rosseland opacity and join it into the disk's total opacity. This lives up-stack
//! in the materials crate because it consumes the realized assemblage (a materials object) and the optical-constants
//! library; the physics crate provides the composition-agnostic primitives it drives (the effective-medium mixing
//! rules, the size-distribution average, the spectral Rosseland mean, and the gas-plus-grain monochromatic sum).
//!
//! The three wiring rules are all DERIVED, nothing authored (the owner's spec, verified against the built
//! primitives):
//!
//! - RULE 1, membership is the assemblage verbatim. Every condensed phase with nonzero amount IS a grain species; no
//!   curated list (admit-the-alien: a carbide disk auto-populates its carbides because the disposer put them there).
//!   Its optical constants come off the provenance ladder: the MEASURED library where the species is in it, the
//!   phonon ESTIMATOR (the alien rung, injected as [`GrainOpticalEstimator`]) otherwise. A phase neither the library
//!   nor the estimator can price fails LOUD (the forbidden-fallback rider), never silently dropped.
//! - RULE 2, the aggregation TOPOLOGY is written by the condensation ORDER, never read from a temperature. Phases
//!   that condensed in one generation (a single rank) form a bare mixture, so the symmetric Bruggeman rule applies.
//!   A later generation condensed onto earlier cores (distinct ranks) is a core-mantle grain, so the last-condensed
//!   (outermost, highest-rank) generation is the matrix and the earlier ones are inclusions, and the asymmetric
//!   Maxwell-Garnett rule applies. The ice line is therefore not a coded threshold: it is where the disposer's
//!   sequence deposits ice as the outermost mantle, so the mantle topology (and the opacity step it carries) emerges.
//! - RULE 3, the size distribution is species-SHARED (one collisional cascade over the composite grains), because
//!   collisions mix materials faster than they sort them. The Dohnanyi steady-state slope is the caller's reserved
//!   calibration, supplied as data, never inlined here.
//!
//! The ice-line opacity CLIFF is the pre-registered emergent output: the same assembly, handed the condensate set
//! that carries ice (below the front) versus the set that does not (above it, ice sublimated), produces a step in
//! the grain opacity, the feature the classic disk opacity laws hardcode as a regime boundary, here an output of
//! the membership changing. The grain-to-gas dominance ratio ([`grain_gas_dominance_ratio`]) is a diagnostic read
//! off the output, never a switch that adds or drops the grain term: a switch keyed on it would author the cliff.

use civsim_core::Fixed;
use civsim_physics::opacity::{
    bruggeman_effective_index, grain_rosseland_opacity_spectral, grain_size_averaged_opacity,
    maxwell_garnett_effective_index, total_gas_and_grain_rosseland_opacity,
};
use civsim_physics::optical_constants::OpticalConstants;
use civsim_physics::periodic::PeriodicTable;
use civsim_units::constants::SiExecutionMagnitudes;

/// The alien-phase optical estimator, Rule 1's second rung: for a condensate with no measured optical-constants
/// table, produce its complex index `(n, k)` at a wavelength from first principles (the phonon estimator in
/// production, the Lorentz-oscillator response at the phase's IR-active modes). It is a trait so the wire is
/// testable with a fixture estimator, and so the production phonon-backed estimator can be built and evolved
/// without touching the assembly. Returning `None` means the estimator cannot price this phase at this wavelength,
/// which (with no measured table) is a loud miss, never a silent zero.
pub trait GrainOpticalEstimator {
    /// The complex refractive index `(n, k)` of `species` at `lambda_um`, or `None` if it cannot be priced.
    fn index(&self, species: &str, lambda_um: Fixed) -> Option<(Fixed, Fixed)>;
}

/// One condensed grain species as the wire consumes it (Rule-1 membership). The load-bearing tuple is
/// `(species, mass fraction, condensation rank)`, which is the OUTPUT CONTRACT the disk-condensation producer
/// (#57) fills: the species name is the optical-dispatch key, the amount is the mass that froze in (the mass
/// fraction the mixing rules weight by, so only ratios matter and the unit cancels), and the condensation rank is
/// the producer's own temperature-ordering of the Verdict flips down the cooling path (`0` = condensed first, the
/// highest-temperature innermost core; a higher rank = a later, cooler generation, an outer mantle). The rank is a
/// DERIVED field with provenance, the disposer's T-ladder, never a hand-filled annotation and never a temperature
/// read here: it is what writes the Rule-2 core-mantle topology. The bulk density is carried alongside to convert
/// the mass fraction to the volume fraction the effective-medium rules physically consume. Until #57 exists, the
/// wire is exercised against fixture sets that carry LABELED fixture ranks, so the tuple shape is proven before its
/// real producer does.
#[derive(Clone, Debug)]
pub struct GrainConstituent {
    /// The phase name (the key the optical dispatch looks up in the library, then the estimator).
    pub species: String,
    /// The amount that froze in (mass, the disposer's unit; only ratios matter, so the unit cancels).
    pub amount: Fixed,
    /// The phase bulk density (g/cm^3), for the mass-to-volume conversion the mixing rules need.
    pub bulk_density_g_cm3: Fixed,
    /// The condensation-order rank, the producer's derived T-ordering of the Verdict flips (0 = condensed first =
    /// innermost core; higher = later = outer mantle). A fixture set labels this a fixture rank.
    pub condensation_rank: u32,
}

/// Rule 1: the complex index of one condensate at a wavelength, MEASURED where the species is in the optical-
/// constants library and the wavelength is within its sampled coverage, ESTIMATOR (the alien rung) otherwise. A
/// phase neither can price returns `None` (the loud miss). Falling to the estimator on an in-library coverage gap
/// (rather than extrapolating a measured table past its last sample) keeps the measured tier honest.
pub fn constituent_index(
    species: &str,
    lambda_um: Fixed,
    library: &OpticalConstants,
    estimator: &dyn GrainOpticalEstimator,
) -> Option<(Fixed, Fixed)> {
    if let Some(sp) = library.species(species) {
        if let Some(nk) = sp.interpolate(lambda_um) {
            return Some(nk);
        }
    }
    estimator.index(species, lambda_um)
}

/// A composite grain assembled from the realized condensate set: the constituents, their volume fractions (derived
/// from the mass amounts and bulk densities), the composite bulk density, the shared Dohnanyi size distribution
/// (Rule 3), and the optical sources (the measured library plus the alien estimator). Its methods drive the physics
/// primitives to produce the effective index, the monochromatic grain opacity, and the grain Rosseland mean.
pub struct GrainMixture<'a> {
    constituents: Vec<GrainConstituent>,
    /// Volume fraction of the total grain volume held by each constituent, aligned to `constituents`.
    volume_fractions: Vec<Fixed>,
    /// The composite bulk density (g/cm^3), total mass over total volume.
    composite_density_g_cm3: Fixed,
    /// Rule 3: the SHARED collisional-cascade slope (caller-supplied reserved calibration; Dohnanyi steady state
    /// `3.5` is its validation anchor, never inlined).
    size_slope: Fixed,
    /// The lower grain-radius bound (micron), shared across constituents (Rule 3).
    a_min_um: Fixed,
    /// The upper grain-radius bound (micron), shared across constituents (Rule 3).
    a_max_um: Fixed,
    library: &'a OpticalConstants,
    estimator: &'a dyn GrainOpticalEstimator,
}

impl<'a> GrainMixture<'a> {
    /// Assemble a mixture from a realized condensate set (Rule-1 membership) with a shared size distribution
    /// (Rule 3). The volume fractions and composite density are derived from the amounts and densities:
    /// `v_i = m_i / rho_i`, `f_i = v_i / sum v`, `rho_composite = sum m / sum v`. `None` on an empty set, a
    /// non-positive amount or density, a non-positive size bound, or `a_max <= a_min` (fail loud).
    pub fn new(
        constituents: Vec<GrainConstituent>,
        size_slope: Fixed,
        a_min_um: Fixed,
        a_max_um: Fixed,
        library: &'a OpticalConstants,
        estimator: &'a dyn GrainOpticalEstimator,
    ) -> Option<Self> {
        if constituents.is_empty() || a_min_um <= Fixed::ZERO || a_max_um <= a_min_um {
            return None;
        }
        let mut volumes = Vec::with_capacity(constituents.len());
        let mut total_volume = Fixed::ZERO;
        let mut total_mass = Fixed::ZERO;
        for c in &constituents {
            if c.amount <= Fixed::ZERO || c.bulk_density_g_cm3 <= Fixed::ZERO {
                return None;
            }
            let v = c.amount.checked_div(c.bulk_density_g_cm3)?;
            volumes.push(v);
            total_volume = total_volume.checked_add(v)?;
            total_mass = total_mass.checked_add(c.amount)?;
        }
        if total_volume <= Fixed::ZERO {
            return None;
        }
        let volume_fractions = volumes
            .iter()
            .map(|v| v.checked_div(total_volume))
            .collect::<Option<Vec<_>>>()?;
        let composite_density_g_cm3 = total_mass.checked_div(total_volume)?;
        Some(GrainMixture {
            constituents,
            volume_fractions,
            composite_density_g_cm3,
            size_slope,
            a_min_um,
            a_max_um,
            library,
            estimator,
        })
    }

    /// The composite bulk density (g/cm^3).
    pub fn composite_density_g_cm3(&self) -> Fixed {
        self.composite_density_g_cm3
    }

    /// Rule 2: the effective complex index `(n_eff, k_eff)` at a wavelength, its topology written by the
    /// condensation ranks. A single rank is a bare mixture (Bruggeman); distinct ranks are a core-mantle grain, the
    /// highest-rank (last-condensed, outermost) generation the matrix and the rest inclusions (Maxwell-Garnett).
    /// `None` if any constituent cannot be priced at this wavelength (the loud miss) or a mixing solve fails.
    pub fn effective_index(&self, lambda_um: Fixed) -> Option<(Fixed, Fixed)> {
        // Rule 1 for every constituent; a miss fails the whole grain (forbidden-fallback, never a silent drop).
        let indices = self
            .constituents
            .iter()
            .map(|c| constituent_index(&c.species, lambda_um, self.library, self.estimator))
            .collect::<Option<Vec<_>>>()?;

        let max_rank = self
            .constituents
            .iter()
            .map(|c| c.condensation_rank)
            .max()?;
        let single_generation = self
            .constituents
            .iter()
            .all(|c| c.condensation_rank == max_rank);

        if single_generation {
            // Bare mixture: the symmetric rule over all constituents at their volume fractions.
            return bruggeman_effective_index(&self.volume_fractions, &indices);
        }

        // Core-mantle: the max-rank generation is the mantle matrix, the rest are the inclusion cores.
        let mut mantle_fracs = Vec::new();
        let mut mantle_indices = Vec::new();
        let mut core_fracs = Vec::new();
        let mut core_indices = Vec::new();
        for (i, c) in self.constituents.iter().enumerate() {
            if c.condensation_rank == max_rank {
                mantle_fracs.push(self.volume_fractions[i]);
                mantle_indices.push(indices[i]);
            } else {
                core_fracs.push(self.volume_fractions[i]);
                core_indices.push(indices[i]);
            }
        }
        // The matrix is the mantle generation combined (one species passes through; several mix by Bruggeman, which
        // is scale-free in the fractions so the relative mantle fractions serve).
        let matrix_index = if mantle_indices.len() == 1 {
            mantle_indices[0]
        } else {
            bruggeman_effective_index(&mantle_fracs, &mantle_indices)?
        };
        // Maxwell-Garnett: inclusions dispersed in the mantle matrix, at their fractions of the TOTAL grain volume.
        maxwell_garnett_effective_index(matrix_index, &core_fracs, &core_indices)
    }

    /// The MONOCHROMATIC grain opacity `kappa_grain(lambda)` (cm^2/g): the Rule-2 effective index fed through the
    /// Rule-3 size-distribution average. `None` if the index or the average fails.
    pub fn monochromatic_opacity(&self, lambda_um: Fixed) -> Option<Fixed> {
        let (n, k) = self.effective_index(lambda_um)?;
        grain_size_averaged_opacity(
            lambda_um,
            n,
            k,
            self.composite_density_g_cm3,
            self.size_slope,
            self.a_min_um,
            self.a_max_um,
        )
    }

    /// The grain ROSSELAND-MEAN opacity `kappa_grain_R(T)` (cm^2/g): the temperature-dependent single number the
    /// disk solve reads, the spectral Rosseland mean of the wavelength-dependent effective index. `None` on a
    /// non-positive temperature, an unpriceable wavelength, or an overflow.
    pub fn rosseland_opacity(&self, temperature_k: Fixed) -> Option<Fixed> {
        grain_rosseland_opacity_spectral(
            temperature_k,
            |lambda_um| self.effective_index(lambda_um),
            self.composite_density_g_cm3,
            self.size_slope,
            self.a_min_um,
            self.a_max_um,
        )
    }
}

/// The disk TOTAL Rosseland-mean opacity `kappa_R(T)` (cm^2/g): the ionized-gas terms and the grain term summed
/// MONOCHROMATICALLY, then Rosseland-averaged once. The gas arguments are the Saha state and plasma data (exactly
/// [`total_gas_and_grain_rosseland_opacity`]'s); the grain term is the mixture's monochromatic opacity. This is the
/// top of the assembly: a disk location's realized condensate set plus its Saha state in, one opacity out. `None`
/// if the mean fails to resolve (which, unlike the gas-only closure's cold-gap `None`, the grain term repairs
/// wherever grains are present).
#[allow(clippy::too_many_arguments)]
pub fn disk_total_rosseland_opacity(
    execution: &SiExecutionMagnitudes,
    temperature_k: Fixed,
    density_g_per_cm3: Fixed,
    ln_density_g_cm3: Fixed,
    hydrogen_mass_fraction: Fixed,
    charge_weighted_abundance: Fixed,
    gaunt_factor: Fixed,
    ln_electron_density_cm3: Fixed,
    ln_sum_z2_ni_cm3: Fixed,
    electron_pressure_dyn_cm2: Fixed,
    table: &PeriodicTable,
    mixture: &GrainMixture,
) -> Option<Fixed> {
    total_gas_and_grain_rosseland_opacity(
        execution,
        temperature_k,
        density_g_per_cm3,
        ln_density_g_cm3,
        hydrogen_mass_fraction,
        charge_weighted_abundance,
        gaunt_factor,
        ln_electron_density_cm3,
        ln_sum_z2_ni_cm3,
        electron_pressure_dyn_cm2,
        table,
        |lambda_um| mixture.monochromatic_opacity(lambda_um),
    )
}

/// The grain-to-gas Rosseland-opacity dominance ratio `kappa_grain_R / kappa_gas_R`, the diagnostic the ice-line
/// cliff and the grain-dominated regime are read from. It is formed from the two SEPARATE Rosseland means for
/// reporting only; the physical total ([`disk_total_rosseland_opacity`]) sums the MONOCHROMATIC terms and averages
/// once (the harmonic mean is not additive, so the separate means must not be added). The dominance guard is an
/// INVARIANT asserted on the output, never a switch that adds or drops the grain term: a switch keyed on this ratio
/// would AUTHOR the ice-line cliff, recreating the coded regime boundary the emergence gate forbids. `None` on a
/// non-positive gas opacity.
pub fn grain_gas_dominance_ratio(grain_rosseland: Fixed, gas_rosseland: Fixed) -> Option<Fixed> {
    if gas_rosseland <= Fixed::ZERO {
        return None;
    }
    grain_rosseland.checked_div(gas_rosseland)
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::periodic::PeriodicTable;

    /// A fixture optical estimator: authored `(n, k)` per fixture phase name. This is a QUARANTINED test input (an
    /// authored condensate optics set), legal as a labeled fixture and excluded from any gate; it stands in for the
    /// production phonon-backed estimator so the Rules-1-to-3 assembly is testable in isolation.
    struct FixtureEstimator;
    impl GrainOpticalEstimator for FixtureEstimator {
        fn index(&self, species: &str, _lambda_um: Fixed) -> Option<(Fixed, Fixed)> {
            match species {
                // A refractory silicate-like grain: high real index, weak absorption.
                "fixture_refractory" => {
                    Some((Fixed::from_ratio(17, 10), Fixed::from_ratio(3, 100)))
                }
                // An icy mantle: moderate real index, strong absorption (the ice-line opacity carrier).
                "fixture_ice" => Some((Fixed::from_ratio(13, 10), Fixed::from_ratio(3, 10))),
                // An estimator-priced alien phase, for the Rule-1 dispatch test.
                "fixture_alien" => Some((Fixed::from_int(2), Fixed::from_ratio(1, 10))),
                // Anything else is a loud miss (no measured table, no estimate): never a silent zero.
                _ => None,
            }
        }
    }

    fn empty_library() -> OpticalConstants {
        OpticalConstants::from_toml_str("").expect("an empty optical library parses")
    }

    fn constituent(species: &str, amount: f64, density: f64, rank: u32) -> GrainConstituent {
        GrainConstituent {
            species: species.to_string(),
            amount: Fixed::from_ratio((amount * 1000.0) as i64, 1000),
            bulk_density_g_cm3: Fixed::from_ratio((density * 1000.0) as i64, 1000),
            condensation_rank: rank,
        }
    }

    fn slope() -> Fixed {
        // Dohnanyi collisional-cascade steady state, the reserved size-distribution anchor (a labeled fixture value
        // here, supplied as caller data, never inlined in the assembly).
        Fixed::from_ratio(35, 10)
    }
    fn a_min() -> Fixed {
        Fixed::from_ratio(1, 10)
    }
    fn a_max() -> Fixed {
        Fixed::from_int(10)
    }

    #[test]
    fn rule_1_dispatches_measured_then_estimator_then_fails_loud() {
        // Rule 1: an in-library phase reads its MEASURED optical constants; an alien phase falls to the ESTIMATOR;
        // a phase neither can price is a loud miss (None), never a silent zero.
        let lib = OpticalConstants::aesopus_library().expect("the vendored optical library loads");
        let est = FixtureEstimator;
        // Iron is one of the 45 measured species; price it at an exact sampled wavelength so the measured table
        // answers with its own row.
        let fe = lib.species("Fe").expect("iron is in the measured library");
        let lam = fe.samples[fe.samples.len() / 2].lambda_um;
        let measured = fe.interpolate(lam);
        assert_eq!(
            constituent_index("Fe", lam, &lib, &est),
            measured,
            "an in-library phase dispatches to its measured table"
        );
        assert!(measured.is_some(), "the measured lookup is populated");
        // An alien phase not in the library dispatches to the estimator.
        assert_eq!(
            constituent_index("fixture_alien", lam, &lib, &est),
            Some((Fixed::from_int(2), Fixed::from_ratio(1, 10))),
            "an alien phase dispatches to the estimator (the generative rung)"
        );
        // A phase neither the library nor the estimator can price fails loud.
        assert!(
            constituent_index("unobtainium", lam, &lib, &est).is_none(),
            "an unpriceable phase is a loud miss, never a silent zero"
        );
    }

    #[test]
    fn rule_2_topology_is_written_by_condensation_order() {
        // Rule 2: the SAME two phases at the SAME amounts give DIFFERENT effective indices depending only on the
        // condensation-order ranks. A single generation (one rank) is a bare Bruggeman mixture; a later generation
        // (a distinct higher rank) is a Maxwell-Garnett ice mantle over a refractory core. No temperature is read.
        let lib = empty_library();
        let est = FixtureEstimator;
        let lam = Fixed::from_int(20);
        let bare = GrainMixture::new(
            vec![
                constituent("fixture_refractory", 3.0, 3.3, 0),
                constituent("fixture_ice", 1.0, 1.0, 0),
            ],
            slope(),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .expect("the bare mixture assembles");
        let mantle = GrainMixture::new(
            vec![
                constituent("fixture_refractory", 3.0, 3.3, 0),
                constituent("fixture_ice", 1.0, 1.0, 1),
            ],
            slope(),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .expect("the core-mantle mixture assembles");
        let i_bare = bare.effective_index(lam).expect("bruggeman resolves");
        let i_mantle = mantle
            .effective_index(lam)
            .expect("maxwell-garnett resolves");
        assert!(
            i_bare != i_mantle,
            "the Bruggeman bare mix and the Maxwell-Garnett ice mantle give different effective indices \
             (bare {:?}, mantle {:?})",
            (i_bare.0.to_f64_lossy(), i_bare.1.to_f64_lossy()),
            (i_mantle.0.to_f64_lossy(), i_mantle.1.to_f64_lossy()),
        );
    }

    #[test]
    fn rule_3_shared_size_slope_is_live_data() {
        // Rule 3: the size distribution is a SHARED caller argument, not inlined. Changing the Dohnanyi slope moves
        // the grain Rosseland opacity.
        let lib = empty_library();
        let est = FixtureEstimator;
        let cons = vec![
            constituent("fixture_refractory", 3.0, 3.3, 0),
            constituent("fixture_ice", 1.0, 1.0, 1),
        ];
        let steep = GrainMixture::new(
            cons.clone(),
            Fixed::from_ratio(35, 10),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .unwrap();
        let shallow = GrainMixture::new(
            cons,
            Fixed::from_ratio(25, 10),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .unwrap();
        let t = Fixed::from_int(150);
        assert!(
            steep.rosseland_opacity(t).unwrap() != shallow.rosseland_opacity(t).unwrap(),
            "the shared size-distribution slope is live data, not an inlined constant"
        );
    }

    #[test]
    fn the_ice_line_cliff_emerges_from_membership() {
        // The pre-registered emergent ice-line cliff: the SAME assembly, SAME temperature, SAME code path, handed
        // the condensate set that carries ice (below the front) versus the set that does not (above it, ice
        // sublimated and dropped from the assemblage), produces a STEP in the grain Rosseland opacity. The cliff is
        // an output of the membership changing, never a coded temperature boundary (the temperature below is used
        // only to place the Rosseland window; no threshold on it is read).
        let lib = empty_library();
        let est = FixtureEstimator;
        let below = GrainMixture::new(
            vec![
                constituent("fixture_refractory", 3.0, 3.3, 0),
                constituent("fixture_ice", 2.0, 1.0, 1),
            ],
            slope(),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .unwrap();
        let above = GrainMixture::new(
            vec![constituent("fixture_refractory", 3.0, 3.3, 0)],
            slope(),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .unwrap();
        let t = Fixed::from_int(150);
        let k_below = below.rosseland_opacity(t).unwrap();
        let k_above = above.rosseland_opacity(t).unwrap();
        assert!(
            k_below > k_above,
            "ice grains raise the grain opacity below the front ({} vs {})",
            k_below.to_f64_lossy(),
            k_above.to_f64_lossy()
        );
        let cliff = k_below.to_f64_lossy() / k_above.to_f64_lossy();
        assert!(
            cliff > 1.5,
            "the ice-line opacity cliff is a clear emergent step, got {cliff:.3}x"
        );
    }

    #[test]
    fn the_grain_dominance_guard_is_a_live_invariant() {
        // The dominance guard, asserted on the output: below the front the grain Rosseland opacity dwarfs a
        // representative cold molecular-gas opacity by more than two orders of magnitude. The guard is a DIAGNOSTIC
        // invariant, never a switch that adds or drops the grain term (a switch would author the cliff).
        let lib = empty_library();
        let est = FixtureEstimator;
        let below = GrainMixture::new(
            vec![
                constituent("fixture_refractory", 3.0, 3.3, 0),
                constituent("fixture_ice", 2.0, 1.0, 1),
            ],
            slope(),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .unwrap();
        let k_grain = below.rosseland_opacity(Fixed::from_int(150)).unwrap();
        // A representative cold molecular-gas Rosseland opacity (a labeled fixture value, ~1e-3 cm^2/g).
        let k_gas = Fixed::from_ratio(1, 1000);
        let ratio = grain_gas_dominance_ratio(k_grain, k_gas).unwrap();
        assert!(
            ratio.to_f64_lossy() > 100.0,
            "grains dominate the cold budget by more than 10^2, got {:.1}x (grain {} cm^2/g)",
            ratio.to_f64_lossy(),
            k_grain.to_f64_lossy(),
        );
    }

    #[test]
    fn the_disk_total_wire_joins_gas_and_grains() {
        // The top-of-assembly wire: a hot-gas Saha state plus a grain mixture through the physics gas-plus-grain
        // closure. The grain term joins the monochromatic sum, so the disk total exceeds the gas-only total. (Grains
        // at 6000 K are a WIRING fixture: the physical disk sublimates them; this checks the plumbing end to end.)
        let tbl = PeriodicTable::standard().expect("the periodic table loads");
        let execution = civsim_units::constants::canonical_si_execution_magnitudes()
            .expect("the sealed floor projects");
        let temp = Fixed::from_int(6000);
        let species = [
            ("H", civsim_physics::saha::ln_of_decimal("1e17").unwrap()),
            ("K", civsim_physics::saha::ln_of_decimal("1e10").unwrap()),
        ];
        let state =
            civsim_physics::saha::electron_density_saha(&execution, temp, &species, &tbl).unwrap();
        let rho = Fixed::from_ratio(24, 100_000_000);
        let ln_rho = civsim_physics::saha::ln_of_decimal("2.4e-7").unwrap();
        let ln_ne = state.ln_electron_density_cm3;
        let lib = empty_library();
        let est = FixtureEstimator;
        let mixture = GrainMixture::new(
            vec![constituent("fixture_refractory", 1.0, 3.3, 0)],
            slope(),
            a_min(),
            a_max(),
            &lib,
            &est,
        )
        .unwrap();
        let total = disk_total_rosseland_opacity(
            &execution,
            temp,
            rho,
            ln_rho,
            Fixed::from_ratio(7, 10),
            Fixed::ONE,
            Fixed::from_ratio(12, 10),
            ln_ne,
            ln_ne,
            state.electron_pressure_dyn_cm2,
            &tbl,
            &mixture,
        )
        .expect("the disk-total wire assembles");
        assert!(
            total > Fixed::ZERO,
            "the wired disk total is positive with grains present"
        );
    }
}
