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

//! The per-phase thermodynamic Gruneisen parameter, read from the cited table in
//! `data/gruneisen.toml`, and the AGGREGATOR that derives a rock's gamma from a world's own mineral
//! census. This is the thermal-EoS sibling of [`crate::mineral_moduli`], and it deliberately mirrors
//! that module's shape: the same two-rung ladder per quantity, the same canonical phase key, the same
//! refusal rather than a default.
//!
//! WHY THERE IS NO "MANTLE GAMMA". A mineral's gamma is a FLOOR datum: it was measured in ignorance of
//! any rock the mineral would later sit in, so it cannot be fitted to an outcome. A ROCK's gamma is an
//! aggregate over its phases, so it is DERIVED from the assemblage and is never a reserved value. The
//! data file states this rule and this module enforces it: [`GruneisenTable::assemblage_gamma`] weights
//! the cited per-phase gammas by the census fractions, and a phase with no cited row is REFUSED rather
//! than defaulted. A default there would author a mantle's thermal pressure, which is exactly the
//! value-authoring line (Principle 11).
//!
//! THE TWO-RUNG LADDER, per phase and per quantity, the same discipline the moduli file carries. The
//! measured anchor is `gamma_thermodynamic`, the thermodynamic definition `gamma_th = alpha K_S V / C_P`
//! at the row's own conditions. Below it sits the derived estimator `gamma_eos_debye`, the equation-of-
//! state compilation value. [`GruneisenTable::gamma`] returns the measured anchor when the row carries
//! one and falls back to the estimator otherwise, reporting WHICH rung it used through
//! [`GruneisenRung`], so a caller can propagate the grade rather than silently mixing measured and
//! estimated values in one aggregate.
//!
//! HONEST LIMITS. The rows are ambient-frame (near 300 K, 1 bar), so an aggregate carries that chord and
//! nothing else: this module does not extrapolate gamma in pressure or temperature, and a caller that
//! needs a deep-mantle gamma is reading a value outside the frame its rows were measured in. The
//! aggregate is a volume or mass weighted mean, which is the standard first-order mixing rule and not a
//! rigorous bound; a rock whose phases differ strongly in stiffness is not well described by it. Both
//! limits are the caller's to carry, and [`AssemblageGamma`] reports the frame and the rung mix so the
//! caller can.

use std::collections::BTreeMap;
use std::fmt;

use civsim_core::Fixed;

use crate::mineral_moduli::canonical_phase_key;

const ZERO: Fixed = Fixed::ZERO;

/// Why a Gruneisen table failed to load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GruneisenError {
    /// A row could not be read.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every measured value is real-with-source).
    MissingSource(String),
    /// A gamma is non-positive, or a band is negative. A non-positive Gruneisen parameter is not
    /// physical for a solid: it would invert the sign of thermal pressure.
    NonPhysical(String),
    /// A phase name appears twice.
    Duplicate(String),
    /// The table parsed to no rows at all.
    Empty,
}

impl fmt::Display for GruneisenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GruneisenError::Parse(m) => write!(f, "gruneisen parse error: {m}"),
            GruneisenError::BadValue(m) => write!(f, "gruneisen value error: {m}"),
            GruneisenError::MissingSource(m) => write!(f, "gruneisen row without citation: {m}"),
            GruneisenError::NonPhysical(m) => write!(f, "gruneisen non-physical value: {m}"),
            GruneisenError::Duplicate(m) => write!(f, "duplicate gruneisen key: {m}"),
            GruneisenError::Empty => write!(f, "gruneisen table parsed to zero rows"),
        }
    }
}

impl std::error::Error for GruneisenError {}

/// Which rung of the ladder a returned gamma came from, so a caller can propagate the grade instead of
/// mixing a measured value and an estimate without saying so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GruneisenRung {
    /// The measured thermodynamic anchor, `gamma_th = alpha K_S V / C_P`.
    MeasuredThermodynamic,
    /// The derived equation-of-state compilation estimate.
    EosDebyeEstimate,
}

