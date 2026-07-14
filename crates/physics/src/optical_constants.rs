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

//! The OPTICAL-CONSTANTS library (`crates/physics/data/optical_constants.toml`), the measured [M] inputs the
//! disk-opacity GENERATOR's grain terms consume: per dust species the complex refractive index `n(lambda),
//! k(lambda)` sampled across the protoplanetary-disk wavelength range. The grain absorption (the Rayleigh/Mie step,
//! later slices) reads `m = n + i k` at each wavelength; this module only LOADS and validates the cited tables.
//!
//! These are a two-electron-quantum-hard measured quantity for each material (the dielectric response is not
//! derivable at the floor level), so they enter as cited [M] data, the same tier the H- cross section set the
//! precedent for. The generator's whole thesis over a fitted opacity ladder is that a carbon-rich or metal-poor
//! disk is a different MEMBERSHIP over this same library (composition-keyed on the disposer's condensate fractions),
//! not a rewrite, and an exotic condensate with no measured `n,k` is handled later by a Lorentz-Drude ESTIMATOR
//! from the material's own derived band gap and plasma frequency (a later admit-the-alien slice), never a
//! missing-row hard-fail.
//!
//! PROVENANCE (tier-honest, per species citation): silicate and crystalline water ice are PRIMARY (pulled from and
//! self-checked against Draine 2003's and Warren-Brandt 2008's own files); iron/troilite/carbon (later slices) are
//! SECONDARY compilations, tagged as such. The loader re-verifies the reference values (the H- peak-gate pattern):
//! a corruption of a cited row fails the build. HONEST LIMITS: the tables are LOG-SAMPLED subsets of the full files
//! (dense enough for the Rosseland grid, the resonance bands bracketed), Warren-Brandt is crystalline ice only (the
//! amorphous branch, Mastrapa/Hudgins, is an uncovered data-membership gap), and `k` below the Q32.32 floor
//! (`~2.3e-10`, the ultraviolet transparent tail of ice) stores as zero (the transparent limit). No consumer is
//! wired in any pinned run path (byte-neutral).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the optical-constants library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpticalConstantsError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A species carries no citation (every row is real-with-source).
    MissingSource(String),
    /// The provenance tier is not one of the recognised tiers (fail-closed).
    BadTier(String),
    /// A species name appears twice.
    Duplicate(String),
    /// A species carries no samples.
    Empty(String),
    /// A non-physical value: `n <= 0` or `k < 0`.
    NotPhysical(String),
    /// The wavelength grid is not strictly increasing (needed for the interpolation the grain step does).
    NotMonotonic(String),
}

impl fmt::Display for OpticalConstantsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpticalConstantsError::Parse(m) => write!(f, "optical-constants parse error: {m}"),
            OpticalConstantsError::BadValue(m) => write!(f, "optical-constants value error: {m}"),
            OpticalConstantsError::MissingSource(m) => {
                write!(f, "optical-constants species without citation: {m}")
            }
            OpticalConstantsError::BadTier(m) => write!(f, "optical-constants bad tier: {m}"),
            OpticalConstantsError::Duplicate(m) => write!(f, "duplicate optical species: {m}"),
            OpticalConstantsError::Empty(m) => write!(f, "optical species with no samples: {m}"),
            OpticalConstantsError::NotPhysical(m) => write!(f, "non-physical n or k: {m}"),
            OpticalConstantsError::NotMonotonic(m) => write!(f, "non-monotonic wavelength: {m}"),
        }
    }
}

impl std::error::Error for OpticalConstantsError {}

/// The provenance tier of a species table, a type-level honesty tag (fail-closed on an unknown tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceTier {
    /// Pulled from and self-checked against the original author's data file.
    Primary,
    /// A faithful copy from a compilation that preserves the original grid (a step below primary).
    Secondary,
    /// A compilation that RE-GRIDDED the data (the sampled wavelengths are interpolated, the weakest tier).
    SecondaryRegridded,
}

