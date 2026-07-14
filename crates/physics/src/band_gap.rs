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

//! The band-gap column (`crates/physics/data/band_gap.toml`), the gap tier's measured `[M]` top rung and the
//! compute-once bottom rung, keyed per composition.
//!
//! Per substance the band gap in eV with its PROVENANCE on the tier's ladder: a MEASURED `[M]` value (top rung,
//! cited, refutable without the sim, the same status as `B_0` and `dH_f`) or a COMPUTE-ONCE eigenvalue (bottom
//! rung). The materials band-gap tier reads this to sort a substance metal / semiconductor / insulator by its gap;
//! no consumer is wired to it in any pinned run path yet (byte-neutral). The reduced-order Harrison estimator (the
//! middle rung) is held for the owner's fork and is not a data column.
//!
//! THE COMPUTE-ONCE EIGENVALUE-ROUTING LAW, made a build guard. A compute-once gap must come from a HYBRID
//! functional or a GW quasiparticle calculation, the classes whose eigenvalue gap tracks the physical gap. A plain
//! PBE or LDA (semilocal) gap underestimates by the derivative discontinuity, often by half or more, so it must
//! NEVER be wired as a gap. The loader ENFORCES this: a computed row tagged with a semilocal or unknown functional
//! is REJECTED at load (the way the d-state-radius loader rejects a 4s-scale radius), so no one wires a PBE gap in
//! good faith. The [`EigenvalueFunctional`] type carries only `Hybrid` and `Gw`, so a semilocal gap is
//! unrepresentable rather than merely discouraged.
//!
//! HONEST LIMIT (a flagged seam): a band gap is a property of a PHASE, not a bare composition, and this column is
//! composition-keyed like its `[M]`-column siblings. So it is correct for a single-phase substance but cannot hold
//! two polymorphs of one composition (diamond at `5.47 eV` and graphite the semimetal are both `{C:1}`). A
//! polymorph-bearing substance needs a phase-keyed column aligned with `phase_registry.toml` (the per-phase
//! registry), a named follow-on; this seed is scoped to substances with one ambient phase.

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the band-gap column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BandGapError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// A composition appears twice.
    Duplicate(String),
    /// The gap is negative (a gap is non-negative; a metal is a zero gap or an absent row, never a negative one).
    NegativeGap(String),
    /// The provenance tag is neither `measured` nor `computed`.
    BadProvenance(String),
    /// A COMPUTED gap tagged with a semilocal or unknown functional (PBE/LDA and the like): the compute-once
    /// eigenvalue-routing law, fail-closed. Only a hybrid functional or a GW calculation is admitted.
    ForbiddenFunctional(String),
    /// A COMPUTED gap with no functional tag (a compute-once value must name its functional class).
    MissingFunctional(String),
    /// A MEASURED gap carrying a functional tag (a measurement has no functional): a provenance confusion.
    StrayFunctional(String),
}

impl fmt::Display for BandGapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BandGapError::Parse(m) => write!(f, "band-gap parse error: {m}"),
            BandGapError::BadValue(m) => write!(f, "band-gap value error: {m}"),
            BandGapError::MissingSource(m) => write!(f, "band-gap row without citation: {m}"),
            BandGapError::Duplicate(m) => write!(f, "duplicate band-gap composition: {m}"),
            BandGapError::NegativeGap(m) => write!(f, "band-gap is negative (not a gap): {m}"),
            BandGapError::BadProvenance(m) => {
                write!(f, "band-gap provenance neither measured nor computed: {m}")
            }
            BandGapError::ForbiddenFunctional(m) => write!(
                f,
                "band-gap computed with a forbidden (non-hybrid/GW) functional, the compute-once law: {m}"
            ),
            BandGapError::MissingFunctional(m) => {
                write!(f, "band-gap computed with no functional tag: {m}")
            }
            BandGapError::StrayFunctional(m) => {
                write!(f, "band-gap measured but carrying a functional tag: {m}")
            }
        }
    }
}

impl std::error::Error for BandGapError {}