/// One phase's cited Gruneisen row, in the ambient reference frame the table declares.
#[derive(Clone, Debug)]
pub struct GruneisenRow {
    /// The phase name, as the registry spells it.
    pub name: String,
    /// The measured thermodynamic gamma, when the row carries one.
    pub gamma_thermodynamic: Option<Fixed>,
    /// Its symmetric half-width band.
    pub gamma_thermodynamic_band: Option<Fixed>,
    /// The derived equation-of-state gamma, the estimator rung.
    pub gamma_eos_debye: Option<Fixed>,
    /// Its symmetric half-width band.
    pub gamma_eos_debye_band: Option<Fixed>,
    /// The temperature the row's values were measured at (kelvin).
    pub temperature_k: Fixed,
    /// The pressure the row's values were measured at (bar).
    pub pressure_bar: Fixed,
}

impl GruneisenRow {
    /// The row's gamma by the LADDER: the measured anchor when present, the estimator otherwise, with
    /// the rung it came from. `None` when the row carries NEITHER rung, which is a real loaded state
    /// rather than an impossible one: quartz is the cited instance, a K'-only row whose direct gamma_th
    /// is strongly temperature dependent and is not held at a primary, so the table declines to hold one.
    /// A caller that needs a gamma must handle the `None`, and the aggregator refuses such a phase.
    pub fn gamma(&self) -> Option<(Fixed, GruneisenRung)> {
        if let Some(g) = self.gamma_thermodynamic {
            return Some((g, GruneisenRung::MeasuredThermodynamic));
        }
        self.gamma_eos_debye
            .map(|g| (g, GruneisenRung::EosDebyeEstimate))
    }

    /// The band on whichever rung [`Self::gamma`] returns, so an uncertainty travels with its value.
    pub fn gamma_band(&self) -> Option<Fixed> {
        if self.gamma_thermodynamic.is_some() {
            return self.gamma_thermodynamic_band;
        }
        self.gamma_eos_debye_band
    }
}

/// A rock's derived Gruneisen parameter, with the evidence a caller needs to know what it is holding.
#[derive(Clone, Debug)]
pub struct AssemblageGamma {
    /// The census-weighted gamma.
    pub gamma: Fixed,
    /// The weighted band, propagated from the per-phase bands on the rung each row supplied.
    pub band: Fixed,
    /// How much of the weight came from MEASURED rows rather than estimates, as a fraction. A caller
    /// that needs a measured-grade value reads this rather than assuming.
    pub measured_weight_fraction: Fixed,
    /// The frame the rows were measured in, carried so a caller cannot silently use an ambient-frame
    /// aggregate as a deep-interior value.
    pub frame_temperature_k: Fixed,
    /// The frame pressure (bar).
    pub frame_pressure_bar: Fixed,
}

/// A census phase the table cannot supply, which is REFUSED rather than defaulted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GammaRefusal {
    /// The phase the census named.
    pub phase: String,
}

impl fmt::Display for GammaRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "census phase {} has no cited Gruneisen row: add its cited gamma to \
             crates/physics/data/gruneisen.toml. A rock gamma is an aggregate over cited phases, so an \
             uncited phase is refused rather than defaulted (Principle 11).",
            self.phase
        )
    }
}

impl std::error::Error for GammaRefusal {}

/// The cited per-phase Gruneisen table.
#[derive(Clone, Debug)]
pub struct GruneisenTable {
    rows: BTreeMap<String, GruneisenRow>,
}

impl GruneisenTable {
    /// Load the vendored table.
    pub fn standard() -> Result<Self, GruneisenError> {
        Self::from_toml_str(include_str!("../data/gruneisen.toml"))
    }

