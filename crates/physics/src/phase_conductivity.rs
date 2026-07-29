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

//! The per-phase columns the THERMAL-CONDUCTIVITY LADDER needs, read from the cited table in
//! `data/phase_conductivity.toml`. Sibling of [`crate::gruneisen`] and [`crate::mineral_moduli`]:
//! same eight phase keys, same canonical phase bridge, same refusal rather than a default.
//!
//! WHY THIS MODULE EXISTS. Both rungs of `civsim_materials::conductivity` key on ATOMS PER PRIMITIVE
//! CELL, which Slack's own docstring marks `(DATA)`, and no data file carried that column for any
//! phase. It was a function parameter and a struct field and nothing else, so `assemblage_conductivity`
//! had no production caller. This module supplies the column, and the measured `kappa_298` anchor for
//! the phases where a citable primary reports one.
//!
//! THE CELL COUNT IS RECONSTRUCTED, NEVER TRUSTED AS WRITTEN. Every row carries the space group, the
//! formula units per conventional cell `Z`, the atoms per formula unit, and the lattice points per
//! conventional cell, and [`PhaseConductivityTable::from_toml_str`] RECOMPUTES
//!
//! ```text
//! atoms_per_primitive_cell = Z * atoms_per_formula_unit / lattice_points_per_conventional_cell
//! ```
//!
//! refusing the row when the stated count disagrees or the division is not exact. A transcription slip
//! in any one of the four fields fails the load rather than shipping a wrong cell count into a
//! conductivity that then sets a lid thickness. That check earns its keep on two real traps this data
//! set contains: three of the eight phases sit on CENTRED lattices (periclase and spinel F, corundum
//! and hematite R), where the conventional cell holds several primitive cells; and the two independent
//! enstatite records disagree on `Z` (8 against 16) purely because they write the formula unit
//! differently (Mg2Si2O6 against MgSiO3), agreeing on 80 atoms, which is why the atoms per formula unit
//! is carried beside `Z` rather than `Z` alone.
//!
//! THE MEASURED ANCHOR IS OPTIONAL AND THE ABSENCE IS LOAD-BEARING. Seven of the eight phases carry a
//! cited `kappa_298`; SPINEL does not, because no retrieved source measures the stoichiometric phase.
//! Every spinel specimen in the literature reached is off-composition: Slack's 1962 crystal is the
//! alumina-rich Verneuil `MgO . 3.5 Al2O3`, and his two natural pleonastes are iron-bearing, so even
//! retrieving that paywalled paper would give a value for a composition the registry does not carry.
//! A phase with no anchor is NOT defaulted: it resolves through Slack's estimator rung and the aggregate
//! reports how much of its census was measured.
//!
//! HEMATITE'S ROW IS THE ONE ANCHOR WITH NO BAND, and that is a property of its source rather than an
//! omission. Akiyama et al. 1992 state no measurement accuracy, and no independent determination of
//! dense hematite at 300 K exists in what was retrieved to set a gap against, so the row carries a value
//! and no width. A measured row without a band contributes no width to an aggregate, which is a weaker
//! claim than the six banded rows make and is visible as such rather than filled in.
//!
//! HONEST LIMITS. The `kappa_298` rows are ambient-frame (300 K, 1 bar) and the ladder carries no
//! pressure dependence, so an aggregate built from them is an ambient-pressure quantity and a caller at
//! depth is reading outside the frame. The rows mix specimen forms (single crystal, dense polycrystal,
//! needle-probe aggregate) and each row states which, because that difference is larger than the
//! measurement scatter for periclase and is carried in its band.

use std::collections::BTreeMap;
use std::fmt;

use civsim_core::Fixed;

use crate::mineral_moduli::canonical_phase_key;

const ZERO: Fixed = Fixed::ZERO;

