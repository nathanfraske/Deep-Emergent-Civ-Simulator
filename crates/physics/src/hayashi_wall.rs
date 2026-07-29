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

//! BHAC15 HAYASHI-WALL CANDIDATE GRID: top-of-track `T_eff` as a function of stellar mass, read from the vendored
//! model tracks in `data/bhac15/` through `data/hayashi_wall.toml`.
//!
//! # WHAT IT SUPPLIES, AND WHY IT IS A GRID
//!
//! Pre-migration stellar code can read this grid instead of a single authored solar constant. Canonical Stage 1
//! cannot consume it until the theory-model rows complete Residue `[E]` admission. The read interpolates in the
//! table's own mass spacing and never snaps to the nearest row, so a `0.55 Msun` query is not quantized to `0.5`
//! or `0.6`. Evidence custody and the remaining admission obligations are tracked in
//! `docs/working/ABIOTIC_EVIDENCE_DEBT.md`.
//!
//! # THE CHORD AND THE DOMAIN
//!
//! The wall is a CHORD OVER AGE: [`WallReading`] carries the drift band (`drift_lo_k`, `drift_hi_k`, the min and
//! max `T_eff` over the first 2 Myr of the descent) beside the top-of-track value, so a consumer reads the wall
//! with its age uncertainty. The domain guards are TWO-ENDED and refuse BY NAME: below the
//! table's `0.010 Msun` and above its `1.400 Msun`, the high side pointing at the planned radiative-branch
//! dispatch. The rows are theory-model evidence conditioned on solar `[M/H] = 0`, so they are proposed Residue
//! `[E]`, not measurements. A different composition requires a derived branch or refusal.

use std::path::Path;

use civsim_core::Fixed;
use serde::Deserialize;

const ZERO: Fixed = Fixed::ZERO;

#[derive(Debug, Deserialize)]
struct WallFile {
    #[serde(default)]
    source: String,
    #[serde(default)]
    grade: String,
    #[serde(default)]
    composition: String,
    #[serde(default)]
    epoch: String,
    #[serde(default)]
    wall: Vec<WallRowRaw>,
}

#[derive(Debug, Deserialize)]
struct WallRowRaw {
    mass_msun: String,
    wall_teff_k: String,
    drift_lo_k: String,
    drift_hi_k: String,
}

/// One mass row of the wall grid: the stellar mass and the top-of-track `T_eff` with its age-drift band, all in
/// fixed point parsed from the cited decimal strings (no f64 on the value path).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HayashiWallRow {
    /// The stellar mass `M / Msun`.
    pub mass_ratio: Fixed,
    /// The wall `T_eff` (K): the Teff at the earliest tabulated age (top of the Hayashi track).
    pub wall_teff_k: Fixed,
    /// The coolest `T_eff` (K) over the first 2 Myr (the low edge of the age-drift band).
    pub drift_lo_k: Fixed,
    /// The warmest `T_eff` (K) over the first 2 Myr (the high edge of the age-drift band).
    pub drift_hi_k: Fixed,
}

/// A wall read at a queried mass: the interpolated wall `T_eff` and its drift band, carrying the mass so the value
/// cannot be quoted without the star it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WallReading {
    /// The queried stellar mass `M / Msun`.
    pub mass_ratio: Fixed,
    /// The interpolated wall `T_eff` (K).
    pub wall_teff_k: Fixed,
    /// The interpolated low edge of the age-drift band (K), the chord field.
    pub drift_lo_k: Fixed,
    /// The interpolated high edge of the age-drift band (K), the chord field.
    pub drift_hi_k: Fixed,
}

/// Why a wall read refused. Every variant is a refusal to answer, never a fabricated wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallRefusal {
    /// The mass is below the grid's least tabulated mass (the hydrogen-burning-limit end). No sub-brown-dwarf
    /// track exists to interpolate.
    BelowDomain {
        /// The grid's least tabulated mass `M / Msun`.
        min_mass_ratio: Fixed,
    },
    /// The mass is above the grid's greatest tabulated mass. Higher-mass pre-main-sequence stars leave the
    /// convective Hayashi track for a radiative Henyey track; that dispatch is the planned follow-on this refusal
    /// names rather than extrapolating the wall past its physics.
    AboveDomain {
        /// The grid's greatest tabulated mass `M / Msun`.
        max_mass_ratio: Fixed,
    },
    /// A fixed-point intermediate in the interpolation left the representable range.
    Unrepresentable,
    /// The grid holds no rows (a load failure the caller did not check).
    Empty,
}