    /// Parse the `[[mineral]]` blocks. Values are QUOTED strings and are parsed to fixed-point through
    /// [`Fixed::from_decimal_str`], so no float ever enters, the same discipline the moduli and
    /// convection-scaling loaders use.
    pub fn from_toml_str(s: &str) -> Result<Self, GruneisenError> {
        let mut rows: BTreeMap<String, GruneisenRow> = BTreeMap::new();
        // Split on LINE-ANCHORED block headers, never on the bare string. The file's own header prose
        // names `[[mineral]]` while explaining the block-kind idiom, so a bare split manufactures
        // phantom blocks out of comment text. A block header is a line that is exactly the marker.
        let mut blocks: Vec<String> = Vec::new();
        let mut current: Option<Vec<&str>> = None;
        for line in s.lines() {
            if line.trim() == "[[mineral]]" {
                if let Some(prev) = current.take() {
                    blocks.push(prev.join("\n"));
                }
                current = Some(Vec::new());
            } else if let Some(buf) = current.as_mut() {
                buf.push(line);
            }
        }
        if let Some(last) = current.take() {
            blocks.push(last.join("\n"));
        }
        for block in &blocks {
            let block = block.as_str();
            let field = |key: &str| -> Option<String> {
                block.lines().find_map(|line| {
                    let rest = line
                        .trim()
                        .strip_prefix(key)?
                        .trim_start()
                        .strip_prefix('=')?;
                    Some(rest.trim().trim_matches('"').to_string())
                })
            };
            let Some(name) = field("name") else {
                return Err(GruneisenError::Parse(
                    "a [[mineral]] block has no name".into(),
                ));
            };
            // Every cited row carries its source; a row without one is not a measured value.
            if field("citation").is_none() {
                return Err(GruneisenError::MissingSource(name));
            }
            let num = |key: &str| -> Result<Option<Fixed>, GruneisenError> {
                match field(key) {
                    None => Ok(None),
                    Some(raw) => Fixed::from_decimal_str(raw.trim())
                        .map(Some)
                        .map_err(|_| GruneisenError::BadValue(format!("{name}.{key} = {raw}"))),
                }
            };
            let gamma_thermodynamic = num("gamma_thermodynamic")?;
            let gamma_eos_debye = num("gamma_eos_debye")?;
            // A row may legitimately carry NO gamma at all. Quartz is the cited instance: a framework
            // silicate whose direct gamma_th is strongly temperature dependent and is "not held at a
            // primary", so the table holds its K' and declines to hold a gamma rather than fitting one.
            // Such a row LOADS, because its K' and its citation are real, and the AGGREGATOR refuses it
            // if a census names it. Rejecting the whole table here would punish the data for being
            // more disciplined than the reader.
            for (label, g) in [
                ("gamma_thermodynamic", gamma_thermodynamic),
                ("gamma_eos_debye", gamma_eos_debye),
            ] {
                if let Some(v) = g {
                    if v <= ZERO {
                        return Err(GruneisenError::NonPhysical(format!(
                            "{name}.{label} = {v:?}"
                        )));
                    }
                }
            }
            let gamma_thermodynamic_band = num("gamma_thermodynamic_band")?;
            let gamma_eos_debye_band = num("gamma_eos_debye_band")?;
            for (label, b) in [
                ("gamma_thermodynamic_band", gamma_thermodynamic_band),
                ("gamma_eos_debye_band", gamma_eos_debye_band),
            ] {
                if let Some(v) = b {
                    if v < ZERO {
                        return Err(GruneisenError::NonPhysical(format!(
                            "{name}.{label} negative"
                        )));
                    }
                }
            }
            let row = GruneisenRow {
                name: name.clone(),
                gamma_thermodynamic,
                gamma_thermodynamic_band,
                gamma_eos_debye,
                gamma_eos_debye_band,
                temperature_k: num("temperature_k")?.unwrap_or(ZERO),
                pressure_bar: num("pressure_bar")?.unwrap_or(ZERO),
            };
            let key = canonical_phase_key(&name).to_string();
            if rows.insert(key.clone(), row).is_some() {
                return Err(GruneisenError::Duplicate(key));
            }
        }
        if rows.is_empty() {
            return Err(GruneisenError::Empty);
        }
        Ok(Self { rows })
    }

    /// The row for a phase, resolving the two naming conventions through the SAME canonical key the
    /// moduli floor uses, so a crust phase named by the condensation solver and a mantle phase named by
    /// the petrology kernel reach the one cited row.
    pub fn row(&self, name: &str) -> Option<&GruneisenRow> {
        self.rows.get(canonical_phase_key(name))
    }

    /// A phase's gamma by the ladder, with its rung.
    pub fn gamma(&self, name: &str) -> Option<(Fixed, GruneisenRung)> {
        self.row(name).and_then(|r| r.gamma())
    }