/// Why a phase-conductivity table failed to load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhaseConductivityError {
    /// A row could not be read.
    Parse(String),
    /// A decimal or integer value could not be parsed.
    BadValue(String),
    /// A row carries no citation for its crystallography (every column is cited-with-source).
    MissingSource(String),
    /// A conductivity is non-positive, or a band is negative.
    NonPhysical(String),
    /// THE RECONSTRUCTION CHECK FAILED: the stated atoms per primitive cell does not equal
    /// `Z * atoms_per_formula_unit / lattice_points_per_conventional_cell`, or that division is not
    /// exact. Either is a transcription defect, and either would silently mis-scale Slack's magnitude
    /// through its `n^(-2/3)` dependence.
    CellCountMismatch {
        /// The phase whose four crystallographic fields disagree.
        phase: String,
        /// The count the row asserts.
        stated: i32,
        /// The count the row's own space-group data reconstructs.
        reconstructed: i32,
    },
    /// A phase name appears twice.
    Duplicate(String),
    /// The table parsed to no rows at all.
    Empty,
}

impl fmt::Display for PhaseConductivityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseConductivityError::Parse(m) => write!(f, "phase-conductivity parse error: {m}"),
            PhaseConductivityError::BadValue(m) => write!(f, "phase-conductivity value error: {m}"),
            PhaseConductivityError::MissingSource(m) => {
                write!(f, "phase-conductivity row without citation: {m}")
            }
            PhaseConductivityError::NonPhysical(m) => {
                write!(f, "phase-conductivity non-physical value: {m}")
            }
            PhaseConductivityError::CellCountMismatch {
                phase,
                stated,
                reconstructed,
            } => write!(
                f,
                "phase {phase} states {stated} atoms per primitive cell but its own space group, Z, \
                 formula content and lattice centring reconstruct {reconstructed}. The count is \
                 reconstructed rather than trusted as written, because it enters Slack's magnitude as \
                 n^(-2/3) and a silent slip would mis-scale a rock's conductivity."
            ),
            PhaseConductivityError::Duplicate(m) => {
                write!(f, "duplicate phase-conductivity key: {m}")
            }
            PhaseConductivityError::Empty => {
                write!(f, "phase-conductivity table parsed to zero rows")
            }
        }
    }
}

impl std::error::Error for PhaseConductivityError {}

/// One phase's cited row: the crystallographic cell count with the data that reconstructs it, and the
/// measured ambient conductivity anchor where a primary reports one.
#[derive(Clone, Debug)]
pub struct PhaseConductivityRow {
    /// The phase name, as the registry spells it.
    pub name: String,
    /// Atoms per PRIMITIVE cell, the class variable both ladder rungs key on. Reconstructed at load.
    pub atoms_per_primitive_cell: i32,
    /// The space group symbol, carried so the count is checkable by a reader rather than only by code.
    pub space_group: String,
    /// Formula units per CONVENTIONAL cell.
    pub formula_units_per_conventional_cell: i32,
    /// Atoms per formula unit, carried beside `Z` because `Z` alone is ambiguous: it counts whichever
    /// formula unit the source chose to write.
    pub atoms_per_formula_unit: i32,
    /// Lattice points per conventional cell: 1 for P, 2 for I or a base centring, 3 for R in the
    /// hexagonal setting, 4 for F. This is the factor that separates the conventional cell from the
    /// primitive one, and getting it wrong is the trap this column exists to foreclose.
    pub lattice_points_per_conventional_cell: i32,
    /// The measured conductivity anchor at 300 K (W/(m*K)), when a primary reports a pure-phase value.
    /// `None` sends the phase to Slack's estimator rung.
    pub kappa_298: Option<Fixed>,
    /// The anchor's symmetric half-width, derived from the source's stated accuracy or from the gap to
    /// an independent second determination, whichever is wider. Never fabricated.
    pub kappa_298_band: Option<Fixed>,
    /// The specimen form the anchor was measured on, carried because it is the difference between a
    /// phase property and a sample property.
    pub kappa_specimen_form: Option<String>,
}

/// The cited per-phase conductivity-input table.
#[derive(Clone, Debug)]
pub struct PhaseConductivityTable {
    rows: BTreeMap<String, PhaseConductivityRow>,
}

