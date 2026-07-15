//! The composition-selecting ÆSOPUS opacity-grid registry: given a disk composition (X, Z, C/O), select the [M]
//! low-temperature opacity grid to read, as a [`Verdict`] (the Gap Law typestate), with the no-cross-C/O=1 rule
//! encoded in the TYPE rather than a comment.
//!
//! The physics crate provides the grid primitive ([`LowTempRosselandGrid`], loaded from the vendored ÆSOPUS
//! tables); this registry lives up-stack because it binds to the Verdict machinery (the Gap Law is a materials
//! concern) and keys on the fetch's POST parameters. A selection outcome is the same typestate every other
//! sub-resolution returns: a clear winner is [`Verdict::Decided`] or [`Verdict::Trivial`], and the two escalation
//! cases (a query in the carbide-flip bistable zone, or a composition with no in-regime grid) surface as
//! [`Verdict::Escalate`] through the sealed [`dispose`] constructor, so the Gap Law bookkeeping sees them and no
//! bespoke error hides an unresolved selection.

use crate::verdict::{dispose, Candidate, ProvenanceKey, TieSlot, Verdict};
use civsim_core::{Fixed, StateHasher};
use civsim_physics::molecular_opacity::LowTempRosselandGrid;

/// The carbon-to-oxygen composition REGIME, a type that makes crossing the carbide flip (C/O = 1) unrepresentable.
/// Below the bistable zone the chemistry is water/oxide-dominated (`WaterRich`); inside `[0.8, 1.0]` the water and
/// carbon reservoirs are near-degenerate (`Bistable`, the banked zone the disposer escalates); above 1.0 it is
/// carbide/CN-dominated (`CarbonRich`). A single value is ONE side of the flip; two grids that straddle C/O = 1 are
/// different regimes and never interpolate (a linear blend of a 0.8 and a 1.2 table manufactures a gas that exists
/// at no composition, the carbide flip being a first-order chemistry boundary, not a smooth axis).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CarbonOxygenRegime {
    /// C/O below the bistable zone: water and oxides own the molecular bands.
    WaterRich,
    /// C/O in the banked bistable zone near the carbide flip: escalate, never interpolate.
    Bistable,
    /// C/O above the flip: carbides and CN own the bands.
    CarbonRich,
}

impl CarbonOxygenRegime {
    /// Classify a C/O ratio. The `0.8` and `1.0` bounds are the bistable-zone edges (reserved calibration, basis the
    /// carbide-condensation transition width around C/O = 1; the `1.0` edge is the flip itself).
    pub fn classify(carbon_to_oxygen: Fixed) -> Self {
        if carbon_to_oxygen < Fixed::from_ratio(8, 10) {
            CarbonOxygenRegime::WaterRich
        } else if carbon_to_oxygen <= Fixed::ONE {
            CarbonOxygenRegime::Bistable
        } else {
            CarbonOxygenRegime::CarbonRich
        }
    }
}

/// A registry entry's composition key, keyed on the fetch's POST PARAMETERS (X = `xhmin`, Z = `zeta_ref`, C/O =
/// `(C/O)_ref * 10^fco1`), never the filename. Its content identity (for the Verdict's canonical ordering and
/// content-hash draw) is the composition; the `index` into the grid table is carried for lookup and is NOT fed
/// (it is enumeration order, not content).
#[derive(Clone, Copy, Debug)]
pub struct GridComposition {
    /// Hydrogen mass fraction `X` (the `xhmin` POST parameter).
    pub hydrogen_mass_fraction: Fixed,
    /// Metallicity `Z` (the `zeta_ref` POST parameter).
    pub metallicity: Fixed,
    /// Carbon-to-oxygen ratio `C/O` (the reference C/O times `10^fco1`).
    pub carbon_to_oxygen: Fixed,
    /// Index into the registry's grid table (lookup only, not content).
    pub index: usize,
}

