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

//! The Murphy & Koop (2005) ice-Ih saturation vapor pressure column
//! (`crates/physics/data/ice_sublimation_murphy_koop.toml`), the `H2O(gas) <-> H2O(ice)` snow-line front: per
//! temperature the ice saturation vapor pressure (Pa). NIST-JANAF (see `janaf.rs`) tabulates only liquid water
//! `H2O(l)`, so this fills the solid-ice gap the condensation sequence needs below the freezing point. The snow line
//! is where this saturation pressure meets the solar water partial pressure, so this column is the ground truth a
//! Gibbs-minimization water condensate must reproduce, a calibration target rather than a run input.
//!
//! Cited [M]. The load-bearing datum is the Murphy-Koop equation itself (natural log, T in K, p in Pa, valid above
//! 110 K): `ln p_ice = 9.550426 - 5723.265/T + 3.53068*ln(T) - 0.00728332*T` (Murphy & Koop 2005, Q. J. R. Meteorol.
//! Soc. 131, 1539, their ice eq. 7). The second witness is the IAPWS ice-Ih Gibbs function (Feistel & Wagner 2006,
//! J. Phys. Chem. Ref. Data 35, 1021, release R10-06), from which the sublimation pressure derives and which agrees
//! with Murphy-Koop to well under a percent over this range. The tabulated points are the equation's evaluation at
//! 10 K steps (130-220 K), reproducible byte-free from the four header coefficients (the provenance battery
//! recomputes them). The parse routes every value through the exact `BigRat` conduit to `Fixed` (no floating point
//! reaches canonical state). BLOCK KIND `[[point]]`, the cited-data-column idiom, NOT a reserved floor kind.
//!
//! HONEST LIMITS: the points are a 3-significant-figure rounding of the equation; the smallest (below ~1e-6 Pa)
//! carry reduced Q32.32 precision (the ~1.2e-8 Pa point at 130 K stores to ~2 sig figs, near the fixed-point
//! epsilon), so the header equation is the full-resolution source. The curve is ice Ih only, the stable phase in
//! this range. The source-ladder rule (owner ruling: alternative [M] precedes estimator) makes Murphy-Koop /
//! Feistel-Wagner the [M], with Clausius-Clapeyron on the sublimation branch (`L_sub = L_vap + L_fus`) the in-house
//! certifier that shadow-validates the fetch, never the source of the numbers. No consumer is wired in any pinned
//! run path (byte-neutral).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use serde::Deserialize;
use std::fmt;

/// What can go wrong loading the ice sublimation column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IceSublimationError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A temperature or pressure string could not be parsed to fixed-point.
    BadValue(String),
    /// A temperature appears twice.
    Duplicate(String),
    /// The saturation pressure is non-positive (a vapor pressure must be a positive quantity).
    NotPhysical(String),
}

impl fmt::Display for IceSublimationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IceSublimationError::Parse(m) => write!(f, "ice sublimation parse error: {m}"),
            IceSublimationError::BadValue(m) => write!(f, "ice sublimation value error: {m}"),
            IceSublimationError::Duplicate(m) => write!(f, "duplicate ice sublimation point: {m}"),
            IceSublimationError::NotPhysical(m) => {
                write!(f, "non-physical ice saturation pressure: {m}")
            }
        }
    }
}

impl std::error::Error for IceSublimationError {}

/// One tabulated `(temperature, ice saturation vapor pressure)` point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcePoint {
    /// Temperature (K).
    pub t_k: Fixed,
    /// Ice saturation vapor pressure (Pa).
    pub p_sat_pa: Fixed,
}

/// The loaded ice sublimation column: the points in temperature order, plus the phase and the four Murphy-Koop
/// equation coefficients (the recipe, so the reproduction battery recomputes the points from them).
#[derive(Debug, Clone, Default)]
pub struct IceSublimation {
    points: Vec<IcePoint>,
    phase: String,
    eq_a: String,
    eq_b: String,
    eq_c: String,
    eq_d: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    phase: String,
    #[serde(default)]
    eq_a: String,
    #[serde(default)]
    eq_b: String,
    #[serde(default)]
    eq_c: String,
    #[serde(default)]
    eq_d: String,
    // `[[point]]`, the cited-data-column block kind, NOT a reserved floor kind.
    #[serde(default)]
    point: Vec<RawPoint>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPoint {
    t_k: String,
    p_sat_pa: String,
}

/// Parse one decimal string (plain or scientific, e.g. `1.20e-8`) to `Fixed` through the exact `BigRat` path.
fn fixed_from_decimal(s: &str) -> Result<Fixed, IceSublimationError> {
    let br = BigRat::from_decimal_str(s).map_err(IceSublimationError::BadValue)?;
    let bits = br
        .round_to_scale(Fixed::FRAC_BITS)
        .ok_or_else(|| IceSublimationError::BadValue(format!("{s} out of range")))?;
    Fixed::from_bits_i128(bits)
        .ok_or_else(|| IceSublimationError::BadValue(format!("{s} out of range")))
}

impl IceSublimation {
    /// Parse and validate the column from a TOML string. Every point keys off a unique temperature; the saturation
    /// pressure must be strictly positive; a malformed value fails closed.
    pub fn from_toml_str(s: &str) -> Result<Self, IceSublimationError> {
        let file: RawFile =
            toml::from_str(s).map_err(|e| IceSublimationError::Parse(e.to_string()))?;
        let mut points: Vec<IcePoint> = Vec::with_capacity(file.point.len());
        for raw in file.point {
            let t_k = fixed_from_decimal(&raw.t_k)?;
            let p_sat_pa = fixed_from_decimal(&raw.p_sat_pa)?;
            if p_sat_pa <= Fixed::ZERO {
                return Err(IceSublimationError::NotPhysical(raw.t_k));
            }
            if points.iter().any(|p| p.t_k == t_k) {
                return Err(IceSublimationError::Duplicate(raw.t_k));
            }
            points.push(IcePoint { t_k, p_sat_pa });
        }
        Ok(IceSublimation {
            points,
            phase: file.phase,
            eq_a: file.eq_a,
            eq_b: file.eq_b,
            eq_c: file.eq_c,
            eq_d: file.eq_d,
        })
    }