impl ProvenanceTier {
    fn parse(s: &str) -> Result<Self, OpticalConstantsError> {
        match s {
            "primary" => Ok(ProvenanceTier::Primary),
            "secondary" => Ok(ProvenanceTier::Secondary),
            "secondary_regridded" => Ok(ProvenanceTier::SecondaryRegridded),
            other => Err(OpticalConstantsError::BadTier(other.to_string())),
        }
    }
}

/// One sampled complex refractive index: the wavelength (micron) and the real and imaginary parts `n`, `k`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefractiveIndex {
    /// Wavelength in micron.
    pub lambda_um: Fixed,
    /// The real refractive index `n` (> 0).
    pub n: Fixed,
    /// The imaginary refractive index `k` (>= 0; a sub-floor ultraviolet value stores as zero, the transparent
    /// limit).
    pub k: Fixed,
}

/// One species' cited optical-constants table.
#[derive(Debug, Clone)]
pub struct OpticalSpecies {
    /// The species name (the composition key the grain step dispatches on).
    pub name: String,
    /// The primary/compilation citation, required non-empty.
    pub citation: String,
    /// The provenance tier (the honesty tag).
    pub tier: ProvenanceTier,
    /// The sampled `(lambda, n, k)` rows, strictly increasing in wavelength.
    pub samples: Vec<RefractiveIndex>,
}