impl Candidate for GridComposition {
    fn feed_content(&self, hasher: &mut StateHasher) {
        hasher.write_i64(self.hydrogen_mass_fraction.to_bits());
        hasher.write_i64(self.metallicity.to_bits());
        hasher.write_i64(self.carbon_to_oxygen.to_bits());
    }
}

/// Distinct provenance keys for the escalation reasons: the same `Escalate` TYPESTATE, but distinct tags so the
/// Gap Law bookkeeping can tell a bistable-zone escalation from a composition miss (the pull-on-miss).
const PK_SELECT: ProvenanceKey = ProvenanceKey(0xAE50_0001);
const PK_BISTABLE: ProvenanceKey = ProvenanceKey(0xAE50_0002);
const PK_MISS: ProvenanceKey = ProvenanceKey(0xAE50_0003);
const SLOT: TieSlot = TieSlot(0xAE50);

/// The composition-selection resolution: two grids whose composition distances differ by less than this are treated
/// as equally good, and the selection escalates rather than picking arbitrarily. Reserved calibration (basis: the
/// grid spacing below which the opacity difference falls under the table's own dex precision).
fn selection_resolution() -> Fixed {
    Fixed::from_ratio(1, 100)
}

/// `|a - b|` in `Fixed`, saturating on the unreachable full-range overflow.
fn abs_diff(a: Fixed, b: Fixed) -> Fixed {
    let d = a.checked_sub(b).unwrap_or(Fixed::MAX);
    if d < Fixed::ZERO {
        Fixed::ZERO.checked_sub(d).unwrap_or(Fixed::MAX)
    } else {
        d
    }
}

/// `|ln(a/b)| = |ln a - ln b|`, the log-space gap for the positive composition quantities (metallicity, C/O).
fn abs_log_ratio(a: Fixed, b: Fixed) -> Fixed {
    abs_diff(a.ln(), b.ln())
}

/// The composition distance (the [`dispose`] energy, lower = closer): the log-metallicity gap plus the log-(C/O)
/// gap plus the hydrogen-fraction gap. Within a regime the nearest grid wins; a near-tie escalates.
fn composition_distance(gc: &GridComposition, query: (Fixed, Fixed, Fixed)) -> Fixed {
    let (x, z, co) = query;
    abs_log_ratio(gc.metallicity, z)
        .checked_add(abs_log_ratio(gc.carbon_to_oxygen, co))
        .unwrap_or(Fixed::MAX)
        .checked_add(abs_diff(gc.hydrogen_mass_fraction, x))
        .unwrap_or(Fixed::MAX)
}

/// A registry of composition-keyed low-temperature opacity grids, selecting one for a disk composition as a
/// Verdict.
pub struct OpacityGridRegistry {
    table: Vec<(GridComposition, LowTempRosselandGrid)>,
}

impl OpacityGridRegistry {
    /// Build a registry from `(X, Z, C/O, grid)` tuples (the composition from the POST parameters). Indices are
    /// assigned in order.
    pub fn new(grids: Vec<(Fixed, Fixed, Fixed, LowTempRosselandGrid)>) -> Self {
        let table = grids
            .into_iter()
            .enumerate()
            .map(|(index, (x, z, co, grid))| {
                (
                    GridComposition {
                        hydrogen_mass_fraction: x,
                        metallicity: z,
                        carbon_to_oxygen: co,
                        index,
                    },
                    grid,
                )
            })
            .collect();
        OpacityGridRegistry { table }
    }