/// Why loading the grid failed. A load fault is distinct from a read refusal: a malformed or uncited column is a
/// build fault, not a domain edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HayashiWallError {
    /// The TOML did not parse.
    Parse(String),
    /// A value string was not a legal decimal.
    BadValue { field: String, detail: String },
    /// The column carries no source citation (a cited-data column must name its primary).
    MissingProvenance,
    /// The masses are not strictly increasing, so interpolation has no well-defined bracket.
    NonMonotonic { at_mass: Fixed },
    /// The grid holds no `[[wall]]` rows.
    Empty,
}

impl std::fmt::Display for HayashiWallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HayashiWallError::Parse(m) => write!(f, "hayashi-wall parse error: {m}"),
            HayashiWallError::BadValue { field, detail } => {
                write!(
                    f,
                    "hayashi-wall value '{field}' could not be read: {detail}"
                )
            }
            HayashiWallError::MissingProvenance => {
                write!(f, "hayashi-wall column carries no source citation")
            }
            HayashiWallError::NonMonotonic { at_mass } => {
                write!(
                    f,
                    "hayashi-wall masses are not strictly increasing at {at_mass:?}"
                )
            }
            HayashiWallError::Empty => write!(f, "hayashi-wall grid holds no rows"),
        }
    }
}

impl std::error::Error for HayashiWallError {}

/// The mass-indexed Hayashi-wall grid: the cited rows plus the grade and conditioning the modality declares.
#[derive(Clone, Debug)]
pub struct HayashiWallGrid {
    rows: Vec<HayashiWallRow>,
    grade: String,
    composition: String,
    epoch: String,
}

fn parse_field(name: &str, s: &str) -> Result<Fixed, HayashiWallError> {
    Fixed::from_decimal_str(s.trim()).map_err(|detail| HayashiWallError::BadValue {
        field: name.to_string(),
        detail,
    })
}

impl HayashiWallGrid {
    /// Parse the grid from TOML text, requiring a source citation and strictly increasing masses (the
    /// interpolation invariant). Rows are sorted by mass so the read's bracket is well defined.
    pub fn from_toml_str(s: &str) -> Result<Self, HayashiWallError> {
        let file: WallFile =
            toml::from_str(s).map_err(|e| HayashiWallError::Parse(e.to_string()))?;
        if file.source.trim().is_empty() {
            return Err(HayashiWallError::MissingProvenance);
        }
        if file.wall.is_empty() {
            return Err(HayashiWallError::Empty);
        }
        let mut rows: Vec<HayashiWallRow> = Vec::with_capacity(file.wall.len());
        for r in &file.wall {
            rows.push(HayashiWallRow {
                mass_ratio: parse_field("mass_msun", &r.mass_msun)?,
                wall_teff_k: parse_field("wall_teff_k", &r.wall_teff_k)?,
                drift_lo_k: parse_field("drift_lo_k", &r.drift_lo_k)?,
                drift_hi_k: parse_field("drift_hi_k", &r.drift_hi_k)?,
            });
        }
        rows.sort_by(|a, b| a.mass_ratio.cmp(&b.mass_ratio));
        for pair in rows.windows(2) {
            if pair[1].mass_ratio <= pair[0].mass_ratio {
                return Err(HayashiWallError::NonMonotonic {
                    at_mass: pair[1].mass_ratio,
                });
            }
        }
        Ok(HayashiWallGrid {
            rows,
            grade: file.grade.trim().to_string(),
            composition: file.composition.trim().to_string(),
            epoch: file.epoch.trim().to_string(),
        })
    }