/// The functional class of a COMPUTE-ONCE band gap, the eigenvalue-routing law made a type: only a hybrid
/// functional or a GW quasiparticle calculation, the classes whose eigenvalue gap tracks the physical gap. There
/// is deliberately NO PBE or LDA variant, so a semilocal-functional gap (which underestimates by the derivative
/// discontinuity) is unrepresentable here, never merely discouraged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EigenvalueFunctional {
    /// A hybrid functional (a fraction of exact exchange): HSE, PBE0, B3LYP, and the like.
    Hybrid,
    /// A GW quasiparticle calculation (the many-body self-energy): the reference for the gap.
    Gw,
}

impl EigenvalueFunctional {
    /// Parse a functional tag to its class, or `None` (rejected) for a semilocal (PBE/LDA) or unknown functional:
    /// the compute-once eigenvalue-routing law, fail-closed. An unrecognized tag is rejected rather than admitted,
    /// so a new semilocal name cannot slip a bad gap past the guard.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag.trim().to_ascii_lowercase().as_str() {
            "hybrid" | "hse" | "hse06" | "pbe0" | "b3lyp" => Some(EigenvalueFunctional::Hybrid),
            "gw" | "g0w0" | "gw0" | "scgw" | "qsgw" => Some(EigenvalueFunctional::Gw),
            _ => None,
        }
    }
}

/// The provenance of a band gap on the tier's ladder: a measured `[M]` value (top rung) or a compute-once
/// eigenvalue (bottom rung). Both are authoritative (each encodes the correlation gap, a Mott insulator's
/// charge-transfer gap included), so both route through the gap-sign sort directly; only the reduced-order Harrison
/// estimator (the held middle rung, not a data column) runs the `U/W` preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapProvenance {
    /// A measured `[M]` band gap (source-cited, the top rung).
    Measured,
    /// A compute-once eigenvalue from a hybrid functional or a GW calculation (the bottom rung), its functional
    /// class carried so the compute-once law is auditable at the point of use.
    ComputeOnce {
        /// The functional class the compute-once gap came from (hybrid or GW, never semilocal).
        functional: EigenvalueFunctional,
    },
}

/// One substance's band gap and its provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BandGap {
    /// The band gap in eV (non-negative; a zero gap is the metal boundary).
    pub gap_ev: Fixed,
    /// The provenance on the tier's ladder (measured `[M]` or compute-once hybrid/GW).
    pub provenance: GapProvenance,
}

/// The band-gap column: per canonical composition, the cited gap and its provenance.
#[derive(Debug, Clone, Default)]
pub struct BandGapColumn {
    // Keyed by the canonical (BTreeMap-sorted) composition string, so a lookup is order-independent.
    by_composition: BTreeMap<String, BandGap>,
}