    /// The number of registered grids.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Whether the registry holds no grids.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Select the grid for a composition query, as a Verdict. The no-cross-C/O rule lives HERE: a query in the
    /// BISTABLE zone escalates (never interpolate across the carbide flip); otherwise [`dispose`] runs over the
    /// SAME-regime grids by composition distance, so the nearest is `Decided` (or `Escalate` on a near-tie). A miss
    /// (no in-regime grid) escalates through the empty-set path, the FORBIDDEN-FALLBACK rider: a loud hold routed up
    /// the ladder, never a silent substitution of the continuum closure (a certified order-of-magnitude
    /// underestimate wearing a derived pedigree). Every escalation is a `Verdict::Escalate`, tagged by its
    /// provenance key.
    pub fn select(&self, x: Fixed, z: Fixed, carbon_to_oxygen: Fixed) -> Verdict<GridComposition> {
        if CarbonOxygenRegime::classify(carbon_to_oxygen) == CarbonOxygenRegime::Bistable {
            return dispose(
                Vec::new(),
                |_: &GridComposition| Fixed::ZERO,
                selection_resolution(),
                PK_BISTABLE,
                SLOT,
            );
        }
        let regime = CarbonOxygenRegime::classify(carbon_to_oxygen);
        let candidates: Vec<GridComposition> = self
            .table
            .iter()
            .map(|(gc, _)| *gc)
            .filter(|gc| CarbonOxygenRegime::classify(gc.carbon_to_oxygen) == regime)
            .collect();
        let query = (x, z, carbon_to_oxygen);
        let pk = if candidates.is_empty() {
            PK_MISS
        } else {
            PK_SELECT
        };
        dispose(
            candidates,
            move |gc: &GridComposition| composition_distance(gc, query),
            selection_resolution(),
            pk,
            SLOT,
        )
    }

    /// Read the grid a `Decided`/`Trivial` verdict selected.
    pub fn grid(&self, comp: &GridComposition) -> &LowTempRosselandGrid {
        &self.table[comp.index].1
    }