    /// Load from a path (the runtime read).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, HayashiWallError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| HayashiWallError::Parse(format!("read error: {e}")))?;
        Self::from_toml_str(&text)
    }

    /// The standard vendored grid, embedded at compile time.
    pub fn standard() -> Result<Self, HayashiWallError> {
        Self::from_toml_str(include_str!("../data/hayashi_wall.toml"))
    }

    /// The modality grade (`theory`), the composition conditioning, and the epoch convention, the chord fields the
    /// column declares.
    pub fn grade(&self) -> &str {
        &self.grade
    }
    /// The composition conditioning (`solar [M/H]=0`), the Siess metallicity follow-on's key.
    pub fn composition(&self) -> &str {
        &self.composition
    }
    /// The epoch convention the wall and drift band are chords over.
    pub fn epoch(&self) -> &str {
        &self.epoch
    }
    /// The cited rows, ascending in mass.
    pub fn rows(&self) -> &[HayashiWallRow] {
        &self.rows
    }

    /// The wall `T_eff` and drift band at a stellar mass `M / Msun`, INTERPOLATED in the table's own spacing.
    /// Refuses BY NAME below the least and above the greatest tabulated mass (the two-ended domain guard), the
    /// high side naming the radiative-branch dispatch. An exact grid mass returns that row's values exactly.
    pub fn wall_teff(&self, mass_ratio: Fixed) -> Result<WallReading, WallRefusal> {
        let first = self.rows.first().ok_or(WallRefusal::Empty)?;
        let last = self.rows.last().ok_or(WallRefusal::Empty)?;
        if mass_ratio < first.mass_ratio {
            return Err(WallRefusal::BelowDomain {
                min_mass_ratio: first.mass_ratio,
            });
        }
        if mass_ratio > last.mass_ratio {
            return Err(WallRefusal::AboveDomain {
                max_mass_ratio: last.mass_ratio,
            });
        }
        for pair in self.rows.windows(2) {
            let lo = pair[0];
            let hi = pair[1];
            if mass_ratio >= lo.mass_ratio && mass_ratio <= hi.mass_ratio {
                return interpolate(lo, hi, mass_ratio);
            }
        }
        Err(WallRefusal::Unrepresentable)
    }
}