/// The VOIGT-REUSS-HILL orientational average of a uniaxial crystal's principal conductivities
/// (W/(m*K)), with the half-width of the bracket it was taken from.
///
/// A randomly oriented aggregate of an anisotropic crystal is bracketed by the parallel and series
/// averages of its principal values, exactly as an aggregate of differently-conducting phases is
/// (the same bracket `civsim_materials::conductivity` asserts on its own mixing rule):
///
/// ```text
/// voigt = (k_par + 2 k_perp) / 3        the parallel bound, the trace/3 of the conductivity tensor
/// reuss = 3 / (1/k_par + 2/k_perp)      the series bound
/// ```
///
/// and this returns their midpoint with the half-gap, which is the SAME Hill convention
/// `mineral_moduli.toml` already uses to reduce a measured elastic tensor to one aggregate number. The
/// convention is stated rather than chosen per call site, so no caller picks an averaging rule.
///
/// `None` on a non-positive principal value or a fixed-point intermediate leaving the window.
// @derives: a uniaxial phase's isotropic-aggregate conductivity and its orientational band <- the measured principal conductivities (Voigt-Reuss-Hill)
pub fn hill_average_conductivity(parallel: Fixed, perpendicular: Fixed) -> Option<(Fixed, Fixed)> {
    if parallel <= ZERO || perpendicular <= ZERO {
        return None;
    }
    let three = Fixed::from_int(3);
    let two = Fixed::from_int(2);
    let voigt = parallel
        .checked_add(two.checked_mul(perpendicular)?)?
        .checked_div(three)?;
    let reciprocal_sum = Fixed::ONE
        .checked_div(parallel)?
        .checked_add(two.checked_div(perpendicular)?)?;
    if reciprocal_sum <= ZERO {
        return None;
    }
    let reuss = three.checked_div(reciprocal_sum)?;
    let hill = voigt.checked_add(reuss)?.checked_div(two)?;
    let half_gap = voigt.checked_sub(reuss)?.checked_div(two)?;
    Some((hill, if half_gap > ZERO { half_gap } else { ZERO }))
}

impl PhaseConductivityTable {
    /// Load the vendored table.
    pub fn standard() -> Result<Self, PhaseConductivityError> {
        Self::from_toml_str(include_str!("../data/phase_conductivity.toml"))
    }