    /// The standard registry over the vendored ÆSOPUS grids (loaded from the physics manifest, keyed on the POST
    /// parameters). `None` if the manifest fails to load (fail loud, never a partial registry).
    pub fn standard() -> Option<Self> {
        let grids = civsim_physics::molecular_opacity::aesopus_vendored_grids()?
            .into_iter()
            .map(|v| {
                (
                    v.hydrogen_mass_fraction,
                    v.metallicity,
                    v.carbon_to_oxygen,
                    v.grid,
                )
            })
            .collect();
        Some(OpacityGridRegistry::new(grids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toy_grid(x: Fixed, z: Fixed) -> LowTempRosselandGrid {
        // A minimal 2x2 grid; the registry test cares about SELECTION, not the opacity values.
        LowTempRosselandGrid {
            hydrogen_mass_fraction: x,
            metallicity: z,
            log_t: vec![Fixed::from_int(3), Fixed::from_int(4)],
            log_r: vec![Fixed::from_int(-4), Fixed::from_int(0)],
            log_kappa: vec![
                vec![Fixed::ZERO, Fixed::ZERO],
                vec![Fixed::ZERO, Fixed::ZERO],
            ],
        }
    }

    fn co(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    #[test]
    fn the_regime_type_splits_at_the_carbide_flip() {
        assert_eq!(
            CarbonOxygenRegime::classify(co(55, 100)),
            CarbonOxygenRegime::WaterRich
        );
        assert_eq!(
            CarbonOxygenRegime::classify(co(9, 10)),
            CarbonOxygenRegime::Bistable
        );
        assert_eq!(
            CarbonOxygenRegime::classify(Fixed::ONE),
            CarbonOxygenRegime::Bistable
        );
        assert_eq!(
            CarbonOxygenRegime::classify(co(12, 10)),
            CarbonOxygenRegime::CarbonRich
        );
    }

    fn registry() -> OpacityGridRegistry {
        // Two water-rich grids (differing in Z) and one carbon-rich grid.
        OpacityGridRegistry::new(vec![
            (
                co(7, 10),
                co(1, 1000),
                co(55, 100),
                toy_grid(co(7, 10), co(1, 1000)),
            ),
            (
                co(7, 10),
                co(2, 100),
                co(55, 100),
                toy_grid(co(7, 10), co(2, 100)),
            ),
            (
                co(7, 10),
                co(14, 1000),
                co(12, 10),
                toy_grid(co(7, 10), co(14, 1000)),
            ),
        ])
    }

    #[test]
    fn a_within_regime_query_decides_the_nearest_grid() {
        // Water-rich query at Z = 0.0015 selects the nearer low-Z water-rich grid (Z = 0.001), never the carbon-rich
        // grid: the candidate set is regime-filtered, so the carbide flip is never crossed.
        let reg = registry();
        match reg.select(co(7, 10), co(15, 10000), co(55, 100)) {
            Verdict::Decided(d) => {
                assert_eq!(d.winner().index, 0, "the low-Z water-rich grid wins");
                assert_eq!(
                    CarbonOxygenRegime::classify(d.winner().carbon_to_oxygen),
                    CarbonOxygenRegime::WaterRich,
                    "the winner stays inside the water-rich regime (the flip is never crossed)"
                );
            }
            other => panic!("expected a Decided verdict, got {other:?}"),
        }
    }

    #[test]
    fn a_bistable_query_escalates_never_interpolating_across_the_flip() {
        // C/O = 0.9 sits in the banked bistable zone: the selection escalates (Verdict::Escalate) rather than
        // blending the 0.8 and 1.2 tables into a gas that exists at no composition.
        let reg = registry();
        assert!(
            matches!(
                reg.select(co(7, 10), co(14, 1000), co(9, 10)),
                Verdict::Escalate(_)
            ),
            "a bistable-zone C/O escalates through the Verdict machinery"
        );
    }

    #[test]
    fn a_composition_miss_escalates_never_substituting_the_closure() {
        // A carbon-rich registry with no carbon-rich grid (only water-rich) escalates on a carbon-rich query: the
        // pull-on-miss forbidden-fallback, a loud hold, never a silent closure substitution.
        let water_only = OpacityGridRegistry::new(vec![(
            co(7, 10),
            co(1, 1000),
            co(55, 100),
            toy_grid(co(7, 10), co(1, 1000)),
        )]);
        assert!(
            matches!(
                water_only.select(co(7, 10), co(14, 1000), co(12, 10)),
                Verdict::Escalate(_)
            ),
            "a carbon-rich query with no in-regime grid escalates (pull-on-miss), never substitutes"
        );
    }

    #[test]
    fn the_standard_registry_selects_over_the_vendored_grids() {
        // The real registry over the 11 vendored ÆSOPUS grids: a solar-ish water-rich query resolves to a grid (not
        // a miss), a carbon-rich query (C/O = 1.2) resolves to the carbon-rich ladder grid, and a bistable query
        // (C/O = 0.9) escalates.
        let reg = OpacityGridRegistry::standard().expect("the vendored registry loads");
        assert_eq!(reg.len(), 11, "11 vendored grids");
        let solar = reg.select(co(7, 10), co(165, 10000), co(55, 100));
        assert!(
            matches!(solar, Verdict::Decided(_) | Verdict::Trivial(_)),
            "a solar water-rich query resolves to a grid"
        );
        let carbon = reg.select(co(7, 10), co(1337, 100000), co(12, 10));
        match carbon {
            Verdict::Decided(d) => assert!(
                d.winner().carbon_to_oxygen.to_f64_lossy() > 1.0,
                "a carbon-rich query stays carbon-rich"
            ),
            Verdict::Trivial(t) => assert!(t.winner().carbon_to_oxygen.to_f64_lossy() > 1.0),
            other => panic!("expected a carbon-rich grid, got {other:?}"),
        }
        assert!(
            matches!(
                reg.select(co(7, 10), co(1337, 100000), co(9, 10)),
                Verdict::Escalate(_)
            ),
            "a bistable C/O escalates even over the real registry"
        );
    }

    #[test]
    fn a_lone_in_regime_grid_is_trivial() {
        // One carbon-rich grid: a carbon-rich query resolves to it as Trivial (the single unambiguous member).
        let reg = registry();
        match reg.select(co(7, 10), co(14, 1000), co(12, 10)) {
            Verdict::Trivial(t) => assert_eq!(t.winner().index, 2, "the lone carbon-rich grid"),
            other => panic!("expected Trivial, got {other:?}"),
        }
    }
}