impl OpticalSpecies {
    /// Interpolate `n` and `k` at wavelength `lambda_um` by linear interpolation in wavelength between the two
    /// bracketing sampled rows (the grain step reads `m = n + i k` at arbitrary Rosseland-grid wavelengths, which
    /// fall between the log-sampled rows). The grid is strictly increasing (a load invariant), so a binary search
    /// brackets the target. `None` outside the sampled coverage (a per-species coverage gap, e.g. iron beyond 286
    /// micron, which the grain step handles rather than extrapolating a measured table). Linear-in-wavelength is the
    /// honest interpolation over a dense log-spaced grid; the resonance bands are bracketed by extra sampled rows so
    /// a feature is not stepped over.
    pub fn interpolate(&self, lambda_um: Fixed) -> Option<(Fixed, Fixed)> {
        let s = &self.samples;
        if lambda_um < s[0].lambda_um || lambda_um > s[s.len() - 1].lambda_um {
            return None; // outside the sampled coverage
        }
        // Bracket: find lo with s[lo].lambda <= lambda <= s[lo+1].lambda.
        let mut lo = 0usize;
        let mut hi = s.len() - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if s[mid].lambda_um <= lambda_um {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let a = &s[lo];
        let b = &s[hi];
        if lambda_um == a.lambda_um {
            return Some((a.n, a.k));
        }
        if lambda_um == b.lambda_um {
            return Some((b.n, b.k));
        }
        let span = b.lambda_um.checked_sub(a.lambda_um)?;
        let t = lambda_um.checked_sub(a.lambda_um)?.checked_div(span)?;
        let n = a.n.checked_add(t.checked_mul(b.n.checked_sub(a.n)?)?)?;
        let k = a.k.checked_add(t.checked_mul(b.k.checked_sub(a.k)?)?)?;
        Some((n, k))
    }
}

/// The loaded optical-constants library, keyed by species name.
#[derive(Debug, Clone, Default)]
pub struct OpticalConstants {
    species: BTreeMap<String, OpticalSpecies>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RawFile {
    #[serde(default)]
    species: Vec<RawSpecies>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RawSpecies {
    name: String,
    citation: String,
    provenance_tier: String,
    /// Each row is `[lambda_um, n, k]` as decimal strings (kept exact, parsed to fixed-point on load).
    samples: Vec<[String; 3]>,
}

/// Parse one decimal string to `Fixed` through the exact `BigRat` path (a sub-floor magnitude rounds to zero).
fn fixed_from_decimal(s: &str) -> Result<Fixed, OpticalConstantsError> {
    let br = BigRat::from_decimal_str(s).map_err(OpticalConstantsError::BadValue)?;
    let bits = br
        .round_to_scale(Fixed::FRAC_BITS)
        .ok_or_else(|| OpticalConstantsError::BadValue(format!("{s} out of range")))?;
    Fixed::from_bits_i128(bits)
        .ok_or_else(|| OpticalConstantsError::BadValue(format!("{s} out of range")))
}

impl OpticalConstants {
    /// Parse and validate the library from a TOML string. Every species must carry a citation and a recognised
    /// tier, hold at least one sample, keep `n > 0` and `k >= 0`, and be strictly increasing in wavelength.
    pub fn from_toml_str(s: &str) -> Result<Self, OpticalConstantsError> {
        let file: RawFile =
            toml::from_str(s).map_err(|e| OpticalConstantsError::Parse(e.to_string()))?;
        let mut species = BTreeMap::new();
        for raw in file.species {
            if raw.citation.trim().is_empty() {
                return Err(OpticalConstantsError::MissingSource(raw.name));
            }
            let tier = ProvenanceTier::parse(&raw.provenance_tier)?;
            if raw.samples.is_empty() {
                return Err(OpticalConstantsError::Empty(raw.name));
            }
            let mut samples = Vec::with_capacity(raw.samples.len());
            let mut prev_lambda: Option<Fixed> = None;
            for row in &raw.samples {
                let lambda_um = fixed_from_decimal(&row[0])?;
                let n = fixed_from_decimal(&row[1])?;
                let k = fixed_from_decimal(&row[2])?;
                if n <= Fixed::ZERO || k < Fixed::ZERO {
                    return Err(OpticalConstantsError::NotPhysical(format!(
                        "{} at {}",
                        raw.name, row[0]
                    )));
                }
                if let Some(prev) = prev_lambda {
                    if lambda_um <= prev {
                        return Err(OpticalConstantsError::NotMonotonic(format!(
                            "{} at {}",
                            raw.name, row[0]
                        )));
                    }
                }
                prev_lambda = Some(lambda_um);
                samples.push(RefractiveIndex { lambda_um, n, k });
            }
            let entry = OpticalSpecies {
                name: raw.name.clone(),
                citation: raw.citation,
                tier,
                samples,
            };
            if species.insert(raw.name.clone(), entry).is_some() {
                return Err(OpticalConstantsError::Duplicate(raw.name));
            }
        }
        Ok(OpticalConstants { species })
    }

    /// Load the standard library from the checked-in data file.
    pub fn standard() -> Result<Self, OpticalConstantsError> {
        Self::from_toml_str(include_str!("../data/optical_constants.toml"))
    }

    /// The cited table for a species, or `None` if it is not in the library (the caller escalates to the estimator).
    pub fn species(&self, name: &str) -> Option<&OpticalSpecies> {
        self.species.get(name)
    }

    /// The species names in the library, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.species.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> OpticalConstants {
        OpticalConstants::standard().expect("the optical-constants library loads")
    }

    /// Find the sampled row whose wavelength is closest to `target_um` (a test helper for the reference checks).
    fn nearest(sp: &OpticalSpecies, target_um: f64) -> &RefractiveIndex {
        sp.samples
            .iter()
            .min_by(|a, b| {
                let da = (a.lambda_um.to_f64_lossy() - target_um).abs();
                let db = (b.lambda_um.to_f64_lossy() - target_um).abs();
                da.partial_cmp(&db).unwrap()
            })
            .unwrap()
    }

    #[test]
    fn the_library_loads_the_primary_species() {
        let l = lib();
        assert!(l.species("astronomical_silicate").is_some());
        assert!(l.species("water_ice_crystalline").is_some());
        assert_eq!(
            l.species("astronomical_silicate").unwrap().tier,
            ProvenanceTier::Primary
        );
    }

    #[test]
    fn the_silicate_reference_values_re_verify_against_draine() {
        // The standing re-verification gate (the H- peak-gate pattern): the loaded silicate n,k reproduce Draine
        // 2003's own file values at the reference wavelengths, so a corruption of a cited row fails the build.
        let l = lib();
        let sil = l.species("astronomical_silicate").unwrap();
        let at_1 = nearest(sil, 1.0);
        assert!(
            (at_1.n.to_f64_lossy() - 1.6863).abs() < 1e-3
                && (at_1.k.to_f64_lossy() - 0.030770).abs() < 1e-4,
            "silicate ~1 micron is n=1.6863 k=0.03077, got n={} k={}",
            at_1.n.to_f64_lossy(),
            at_1.k.to_f64_lossy()
        );
        let at_10 = nearest(sil, 10.0);
        assert!(
            (at_10.n.to_f64_lossy() - 1.3701).abs() < 1e-2
                && (at_10.k.to_f64_lossy() - 0.93910).abs() < 1e-2,
            "silicate ~10 micron is n=1.3701 k=0.9391 (the Si-O band), got n={} k={}",
            at_10.n.to_f64_lossy(),
            at_10.k.to_f64_lossy()
        );
    }

    #[test]
    fn the_ice_reference_values_re_verify_against_warren_brandt() {
        let l = lib();
        let ice = l.species("water_ice_crystalline").unwrap();
        let at_1 = nearest(ice, 1.0);
        assert!(
            (at_1.n.to_f64_lossy() - 1.3015).abs() < 1e-3
                && (at_1.k.to_f64_lossy() - 1.62e-6).abs() < 1e-7,
            "ice 1 micron is n=1.3015 k=1.62e-6, got n={} k={}",
            at_1.n.to_f64_lossy(),
            at_1.k.to_f64_lossy()
        );
        let at_100 = nearest(ice, 100.0);
        assert!(
            (at_100.n.to_f64_lossy() - 1.8654).abs() < 1e-3,
            "ice 100 micron is n=1.8654, got n={}",
            at_100.n.to_f64_lossy()
        );
    }

    #[test]
    fn the_iron_reference_values_re_verify_against_ordal() {
        // Iron is PRIMARY (Ordal 1988 via the CC0 digitization of the paper's tables); the metal's n,k grow large
        // into the infrared (n=95, k=182 at 100 micron), which the wide Q32.32 range holds.
        let l = lib();
        let fe = l.species("metallic_iron").unwrap();
        assert_eq!(fe.tier, ProvenanceTier::Primary);
        let at_1 = nearest(fe, 1.0);
        assert!(
            (at_1.n.to_f64_lossy() - 3.1838).abs() < 1e-2
                && (at_1.k.to_f64_lossy() - 4.1442).abs() < 1e-2,
            "iron 1 micron is n=3.184 k=4.144, got n={} k={}",
            at_1.n.to_f64_lossy(),
            at_1.k.to_f64_lossy()
        );
        let at_100 = nearest(fe, 100.0);
        assert!(
            (at_100.n.to_f64_lossy() - 95.358).abs() < 0.5
                && (at_100.k.to_f64_lossy() - 181.95).abs() < 1.0,
            "iron 100 micron is n=95.36 k=181.95, got n={} k={}",
            at_100.n.to_f64_lossy(),
            at_100.k.to_f64_lossy()
        );
    }

    #[test]
    fn the_troilite_loads_as_secondary_and_re_verifies() {
        // Troilite is SECONDARY (Henning-Stognienko 1996 via the optool compilation, original grid preserved), the
        // honesty tag one tier below the primary silicate/ice/iron.
        let l = lib();
        let fes = l.species("troilite_fes").unwrap();
        assert_eq!(fes.tier, ProvenanceTier::Secondary);
        let at_1 = nearest(fes, 1.0);
        assert!(
            (at_1.n.to_f64_lossy() - 6.0533).abs() < 1e-2
                && (at_1.k.to_f64_lossy() - 2.1684).abs() < 1e-2,
            "troilite 1 micron is n=6.053 k=2.168, got n={} k={}",
            at_1.n.to_f64_lossy(),
            at_1.k.to_f64_lossy()
        );
    }

    #[test]
    fn the_refractory_organics_load_as_secondary_and_re_verify() {
        // The refractory-organics CHON carrier (~25% of the Pollack solids, the largest single dust carrier), the
        // Henning-Stognienko 1996 "organics" set (via optool, the same confirmed-identity secondary source as
        // troilite, NOT the identity-ambiguous Zubko amorphous carbon which stays held). The file header confirms
        // the material is "Organics / CHON", audited against source before loading.
        let l = lib();
        let org = l.species("refractory_organics_chon").unwrap();
        assert_eq!(org.tier, ProvenanceTier::Secondary);
        let at_1 = nearest(org, 1.0);
        assert!(
            (at_1.n.to_f64_lossy() - 1.6343).abs() < 1e-3
                && (at_1.k.to_f64_lossy() - 0.012467).abs() < 1e-4,
            "organics 1 micron is n=1.6343 k=0.012467, got n={} k={}",
            at_1.n.to_f64_lossy(),
            at_1.k.to_f64_lossy()
        );
        let at_100 = nearest(org, 100.0);
        assert!(
            (at_100.n.to_f64_lossy() - 2.1448).abs() < 1e-3,
            "organics 100 micron is n=2.1448, got n={}",
            at_100.n.to_f64_lossy()
        );
    }

    #[test]
    fn every_species_is_physical_and_monotonic() {
        // n > 0 and k >= 0 everywhere, and the wavelength grid strictly increases (the load-time invariants).
        let l = lib();
        for name in l.names() {
            let sp = l.species(name).unwrap();
            assert!(!sp.samples.is_empty());
            let mut prev = Fixed::ZERO;
            for row in &sp.samples {
                assert!(row.n > Fixed::ZERO, "{name}: n>0");
                assert!(row.k >= Fixed::ZERO, "{name}: k>=0");
                assert!(row.lambda_um > prev, "{name}: strictly increasing lambda");
                prev = row.lambda_um;
            }
        }
    }

    #[test]
    fn the_interpolation_brackets_a_target_and_gaps_outside_coverage() {
        // At a sampled wavelength it returns that row; between rows it interpolates; outside the sampled coverage it
        // returns None (the honest gap, not an extrapolation of the measured table).
        let l = lib();
        let sil = l.species("astronomical_silicate").unwrap();
        let (n, _k) = sil.interpolate(Fixed::from_int(10)).unwrap();
        assert!(
            (n.to_f64_lossy() - 1.3701).abs() < 1e-2,
            "at the 10 micron sampled row n=1.3701, got {}",
            n.to_f64_lossy()
        );
        let between = sil.interpolate(Fixed::from_ratio(105, 10));
        assert!(between.is_some(), "interpolates between sampled rows");
        assert_eq!(
            sil.interpolate(Fixed::from_int(5000)),
            None,
            "beyond the 2000 micron coverage it gaps to None"
        );
    }

    #[test]
    fn a_missing_citation_fails_closed() {
        let bad = r#"
[[species]]
name = "mystery"
citation = ""
provenance_tier = "primary"
samples = [["1.0", "1.5", "0.01"]]
"#;
        assert!(matches!(
            OpticalConstants::from_toml_str(bad),
            Err(OpticalConstantsError::MissingSource(_))
        ));
    }

    #[test]
    fn an_unknown_tier_fails_closed() {
        let bad = r#"
[[species]]
name = "mystery"
citation = "somewhere"
provenance_tier = "guessed"
samples = [["1.0", "1.5", "0.01"]]
"#;
        assert!(matches!(
            OpticalConstants::from_toml_str(bad),
            Err(OpticalConstantsError::BadTier(_))
        ));
    }

    #[test]
    fn a_non_monotonic_grid_fails_closed() {
        let bad = r#"
[[species]]
name = "mystery"
citation = "somewhere"
provenance_tier = "primary"
samples = [["2.0", "1.5", "0.01"], ["1.0", "1.4", "0.02"]]
"#;
        assert!(matches!(
            OpticalConstants::from_toml_str(bad),
            Err(OpticalConstantsError::NotMonotonic(_))
        ));
    }
}