    /// Parse the `[[conductivity]]` blocks. Decimal values are QUOTED strings parsed to fixed-point
    /// through [`Fixed::from_decimal_str`], so no float ever enters; the crystallographic counts are
    /// integers and are parsed as such, which is exact by construction.
    pub fn from_toml_str(s: &str) -> Result<Self, PhaseConductivityError> {
        let mut rows: BTreeMap<String, PhaseConductivityRow> = BTreeMap::new();
        // Split on LINE-ANCHORED block headers, never on the bare string: the file's own header prose
        // names `[[conductivity]]` while explaining the block-kind idiom, so a bare split would
        // manufacture a phantom block out of comment text (the trap the Gruneisen loader records).
        let mut blocks: Vec<String> = Vec::new();
        let mut current: Option<Vec<&str>> = None;
        for line in s.lines() {
            if line.trim() == "[[conductivity]]" {
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
                return Err(PhaseConductivityError::Parse(
                    "a [[conductivity]] block has no name".into(),
                ));
            };
            // Every cited row carries its crystallographic source; a row without one is not cited data.
            if field("crystallography_citation").is_none() {
                return Err(PhaseConductivityError::MissingSource(name));
            }
            let count = |key: &str| -> Result<i32, PhaseConductivityError> {
                let raw = field(key)
                    .ok_or_else(|| PhaseConductivityError::Parse(format!("{name} has no {key}")))?;
                raw.trim()
                    .parse::<i32>()
                    .map_err(|_| PhaseConductivityError::BadValue(format!("{name}.{key} = {raw}")))
            };
            let stated = count("atoms_per_primitive_cell")?;
            let z = count("formula_units_per_conventional_cell")?;
            let atoms_per_formula_unit = count("atoms_per_formula_unit")?;
            let lattice_points = count("lattice_points_per_conventional_cell")?;
            if z < 1 || atoms_per_formula_unit < 1 || lattice_points < 1 {
                return Err(PhaseConductivityError::NonPhysical(format!(
                    "{name}: Z, atoms per formula unit and lattice points are all positive counts"
                )));
            }
            // THE RECONSTRUCTION CHECK. The conventional cell's atom content must divide exactly by the
            // centring multiplicity, and the quotient must be the count the row asserts.
            let conventional = z.checked_mul(atoms_per_formula_unit).ok_or_else(|| {
                PhaseConductivityError::BadValue(format!("{name}: Z * atoms overflow"))
            })?;
            if conventional % lattice_points != 0 {
                return Err(PhaseConductivityError::CellCountMismatch {
                    phase: name.clone(),
                    stated,
                    reconstructed: -1,
                });
            }
            let reconstructed = conventional / lattice_points;
            if reconstructed != stated {
                return Err(PhaseConductivityError::CellCountMismatch {
                    phase: name.clone(),
                    stated,
                    reconstructed,
                });
            }
            let num = |key: &str| -> Result<Option<Fixed>, PhaseConductivityError> {
                match field(key) {
                    None => Ok(None),
                    Some(raw) => Fixed::from_decimal_str(raw.trim()).map(Some).map_err(|_| {
                        PhaseConductivityError::BadValue(format!("{name}.{key} = {raw}"))
                    }),
                }
            };
            // The anchor is either an isotropic scalar, or a pair of principal values the loader
            // reduces by the declared Hill convention. A row supplying neither has no anchor, which is
            // a real loaded state: spinel is the cited instance.
            let scalar = num("kappa_298_w_per_m_k")?;
            let parallel = num("kappa_298_parallel_c")?;
            let perpendicular = num("kappa_298_perpendicular_c")?;
            let (kappa_298, orientational_band) = match (scalar, parallel, perpendicular) {
                (Some(k), _, _) => (Some(k), None),
                (None, Some(par), Some(perp)) => {
                    let (hill, half_gap) =
                        hill_average_conductivity(par, perp).ok_or_else(|| {
                            PhaseConductivityError::NonPhysical(format!(
                                "{name}: principal conductivities do not reduce"
                            ))
                        })?;
                    (Some(hill), Some(half_gap))
                }
                _ => (None, None),
            };
            if let Some(k) = kappa_298 {
                if k <= ZERO {
                    return Err(PhaseConductivityError::NonPhysical(format!(
                        "{name}.kappa_298 = {k:?}"
                    )));
                }
            }
            // The band is the WIDEST of the bases the row supplies, so a stated accuracy, an
            // independent second determination and an averaged-away anisotropy each raise it and none
            // lowers it. A row with no basis carries no band rather than a fabricated one.
            let explicit = num("kappa_298_band")?;
            let relative = num("kappa_298_relative_uncertainty")?;
            let from_relative = match (relative, kappa_298) {
                (Some(r), Some(k)) if r > ZERO => r.checked_mul(k),
                _ => None,
            };
            let mut kappa_298_band: Option<Fixed> = None;
            for c in [explicit, from_relative, orientational_band]
                .into_iter()
                .flatten()
            {
                if c < ZERO {
                    return Err(PhaseConductivityError::NonPhysical(format!(
                        "{name}: negative band"
                    )));
                }
                if kappa_298_band.is_none_or(|held| c > held) {
                    kappa_298_band = Some(c);
                }
            }
            let row = PhaseConductivityRow {
                name: name.clone(),
                atoms_per_primitive_cell: stated,
                space_group: field("space_group").unwrap_or_default(),
                formula_units_per_conventional_cell: z,
                atoms_per_formula_unit,
                lattice_points_per_conventional_cell: lattice_points,
                kappa_298,
                kappa_298_band,
                kappa_specimen_form: field("kappa_specimen_form"),
            };
            let key = canonical_phase_key(&name).to_string();
            if rows.insert(key.clone(), row).is_some() {
                return Err(PhaseConductivityError::Duplicate(key));
            }
        }
        if rows.is_empty() {
            return Err(PhaseConductivityError::Empty);
        }
        Ok(Self { rows })
    }

    /// The row for a phase, resolving both phase-naming conventions through the SAME canonical key the
    /// moduli and Gruneisen floors use.
    pub fn row(&self, name: &str) -> Option<&PhaseConductivityRow> {
        self.rows.get(canonical_phase_key(name))
    }