/// The canonical key of a composition: the elements in sorted order with their counts, so `{Ga:1, As:1}` and the
/// reverse both key the same row.
fn composition_key(composition: &BTreeMap<String, u32>) -> String {
    composition
        .iter()
        .map(|(el, n)| format!("{el}{n}"))
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct GapFile {
    #[serde(default)]
    gap: Vec<GapDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct GapDef {
    name: String,
    #[serde(default)]
    composition: BTreeMap<String, u32>,
    #[serde(default)]
    gap_ev: String,
    #[serde(default)]
    provenance: String,
    #[serde(default)]
    functional: String,
    #[serde(default)]
    source: String,
}

impl BandGapColumn {
    /// Load the column from a TOML string. Every row must carry a citation and a non-negative gap; the provenance
    /// must be `measured` (no functional) or `computed` (a hybrid/GW functional, the compute-once law enforced).
    pub fn from_toml_str(s: &str) -> Result<Self, BandGapError> {
        let file: GapFile = toml::from_str(s).map_err(|e| BandGapError::Parse(e.to_string()))?;
        let mut by_composition = BTreeMap::new();
        for g in file.gap {
            if g.source.trim().is_empty() {
                return Err(BandGapError::MissingSource(g.name.clone()));
            }
            let gap_ev = Fixed::from_decimal_str(g.gap_ev.trim())
                .map_err(|d| BandGapError::BadValue(format!("{}: {d}", g.name)))?;
            if gap_ev < Fixed::ZERO {
                return Err(BandGapError::NegativeGap(g.name.clone()));
            }
            let functional_tag = g.functional.trim();
            let provenance = match g.provenance.trim().to_ascii_lowercase().as_str() {
                "measured" => {
                    if !functional_tag.is_empty() {
                        return Err(BandGapError::StrayFunctional(g.name.clone()));
                    }
                    GapProvenance::Measured
                }
                "computed" => {
                    if functional_tag.is_empty() {
                        return Err(BandGapError::MissingFunctional(g.name.clone()));
                    }
                    // THE COMPUTE-ONCE LAW: a semilocal or unknown functional is rejected, fail-closed.
                    let functional =
                        EigenvalueFunctional::from_tag(functional_tag).ok_or_else(|| {
                            BandGapError::ForbiddenFunctional(format!(
                                "{}: {functional_tag}",
                                g.name
                            ))
                        })?;
                    GapProvenance::ComputeOnce { functional }
                }
                _ => return Err(BandGapError::BadProvenance(g.name.clone())),
            };
            let key = composition_key(&g.composition);
            if by_composition
                .insert(key, BandGap { gap_ev, provenance })
                .is_some()
            {
                return Err(BandGapError::Duplicate(g.name));
            }
        }
        Ok(BandGapColumn { by_composition })
    }

    /// The embedded standard column (`data/band_gap.toml`).
    pub fn standard() -> Result<Self, BandGapError> {
        Self::from_toml_str(include_str!("../data/band_gap.toml"))
    }

    /// The gap and provenance for a composition, or `None` when the substance is not in the seeded column (the gap
    /// tier then escalates or falls to the reduced-order route). Order-independent in the composition.
    pub fn gap(&self, composition: &[(String, u32)]) -> Option<&BandGap> {
        let map: BTreeMap<String, u32> = composition.iter().cloned().collect();
        self.by_composition.get(&composition_key(&map))
    }

    /// The number of seeded substances.
    pub fn len(&self) -> usize {
        self.by_composition.len()
    }

    /// Whether the column is empty.
    pub fn is_empty(&self) -> bool {
        self.by_composition.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column() -> BandGapColumn {
        BandGapColumn::standard().expect("the band-gap column loads")
    }

    fn comp(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect()
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_seed_carries_the_cited_measured_gaps_order_independently() {
        let c = column();
        // Silicon's cited 1.12 eV indirect gap; the lookup is order-independent in the composition.
        let si = c.gap(&comp(&[("Si", 1)])).expect("Si is seeded");
        assert!(
            close(si.gap_ev, 1.12, 0.001),
            "Si gap 1.12 eV, got {}",
            si.gap_ev.to_f64_lossy()
        );
        assert_eq!(si.provenance, GapProvenance::Measured);
        // GaAs, order-independent in the composition.
        let gaas = c.gap(&comp(&[("Ga", 1), ("As", 1)])).expect("GaAs");
        let gaas_rev = c
            .gap(&comp(&[("As", 1), ("Ga", 1)]))
            .expect("GaAs reversed");
        assert_eq!(gaas, gaas_rev, "the lookup is order-independent");
        assert!(close(gaas.gap_ev, 1.42, 0.001), "GaAs direct gap 1.42 eV");
    }

    #[test]
    fn the_seed_spans_semiconductor_to_wide_insulator() {
        // The seed spans the gap tier's regimes: Ge/Si/GaAs semiconductors, NiO a correlated (charge-transfer)
        // insulator, MgO a wide ionic insulator. All measured, all cited.
        let c = column();
        let ge = c.gap(&comp(&[("Ge", 1)])).expect("Ge").gap_ev;
        let mgo = c.gap(&comp(&[("Mg", 1), ("O", 1)])).expect("MgO").gap_ev;
        assert!(close(ge, 0.67, 0.001), "Ge 0.67 eV");
        assert!(close(mgo, 7.8, 0.01), "MgO ~7.8 eV");
        // The ordering is physical: the semiconductor gap is far below the wide insulator's.
        assert!(
            ge < mgo,
            "the semiconductor gap is below the wide-insulator gap"
        );
        assert_eq!(c.len(), 5, "the seed is Si/Ge/GaAs/MgO/NiO");
    }

    #[test]
    fn an_unseeded_substance_is_absent() {
        let c = column();
        assert!(
            c.gap(&comp(&[("Al", 1)])).is_none(),
            "aluminium (a metal, not a gapped substance) is absent, so the tier does not sort it here"
        );
    }

    #[test]
    fn the_compute_once_law_admits_a_gw_gap() {
        // A COMPUTED gap tagged GW loads and carries its functional class (a test-only fixture value).
        let ok = r#"
[[gap]]
name = "test GW substance"
composition = { Xx = 1 }
gap_ev = "3.0"
provenance = "computed"
functional = "GW"
source = "test-only fixture"
"#;
        let c = BandGapColumn::from_toml_str(ok).expect("a GW-computed gap loads");
        let g = c.gap(&comp(&[("Xx", 1)])).expect("the computed substance");
        assert_eq!(
            g.provenance,
            GapProvenance::ComputeOnce {
                functional: EigenvalueFunctional::Gw
            }
        );
    }

    #[test]
    fn the_compute_once_law_rejects_a_pbe_gap() {
        // THE COMPUTE-ONCE LAW as a build guard: a semilocal (PBE) functional is rejected at load, never admitted,
        // so no one wires a derivative-discontinuity-underestimated gap in good faith.
        let pbe = r#"
[[gap]]
name = "test PBE substance"
composition = { Xx = 1 }
gap_ev = "0.6"
provenance = "computed"
functional = "PBE"
source = "test-only fixture"
"#;
        assert!(matches!(
            BandGapColumn::from_toml_str(pbe),
            Err(BandGapError::ForbiddenFunctional(_))
        ));
        // An LDA gap is rejected the same way, and an unknown functional is rejected fail-closed.
        for bad_functional in ["LDA", "PBEsol", "mystery"] {
            let row = format!(
                "[[gap]]\nname = \"x\"\ncomposition = {{ Xx = 1 }}\ngap_ev = \"0.6\"\nprovenance = \"computed\"\nfunctional = \"{bad_functional}\"\nsource = \"test\"\n"
            );
            assert!(
                matches!(
                    BandGapColumn::from_toml_str(&row),
                    Err(BandGapError::ForbiddenFunctional(_))
                ),
                "{bad_functional} must be rejected by the compute-once law"
            );
        }
    }

    #[test]
    fn a_computed_gap_without_a_functional_is_rejected() {
        let no_func = r#"
[[gap]]
name = "x"
composition = { Xx = 1 }
gap_ev = "0.6"
provenance = "computed"
source = "test"
"#;
        assert!(matches!(
            BandGapColumn::from_toml_str(no_func),
            Err(BandGapError::MissingFunctional(_))
        ));
    }

    #[test]
    fn a_measured_gap_with_a_stray_functional_is_rejected() {
        // A measurement has no functional; a measured row carrying one is a provenance confusion, rejected.
        let stray = r#"
[[gap]]
name = "x"
composition = { Xx = 1 }
gap_ev = "1.0"
provenance = "measured"
functional = "GW"
source = "test"
"#;
        assert!(matches!(
            BandGapColumn::from_toml_str(stray),
            Err(BandGapError::StrayFunctional(_))
        ));
    }

    #[test]
    fn a_missing_citation_and_a_negative_gap_are_rejected() {
        let no_src = r#"
[[gap]]
name = "x"
composition = { Xx = 1 }
gap_ev = "1.0"
provenance = "measured"
source = ""
"#;
        assert!(matches!(
            BandGapColumn::from_toml_str(no_src),
            Err(BandGapError::MissingSource(_))
        ));
        let neg = r#"
[[gap]]
name = "x"
composition = { Xx = 1 }
gap_ev = "-0.5"
provenance = "measured"
source = "test"
"#;
        assert!(matches!(
            BandGapColumn::from_toml_str(neg),
            Err(BandGapError::NegativeGap(_))
        ));
    }
}