/// Linear interpolation of the wall and drift band between two bracketing rows. At a grid endpoint (`frac` 0 or 1)
/// it returns that row's values exactly.
fn interpolate(
    lo: HayashiWallRow,
    hi: HayashiWallRow,
    mass_ratio: Fixed,
) -> Result<WallReading, WallRefusal> {
    let span = hi
        .mass_ratio
        .checked_sub(lo.mass_ratio)
        .ok_or(WallRefusal::Unrepresentable)?;
    if span <= ZERO {
        return Ok(WallReading {
            mass_ratio,
            wall_teff_k: lo.wall_teff_k,
            drift_lo_k: lo.drift_lo_k,
            drift_hi_k: lo.drift_hi_k,
        });
    }
    let frac = mass_ratio
        .checked_sub(lo.mass_ratio)
        .and_then(|x| x.checked_div(span))
        .ok_or(WallRefusal::Unrepresentable)?;
    let lerp = |a: Fixed, b: Fixed| -> Option<Fixed> {
        let delta = b.checked_sub(a)?;
        a.checked_add(frac.checked_mul(delta)?)
    };
    Ok(WallReading {
        mass_ratio,
        wall_teff_k: lerp(lo.wall_teff_k, hi.wall_teff_k).ok_or(WallRefusal::Unrepresentable)?,
        drift_lo_k: lerp(lo.drift_lo_k, hi.drift_lo_k).ok_or(WallRefusal::Unrepresentable)?,
        drift_hi_k: lerp(lo.drift_hi_k, hi.drift_hi_k).ok_or(WallRefusal::Unrepresentable)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> HayashiWallGrid {
        HayashiWallGrid::standard().expect("the vendored grid loads")
    }

    #[test]
    fn the_grid_loads_cited_and_monotonic_at_theory_grade() {
        let g = grid();
        assert_eq!(g.rows().len(), 30, "the BHAC15 wall grid has 30 mass rows");
        assert_eq!(g.grade(), "theory");
        assert!(g.composition().contains("solar"));
        for pair in g.rows().windows(2) {
            assert!(
                pair[1].mass_ratio > pair[0].mass_ratio,
                "masses strictly increase"
            );
        }
    }

    #[test]
    fn the_solar_wall_is_the_corrected_digit() {
        // FINGERPRINT: the solar (1.000 Msun) wall is BHAC15's 4397 K, the digit that retires the wrong 4000 K.
        let r = grid()
            .wall_teff(Fixed::ONE)
            .expect("solar is inside the domain");
        assert_eq!(
            r.wall_teff_k,
            Fixed::from_int(4397),
            "the solar wall is 4397 K, not the retired 4000"
        );
        assert_eq!(r.drift_lo_k, Fixed::from_int(4350));
        assert_eq!(r.drift_hi_k, Fixed::from_int(4397));
    }

    #[test]
    fn a_between_grid_mass_interpolates_never_snaps() {
        // 0.55 Msun sits between the 0.5 (3849) and 0.6 (3988) rows; the wall is their midpoint, ~3918.5, NOT
        // either grid value. A snap would quantize every star to the BHAC15 mass sampling.
        let g = grid();
        let r = g
            .wall_teff(Fixed::from_ratio(55, 100))
            .expect("0.55 is inside the domain");
        let lo = Fixed::from_int(3849);
        let hi = Fixed::from_int(3988);
        assert!(
            r.wall_teff_k > lo && r.wall_teff_k < hi,
            "0.55 interpolates strictly between the 0.5 and 0.6 walls, got {:?}",
            r.wall_teff_k
        );
        // The midpoint (0.55 is halfway between 0.5 and 0.6), within the fixed-point rounding of the 0.6 and 0.55
        // decimals (neither is bit-exact in Q32.32, so frac is ~0.5, not exactly 0.5).
        let mid = (lo + hi).checked_div(Fixed::from_int(2)).unwrap();
        let diff = if r.wall_teff_k >= mid {
            r.wall_teff_k - mid
        } else {
            mid - r.wall_teff_k
        };
        assert!(
            diff <= Fixed::from_ratio(1, 10),
            "0.55 is ~the 0.5-0.6 midpoint {mid:?} within rounding, got {:?}",
            r.wall_teff_k
        );
    }

    #[test]
    fn an_exact_grid_mass_returns_that_row_exactly() {
        // 0.070 is a grid mass (2834 K); interpolation at an endpoint must return it to the bit.
        let r = grid()
            .wall_teff(Fixed::from_ratio(70, 1000))
            .expect("0.070 is a grid mass");
        assert_eq!(r.wall_teff_k, Fixed::from_int(2834));
    }

    #[test]
    fn the_domain_guards_refuse_by_name_two_ended() {
        let g = grid();
        // Below the hydrogen-burning-limit end.
        match g.wall_teff(Fixed::from_ratio(5, 1000)) {
            Err(WallRefusal::BelowDomain { min_mass_ratio }) => {
                assert_eq!(min_mass_ratio, Fixed::from_ratio(10, 1000));
            }
            other => panic!("0.005 must refuse below-domain, got {other:?}"),
        }
        // Above the greatest tabulated mass: the radiative-branch dispatch, named not extrapolated.
        match g.wall_teff(Fixed::from_ratio(3, 2)) {
            Err(WallRefusal::AboveDomain { max_mass_ratio }) => {
                assert_eq!(max_mass_ratio, Fixed::from_ratio(14, 10));
            }
            other => panic!("1.5 must refuse above-domain, got {other:?}"),
        }
    }

    #[test]
    fn an_uncited_grid_refuses_to_load() {
        // A column with no source citation is a build fault, not a silent load.
        let no_src = "grade = \"theory\"\n\n[[wall]]\nmass_msun = \"1.0\"\nwall_teff_k = \"4397\"\ndrift_lo_k = \"4350\"\ndrift_hi_k = \"4397\"\n";
        assert_eq!(
            HayashiWallGrid::from_toml_str(no_src).unwrap_err(),
            HayashiWallError::MissingProvenance
        );
    }
}