    /// A phase's atoms per primitive cell, the class variable both ladder rungs key on.
    pub fn atoms_per_primitive_cell(&self, name: &str) -> Option<i32> {
        self.row(name).map(|r| r.atoms_per_primitive_cell)
    }

    /// A phase's measured ambient conductivity anchor with its band, where one is cited. `None` means
    /// the phase resolves through Slack's estimator rung, which is a real state and never a zero.
    pub fn kappa_298(&self, name: &str) -> Option<(Fixed, Option<Fixed>)> {
        self.row(name)
            .and_then(|r| r.kappa_298.map(|k| (k, r.kappa_298_band)))
    }

    /// EVERY banked row, in canonical-key order.
    ///
    /// The walk is a `BTreeMap` walk over the canonical keys, so it is ordered by key and NOT by the
    /// file's line order. That matters for determinism (Principle 3): a consumer folding over these rows
    /// gets the same sequence whoever edits the data file and wherever they insert a block.
    ///
    /// This exists because a consumer that needs a statistic ACROSS the column, rather than one phase's
    /// row, cannot assemble one from [`Self::row`] without a list of names to ask for, and a hardcoded
    /// name list inside such a consumer would go stale the moment a phase is banked.
    pub fn rows(&self) -> impl Iterator<Item = &PhaseConductivityRow> + '_ {
        self.rows.values()
    }

    /// How many phases the table carries.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table is empty (it never is once loaded; the loader refuses an empty table).
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Fixed {
        Fixed::from_decimal_str(s).expect("a decimal literal parses")
    }

    #[test]
    fn the_vendored_table_loads_and_carries_every_registry_phase() {
        let t = PhaseConductivityTable::standard().expect("the vendored table loads");
        assert_eq!(t.len(), 8, "the eight registry phases, got {}", t.len());
        assert!(!t.is_empty());
        for phase in [
            "quartz",
            "corundum",
            "periclase",
            "hematite",
            "forsterite",
            "fayalite",
            "spinel",
            "enstatite",
        ] {
            assert!(t.row(phase).is_some(), "{phase} is a cited row");
        }
    }

    /// THE VALUES PINNED TO THEIR CITED ROWS. Each of these is a number read off a named table in a
    /// named document, so a drifted transcription fails here rather than in a lid thickness.
    #[test]
    fn the_cell_counts_read_their_cited_values() {
        let t = PhaseConductivityTable::standard().expect("loads");
        // Space group and Z per the cited crystallographic records, reduced by the lattice centring.
        for (phase, expect) in [
            ("quartz", 9),      // P3(1)21, Z=3, 3 atoms/fu, P
            ("corundum", 10),   // R-3c hex, Z=6, 5 atoms/fu, R (3 lattice points)
            ("periclase", 2),   // Fm-3m, Z=4, 2 atoms/fu, F (4 lattice points)
            ("hematite", 10),   // R-3c hex, Z=6, 5 atoms/fu, R
            ("forsterite", 28), // Pbnm, Z=4, 7 atoms/fu, P
            ("fayalite", 28),   // Pbnm, Z=4, 7 atoms/fu, P
            ("spinel", 14),     // Fd-3m, Z=8, 7 atoms/fu, F
            ("enstatite", 80),  // Pbca, Z=16, 5 atoms/fu, P
        ] {
            assert_eq!(
                t.atoms_per_primitive_cell(phase),
                Some(expect),
                "{phase} reads its cited primitive-cell count"
            );
        }
    }

    /// The counts must land the phases in the ladder's calibrated classes. This is the check that the
    /// column is USABLE and not merely present: `lattice_exponent_for_cell` refuses `2 < n < 6`, so a
    /// count that fell in that gap would refuse the phase outright.
    #[test]
    fn every_phase_lands_outside_the_uncalibrated_cell_gap() {
        let t = PhaseConductivityTable::standard().expect("loads");
        for phase in [
            "quartz",
            "corundum",
            "periclase",
            "hematite",
            "forsterite",
            "fayalite",
            "spinel",
            "enstatite",
        ] {
            let n = t.atoms_per_primitive_cell(phase).expect("cited");
            assert!(
                n <= 2 || n >= 6,
                "{phase} has n = {n}, inside the 2 < n < 6 gap the cited calibration cannot place"
            );
        }
        // Periclase is the only SIMPLE-class phase, so it is the only overlap point that class has. The
        // note here used to call it the ladder's only overlap sentinel outright, and that was measured
        // false: FIVE banked phases carry both rungs, four of them in the complex class, which is what
        // `civsim_materials::conductivity::derived_estimator_bands` reads to size the estimator's band.
        assert_eq!(t.atoms_per_primitive_cell("periclase"), Some(2));
    }

    /// THE CONVENTIONAL CELL IS NOT THE PRIMITIVE CELL, and this test states the difference in numbers
    /// rather than in prose. For the three centred phases the naive product overstates the count by the
    /// centring multiplicity, and that error would propagate through Slack's `n^(-2/3)`.
    #[test]
    fn the_centred_lattices_are_reduced_and_the_uncentred_ones_are_not() {
        let t = PhaseConductivityTable::standard().expect("loads");
        for (phase, points) in [
            ("periclase", 4),
            ("spinel", 4),
            ("corundum", 3),
            ("hematite", 3),
        ] {
            let r = t.row(phase).expect("cited");
            assert_eq!(r.lattice_points_per_conventional_cell, points);
            let conventional = r.formula_units_per_conventional_cell * r.atoms_per_formula_unit;
            assert_eq!(
                conventional / points,
                r.atoms_per_primitive_cell,
                "{phase}: the conventional content {conventional} reduces by {points}"
            );
            assert!(
                conventional > r.atoms_per_primitive_cell,
                "{phase} is centred, so its conventional cell holds strictly more"
            );
        }
        for phase in ["quartz", "forsterite", "fayalite", "enstatite"] {
            let r = t.row(phase).expect("cited");
            assert_eq!(r.lattice_points_per_conventional_cell, 1);
            assert_eq!(
                r.formula_units_per_conventional_cell * r.atoms_per_formula_unit,
                r.atoms_per_primitive_cell,
                "{phase} is primitive, so the conventional cell IS the primitive cell"
            );
        }
    }

    /// THE REFUSAL IS THE LOAD-BEARING BEHAVIOUR, and it is the reason the count is reconstructed
    /// rather than trusted. A row whose stated count disagrees with its own space-group data must fail
    /// the load naming the phase and both numbers, never load the asserted value.
    #[test]
    fn a_cell_count_that_contradicts_its_own_crystallography_is_refused() {
        // Forsterite's real row with the count corrupted: Pbnm, Z = 4, 7 atoms per formula unit and a
        // primitive lattice reconstruct 28, so an asserted 7 (the atoms-per-FORMULA-UNIT slip the
        // geodynamics blocker note names as the tempting substitution) must be caught.
        let s = "[[conductivity]]\nname = \"forsterite\"\natoms_per_primitive_cell = \"7\"\n\
                 space_group = \"Pbnm\"\nlattice_points_per_conventional_cell = \"1\"\n\
                 formula_units_per_conventional_cell = \"4\"\natoms_per_formula_unit = \"7\"\n\
                 crystallography_citation = \"x\"\n";
        let err = PhaseConductivityTable::from_toml_str(s).expect_err("must refuse");
        assert_eq!(
            err,
            PhaseConductivityError::CellCountMismatch {
                phase: "forsterite".to_string(),
                stated: 7,
                reconstructed: 28,
            }
        );
        assert!(
            err.to_string()
                .contains("reconstructed rather than trusted"),
            "the refusal explains why the count is not taken as written: {err}"
        );

        // And the centring trap in the other direction: spinel's conventional 56 atoms with the F
        // centring dropped would assert 56 where the row's own data reconstructs 14.
        let s2 = "[[conductivity]]\nname = \"spinel\"\natoms_per_primitive_cell = \"56\"\n\
                  space_group = \"Fd-3m\"\nlattice_points_per_conventional_cell = \"4\"\n\
                  formula_units_per_conventional_cell = \"8\"\natoms_per_formula_unit = \"7\"\n\
                  crystallography_citation = \"x\"\n";
        assert!(matches!(
            PhaseConductivityTable::from_toml_str(s2),
            Err(PhaseConductivityError::CellCountMismatch {
                stated: 56,
                reconstructed: 14,
                ..
            })
        ));
    }

    #[test]
    fn a_row_without_a_citation_is_rejected_at_load() {
        let s = "[[conductivity]]\nname = \"ghost\"\natoms_per_primitive_cell = \"2\"\n";
        assert!(matches!(
            PhaseConductivityTable::from_toml_str(s),
            Err(PhaseConductivityError::MissingSource(_))
        ));
    }

    /// The measured anchors read their cited values. These are the numbers that let the ladder's TOP
    /// rung evaluate at all, so each is pinned to the table it was read from.
    #[test]
    fn the_measured_anchors_read_their_cited_values() {
        let t = PhaseConductivityTable::standard().expect("loads");
        // NSRDS-NBS 8 Table 14, 300 K, Watt cm^-1 K^-1 converted by a factor of 100.
        let (periclase, band) = t
            .kappa_298("periclase")
            .expect("periclase carries an anchor");
        assert_eq!(periclase, dec("48.4"));
        assert_eq!(
            band,
            Some(dec("11.6")),
            "the band is the gap to the independent single-crystal determination, not the narrower \
             stated accuracy"
        );
        assert_eq!(t.kappa_298("corundum").expect("cited").0, dec("36.0"));
        // Henke et al. 2016 Table 2, from Horai and Simmons 1969.
        assert_eq!(t.kappa_298("forsterite").expect("cited").0, dec("5.158"));
        assert_eq!(t.kappa_298("fayalite").expect("cited").0, dec("3.161"));
        assert_eq!(t.kappa_298("enstatite").expect("cited").0, dec("4.961"));
        // Akiyama et al. 1992 ISIJ Int. 32, 829, equation (2), the Fe2O3 branch `k = 1/(1.844e-4 T)`
        // evaluated at the 298 K lower bound of its own stated range. The ADJACENT branch in that paper
        // is `k = 1/(1.693e-4 T)` for Fe3O4, which would give 19.82, so this assertion is also the guard
        // against having read the magnetite row by position instead of the hematite row by its heading.
        assert_eq!(t.kappa_298("hematite").expect("cited").0, dec("18.20"));
    }

    /// THE ONE ANCHOR WITH NO BAND, asserted so the absence stays deliberate. Every other cited row
    /// carries a width, from a stated accuracy or from the gap to a second determination. Akiyama et al.
    /// state no accuracy for the laser-flash measurement and no independent determination of dense
    /// hematite at 300 K was retrieved, so there is nothing to derive a width from and none is invented.
    #[test]
    fn the_hematite_anchor_carries_a_value_and_no_band() {
        let t = PhaseConductivityTable::standard().expect("loads");
        let (k, band) = t.kappa_298("hematite").expect("cited");
        assert_eq!(k, dec("18.20"));
        assert!(
            band.is_none(),
            "the source states no accuracy and no second in-frame determination exists, so the row \
             carries no width rather than a fabricated one"
        );
        // And it is the ONLY anchored row without one, so a later bandless row is a change, not a habit.
        for phase in [
            "quartz",
            "corundum",
            "periclase",
            "forsterite",
            "fayalite",
            "enstatite",
        ] {
            assert!(
                t.kappa_298(phase).expect("cited").1.is_some(),
                "{phase} carries a band"
            );
        }
    }

    /// AN ABSENT ANCHOR IS A REAL LOADED STATE, NOT A ZERO. Spinel deliberately carries no measured
    /// value, so it resolves through Slack's estimator rung and the aggregate reports a measured weight
    /// fraction below one. A default here would author a rock's heat transport. The absence survived the
    /// 2026-07-20 fetch for a sharper reason than before: Slack's own 1962 spinel crystals are the
    /// alumina-rich `MgO . 3.5 Al2O3` and two iron-bearing natural pleonastes, so the paper long named as
    /// the follow-up would not have closed this row.
    #[test]
    fn the_one_phase_with_no_cited_anchor_loads_but_reports_no_measurement() {
        let t = PhaseConductivityTable::standard().expect("loads");
        assert!(
            t.row("spinel").is_some(),
            "spinel is a cited row: its crystallography is known"
        );
        assert!(
            t.kappa_298("spinel").is_none(),
            "spinel deliberately carries no measured anchor, so none is reported"
        );
        // And the seven that do carry one are exactly the seven.
        let with = [
            "quartz",
            "corundum",
            "periclase",
            "forsterite",
            "fayalite",
            "enstatite",
            "hematite",
        ];
        for phase in with {
            assert!(t.kappa_298(phase).is_some(), "{phase} carries an anchor");
        }
    }

    /// The uniaxial reduction is checked against its own bracket rather than against a restatement of
    /// the formula: the Hill value must lie strictly between the parallel and series bounds, and both
    /// bounds must lie between the principal values.
    #[test]
    fn the_hill_average_lies_strictly_inside_its_own_bracket() {
        let par = dec("10.4");
        let perp = dec("6.21");
        let (hill, half_gap) = hill_average_conductivity(par, perp).expect("reduces");
        let three = Fixed::from_int(3);
        let two = Fixed::from_int(2);
        let voigt = par
            .checked_add(two.checked_mul(perp).unwrap())
            .unwrap()
            .checked_div(three)
            .unwrap();
        let reuss = three
            .checked_div(
                Fixed::ONE
                    .checked_div(par)
                    .unwrap()
                    .checked_add(two.checked_div(perp).unwrap())
                    .unwrap(),
            )
            .unwrap();
        assert!(reuss < voigt, "the series bound is the lower one");
        assert!(hill > reuss && hill < voigt, "Hill sits inside the bracket");
        assert!(hill > perp && hill < par, "and inside the principal values");
        assert!(
            half_gap > ZERO,
            "an anisotropic crystal carries a bracket width"
        );
        // Magnitude spot-check against the world rather than against the algebra: the reduction of the
        // cited quartz pair must land near the accepted polycrystalline quartz conductivity of ~7.7.
        assert!(
            hill > dec("7.0") && hill < dec("8.0"),
            "the quartz aggregate lands near the accepted polycrystalline value, got {}",
            hill.to_f64_lossy()
        );
        // An isotropic input degenerates to itself with no width.
        let (same, gap) = hill_average_conductivity(dec("5"), dec("5")).expect("reduces");
        assert!((same - dec("5")).abs() < dec("0.001"));
        assert!(
            gap < dec("0.001"),
            "an isotropic crystal carries no orientational width"
        );
        assert!(hill_average_conductivity(ZERO, perp).is_none());
    }

    /// The quartz row's anchor is the DERIVED Hill average of its cited principal values, and its band
    /// is widened past the orientational bracket by the source's own stated accuracy.
    #[test]
    fn the_anisotropic_row_is_reduced_by_the_loader_rather_than_asserted() {
        let t = PhaseConductivityTable::standard().expect("loads");
        let (k, band) = t.kappa_298("quartz").expect("quartz carries an anchor");
        let (expect, bracket) =
            hill_average_conductivity(dec("10.4"), dec("6.21")).expect("reduces");
        assert_eq!(k, expect, "the stored anchor is the loader's own reduction");
        let band = band.expect("quartz carries a band");
        assert!(
            band > bracket,
            "the source's stated 5 percent accuracy is wider than the orientational bracket, so it \
             sets the band: band={} bracket={}",
            band.to_f64_lossy(),
            bracket.to_f64_lossy()
        );
    }

    /// The canonical phase bridge resolves the JANAF-decorated crust spelling to the same row, so a
    /// phase named by the condensation solver and one named by the petrology kernel reach one record.
    #[test]
    fn the_decorated_crust_names_resolve_to_the_same_row() {
        let t = PhaseConductivityTable::standard().expect("loads");
        assert_eq!(
            t.atoms_per_primitive_cell("MgSiO3(cr,enstatite)"),
            Some(80),
            "the decorated name reaches the enstatite row"
        );
        assert_eq!(
            t.atoms_per_primitive_cell("Mg2SiO4(cr,forsterite)"),
            Some(28)
        );
        assert!(
            t.row("unobtainium").is_none(),
            "an uncited phase has no row"
        );
    }
}