    /// Load the standard Murphy-Koop ice column from the checked-in data file.
    pub fn standard() -> Result<Self, IceSublimationError> {
        Self::from_toml_str(include_str!("../data/ice_sublimation_murphy_koop.toml"))
    }

    /// The ice saturation vapor pressure (Pa) at an exactly tabulated temperature, or `None` if that temperature is
    /// not a tabulated point.
    pub fn p_sat_at(&self, t_k: Fixed) -> Option<Fixed> {
        self.points
            .iter()
            .find(|p| p.t_k == t_k)
            .map(|p| p.p_sat_pa)
    }

    /// The tabulated points, in temperature order.
    pub fn points(&self) -> &[IcePoint] {
        &self.points
    }

    /// The ice phase the column is computed for (e.g. `ice-Ih`), for provenance.
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// The four Murphy-Koop equation coefficients `(a, b, c, d)` as decimal strings (the recipe), so a consumer can
    /// evaluate `ln p = a + b/T + c*ln(T) + d*T` at the full resolution the tabulated points round away.
    pub fn equation_coefficients(&self) -> (&str, &str, &str, &str) {
        (&self.eq_a, &self.eq_b, &self.eq_c, &self.eq_d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> IceSublimation {
        IceSublimation::standard().expect("the Murphy-Koop ice column loads")
    }

    #[test]
    fn the_table_loads_all_10_points_for_ice_ih() {
        let t = table();
        assert_eq!(t.points().len(), 10, "130-220 K in 10 K steps");
        assert_eq!(t.phase(), "ice-Ih", "the ice-Ih phase");
        let (a, b, c, d) = t.equation_coefficients();
        assert_eq!(
            (a, b, c, d),
            ("9.550426", "-5723.265", "3.53068", "-0.00728332"),
            "the Murphy-Koop coefficients are recorded"
        );
    }

    #[test]
    fn the_snow_line_fingerprint_reproduces_murphy_koop() {
        // The owner's pre-registered fingerprint: p_sat at 180 K ~ 5.40e-3 Pa = 5.4e-8 bar, of order the solar water
        // partial pressure at 1e-4 bar (~1e-8 bar), the water snow line (Lodders water-ice = 182 K).
        let t = table();
        let p180 = t
            .p_sat_at(Fixed::from_int(180))
            .expect("180 K is a tabulated point");
        assert!(
            (p180.to_f64_lossy() - 5.40e-3).abs() < 1e-4,
            "p_sat(180 K) is 5.40e-3 Pa, got {}",
            p180.to_f64_lossy()
        );
    }

    #[test]
    fn the_pressure_rises_monotonically_with_temperature() {
        // A coarse transcription check: the ice saturation pressure increases with temperature over the whole range
        // (a scrambled column would break this), spanning ~1.2e-8 Pa at 130 K to a few Pa at 220 K.
        let t = table();
        let pts = t.points();
        for w in pts.windows(2) {
            assert!(
                w[1].t_k > w[0].t_k && w[1].p_sat_pa > w[0].p_sat_pa,
                "p_sat rises with T"
            );
        }
        assert!(
            pts.first().unwrap().p_sat_pa.to_f64_lossy() > 0.0,
            "the smallest point is a positive pressure"
        );
        assert!(
            pts.last().unwrap().p_sat_pa.to_f64_lossy() > 1.0,
            "the 220 K point is a few Pa"
        );
    }

    #[test]
    fn an_untabulated_temperature_returns_none() {
        let t = table();
        assert_eq!(t.p_sat_at(Fixed::from_int(175)), None);
    }

    #[test]
    fn a_non_positive_pressure_fails_closed() {
        let bad = r#"
[[point]]
t_k = "150"
p_sat_pa = "0"
"#;
        assert!(matches!(
            IceSublimation::from_toml_str(bad),
            Err(IceSublimationError::NotPhysical(_))
        ));
    }

    #[test]
    fn a_bad_value_fails_closed() {
        let bad = r#"
[[point]]
t_k = "150"
p_sat_pa = "not-a-number"
"#;
        assert!(matches!(
            IceSublimation::from_toml_str(bad),
            Err(IceSublimationError::BadValue(_))
        ));
    }

    #[test]
    fn a_duplicate_temperature_fails_closed() {
        let bad = r#"
[[point]]
t_k = "150"
p_sat_pa = "6.11e-6"

[[point]]
t_k = "150"
p_sat_pa = "6.11e-6"
"#;
        assert!(matches!(
            IceSublimation::from_toml_str(bad),
            Err(IceSublimationError::Duplicate(_))
        ));
    }
}