    /// How many phases the table carries.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table is empty (it never is once loaded; the loader refuses an empty table).
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// DERIVE a rock's gamma from a world's own mineral census: the fraction-weighted mean of the cited
    /// per-phase gammas, with the band carried and the measured-versus-estimated weight reported.
    ///
    /// `census` is a list of (phase name, fraction) pairs, by volume or by mass as the caller chooses;
    /// the weighting is linear in whichever it supplies, so the caller owns that choice. Fractions need
    /// not sum to one: the result is normalized by the supplied total, so a partial census aggregates
    /// what it names.
    ///
    /// REFUSES rather than defaults. A census phase with no cited row returns [`GammaRefusal`] naming
    /// that phase. A mantle's gamma sets its thermal pressure, so inventing one for a missing phase
    /// would author a world's interior.
    // @derives: a rock's Gruneisen parameter <- the cited per-phase gamma table + the world's own mineral census
    pub fn assemblage_gamma(
        &self,
        census: &[(&str, Fixed)],
    ) -> Result<Option<AssemblageGamma>, GammaRefusal> {
        let mut total = ZERO;
        let mut gamma_acc = ZERO;
        let mut band_acc = ZERO;
        let mut measured_acc = ZERO;
        let mut frame_t = ZERO;
        let mut frame_p = ZERO;
        for (phase, fraction) in census {
            if *fraction <= ZERO {
                continue;
            }
            let Some(row) = self.row(phase) else {
                return Err(GammaRefusal {
                    phase: (*phase).to_string(),
                });
            };
            let Some((g, rung)) = row.gamma() else {
                return Err(GammaRefusal {
                    phase: (*phase).to_string(),
                });
            };
            let Some(weighted) = g.checked_mul(*fraction) else {
                continue;
            };
            gamma_acc = gamma_acc.checked_add(weighted).unwrap_or(gamma_acc);
            if let Some(b) = row.gamma_band() {
                if let Some(wb) = b.checked_mul(*fraction) {
                    band_acc = band_acc.checked_add(wb).unwrap_or(band_acc);
                }
            }
            if rung == GruneisenRung::MeasuredThermodynamic {
                measured_acc = measured_acc.checked_add(*fraction).unwrap_or(measured_acc);
            }
            // The frame is the rows' own; they share one ambient frame, so the last row's carries it.
            frame_t = row.temperature_k;
            frame_p = row.pressure_bar;
            total = total.checked_add(*fraction).unwrap_or(total);
        }
        if total <= ZERO {
            return Ok(None);
        }
        let gamma = gamma_acc.checked_div(total).unwrap_or(ZERO);
        let band = band_acc.checked_div(total).unwrap_or(ZERO);
        let measured_weight_fraction = measured_acc.checked_div(total).unwrap_or(ZERO);
        Ok(Some(AssemblageGamma {
            gamma,
            band,
            measured_weight_fraction,
            frame_temperature_k: frame_t,
            frame_pressure_bar: frame_p,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vendored_table_loads_and_carries_its_cited_rows() {
        let t = GruneisenTable::standard().expect("the vendored gruneisen table loads");
        assert!(
            t.len() >= 7,
            "the table carries its cited phases, got {}",
            t.len()
        );
        assert!(!t.is_empty());
    }

    #[test]
    fn periclase_reads_its_cited_measured_anchor() {
        // Anderson and Isaak 1995: gamma_thermodynamic 1.54 at 300 K, 1 bar.
        let t = GruneisenTable::standard().expect("loads");
        let (g, rung) = t.gamma("periclase").expect("periclase is a cited row");
        assert_eq!(rung, GruneisenRung::MeasuredThermodynamic);
        assert!(
            (g - Fixed::from_ratio(154, 100)).abs() < Fixed::from_ratio(1, 100),
            "periclase gamma is the cited 1.54, got {}",
            g.to_f64_lossy()
        );
    }

    #[test]
    fn the_ladder_prefers_the_measured_anchor_over_the_estimator() {
        let row = GruneisenRow {
            name: "test".into(),
            gamma_thermodynamic: Some(Fixed::from_ratio(150, 100)),
            gamma_thermodynamic_band: Some(Fixed::from_ratio(5, 100)),
            gamma_eos_debye: Some(Fixed::from_ratio(120, 100)),
            gamma_eos_debye_band: Some(Fixed::from_ratio(2, 100)),
            temperature_k: Fixed::from_int(300),
            pressure_bar: Fixed::ONE,
        };
        let (g, rung) = row.gamma().expect("a rung resolves");
        assert_eq!(rung, GruneisenRung::MeasuredThermodynamic);
        assert_eq!(g, Fixed::from_ratio(150, 100));
        assert_eq!(row.gamma_band(), Some(Fixed::from_ratio(5, 100)));

        // With no anchor the estimator carries it, and says so.
        let est = GruneisenRow {
            gamma_thermodynamic: None,
            gamma_thermodynamic_band: None,
            ..row
        };
        let (g2, rung2) = est.gamma().expect("the estimator rung resolves");
        assert_eq!(rung2, GruneisenRung::EosDebyeEstimate);
        assert_eq!(g2, Fixed::from_ratio(120, 100));
        assert_eq!(est.gamma_band(), Some(Fixed::from_ratio(2, 100)));
    }

    /// The aggregator is the whole reason this module exists: a rock's gamma is DERIVED from its census,
    /// so a single-phase census must reproduce that phase and a mixture must lie between its endpoints.
    #[test]
    fn the_assemblage_gamma_is_derived_from_the_census() {
        let t = GruneisenTable::standard().expect("loads");
        let single = t
            .assemblage_gamma(&[("periclase", Fixed::ONE)])
            .expect("cited")
            .expect("a positive census");
        let (solo, _) = t.gamma("periclase").expect("cited");
        assert_eq!(single.gamma, solo, "a one-phase census IS that phase");
        assert_eq!(single.measured_weight_fraction, Fixed::ONE);

        // A fraction pair that does not sum to one still normalizes by its own total.
        let doubled = t
            .assemblage_gamma(&[("periclase", Fixed::from_int(2))])
            .expect("cited")
            .expect("positive");
        assert_eq!(
            doubled.gamma, single.gamma,
            "the aggregate normalizes by the supplied total"
        );
    }

    /// The refusal is the load-bearing behaviour. A default here would author a mantle's thermal
    /// pressure, so an uncited phase must stop the aggregate and name itself.
    #[test]
    fn an_uncited_census_phase_is_refused_and_never_defaulted() {
        let t = GruneisenTable::standard().expect("loads");
        let refusal = t
            .assemblage_gamma(&[("periclase", Fixed::ONE), ("unobtainium", Fixed::ONE)])
            .expect_err("an uncited phase must be refused");
        assert_eq!(refusal.phase, "unobtainium");
        assert!(
            refusal
                .to_string()
                .contains("refused rather than defaulted"),
            "the refusal explains why it is not a default"
        );
    }

    /// The refusal is not hypothetical: quartz is a real, cited row that deliberately holds NO gamma,
    /// because a framework silicate's direct gamma_th is strongly temperature dependent and the table
    /// declines to fit one. So a census containing quartz must refuse rather than aggregate around it.
    #[test]
    fn the_cited_gamma_less_quartz_row_loads_but_is_refused_by_the_aggregator() {
        let t = GruneisenTable::standard().expect("loads");
        let row = t
            .row("quartz")
            .expect("quartz is a cited row, it carries K'");
        assert!(
            row.gamma().is_none(),
            "quartz deliberately holds no gamma, so no rung resolves"
        );
        let refusal = t
            .assemblage_gamma(&[("periclase", Fixed::ONE), ("quartz", Fixed::ONE)])
            .expect_err("a census naming a gamma-less phase must be refused");
        assert_eq!(refusal.phase, "quartz");
    }

    #[test]
    fn a_row_without_a_citation_is_rejected_at_load() {
        let s = "[[mineral]]\nname = \"ghost\"\ngamma_thermodynamic = \"1.2\"\n";
        assert!(matches!(
            GruneisenTable::from_toml_str(s),
            Err(GruneisenError::MissingSource(_))
        ));
    }

    #[test]
    fn a_non_physical_gamma_is_rejected_at_load() {
        let s = "[[mineral]]\nname = \"ghost\"\ngamma_thermodynamic = \"0\"\ncitation = \"x\"\n";
        assert!(matches!(
            GruneisenTable::from_toml_str(s),
            Err(GruneisenError::NonPhysical(_))
        ));
    }
}
