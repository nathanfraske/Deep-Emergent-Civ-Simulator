//! The CONDITIONED critical-Rayleigh registry row (owner ruling 2026-07-18).
//!
//! `data/rayleigh_critical_eigenvalues.toml` is the cited registry of marginal-stability eigenvalues, and the
//! critical Rayleigh number a convecting column reads is a CHORD over two conditioning axes, never a preference:
//! `boundary_class` (set at the top by the tectonic regime, mobile or stagnant, and at the bottom by the base
//! state, liquid or solid) and `heating_mode` (bottom-heated Rayleigh-Benard, or internally heated
//! Rayleigh-Roberts). This module reads the registry and dispatches on those axes.
//!
//! DEFAULTS-TAKEN (owner-signed): neither axis is resolvable per column at assembly yet, so the dispatch
//! defaults to the declared rigid-rigid / bottom-heated instance, the value convection onset (`threshold_latch`)
//! and the boundary layer (`thermal_boundary_layer`) already read. That default is the retirement note for two
//! NAMED DEBTS: the top axis keys on the tectonic-regime field the day it resolves (the same seam the
//! stagnant-lid Nusselt branch waits on), and the bottom axis keys on the structure arc's liquid-at-base field.
//! Internal heating's onset is a subcritical bracket (energy vs linear thresholds), which takes a further latch
//! ruling; until then the bottom-heated instance is what the engine reads.

use crate::convection_scaling::BoundaryCondition;
use civsim_core::Fixed;

/// Which heating drives the layer: classical bottom-heated onset, or internally heated (Rayleigh-Roberts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeatingMode {
    /// Bottom-heated Rayleigh-Benard: the classical marginal-stability onset (a single Ra_c).
    BottomHeated,
    /// Internally heated Rayleigh-Roberts: onset is a subcritical bracket, and this reads its linear edge.
    Internal,
}

impl HeatingMode {
    fn tag(self) -> &'static str {
        match self {
            HeatingMode::BottomHeated => "bottom_heated",
            HeatingMode::Internal => "internal",
        }
    }
}

fn boundary_tag(bc: BoundaryCondition) -> &'static str {
    match bc {
        BoundaryCondition::FreeFree => "free-free",
        BoundaryCondition::RigidRigid => "rigid-rigid",
        BoundaryCondition::RigidFree => "rigid-free",
    }
}

struct EigenRow {
    heating_mode: String,
    boundary_class: String,
    threshold_type: String,
    rayleigh_number: Fixed,
    critical_wavenumber: Fixed,
}

/// The cited eigenvalue registry, read from `data/rayleigh_critical_eigenvalues.toml`.
pub struct RayleighCriticalRegistry {
    rows: Vec<EigenRow>,
}

impl RayleighCriticalRegistry {
    /// Load the vendored registry.
    pub fn standard() -> Result<Self, String> {
        Self::from_toml_str(include_str!("../data/rayleigh_critical_eigenvalues.toml"))
    }

    /// Parse the `[[eigenvalue]]` blocks, a focused reader over the quoted string fields, the values through
    /// `Fixed::from_decimal_str` so the cited precision survives (the same idiom the sibling scaling reader uses).
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        for block in s.split("[[eigenvalue]]").skip(1) {
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
            let (Some(heating_mode), Some(boundary_class)) =
                (field("heating_mode"), field("boundary_class"))
            else {
                continue;
            };
            let Some(ra_s) = field("rayleigh_number") else {
                continue;
            };
            // The Rayleigh-Roberts definition row carries a symbolic form, not a number; skip any row whose Ra_c
            // does not parse (the definition, or a future prose row) rather than failing the whole registry.
            let Ok(rayleigh_number) = Fixed::from_decimal_str(&ra_s) else {
                continue;
            };
            let critical_wavenumber = field("critical_wavenumber")
                .and_then(|w| Fixed::from_decimal_str(&w).ok())
                .unwrap_or(Fixed::ZERO);
            rows.push(EigenRow {
                heating_mode,
                boundary_class,
                threshold_type: field("threshold_type").unwrap_or_default(),
                rayleigh_number,
                critical_wavenumber,
            });
        }
        if rows.is_empty() {
            return Err("no eigenvalue rows parsed from the registry".to_string());
        }
        Ok(Self { rows })
    }

    fn row(&self, bc: BoundaryCondition, heating: HeatingMode) -> Option<&EigenRow> {
        self.rows.iter().find(|r| {
            r.boundary_class == boundary_tag(bc)
                && r.heating_mode == heating.tag()
                && match heating {
                    // Bottom-heated onset is marginal stability by definition; a row that omits the threshold is
                    // the same single onset.
                    HeatingMode::BottomHeated => {
                        r.threshold_type == "marginal_stability" || r.threshold_type.is_empty()
                    }
                    // Internal heating's convecting onset is the linear-instability edge of its bracket (the
                    // energy edge is the conducting-guaranteed floor below it).
                    HeatingMode::Internal => r.threshold_type == "linear_instability",
                }
        })
    }

    /// The critical Rayleigh number at which convection onsets for a boundary class and heating mode.
    pub fn critical_rayleigh(&self, bc: BoundaryCondition, heating: HeatingMode) -> Option<Fixed> {
        self.row(bc, heating).map(|r| r.rayleigh_number)
    }

    /// The critical wavenumber (the cell aspect the planform machinery reads) for the same row.
    pub fn critical_wavenumber(
        &self,
        bc: BoundaryCondition,
        heating: HeatingMode,
    ) -> Option<Fixed> {
        self.row(bc, heating).map(|r| r.critical_wavenumber)
    }
}

/// The conditioned critical Rayleigh a column reads. STUBBED at the declared rigid-rigid default until the
/// tectonic-regime and base-state resolvers land (the two named debts); returns the selected boundary class and
/// its Ra_c.
pub fn conditioned_ra_crit(
    heating: HeatingMode,
    registry: &RayleighCriticalRegistry,
) -> Option<(BoundaryCondition, Fixed)> {
    // DEFAULTS-TAKEN: rigid-rigid until the axes resolve. When a mobile-or-stagnant tectonic-regime field and a
    // liquid-or-solid base-state field land, the boundary class keys on them within the cited family; the change
    // is one match arm here, no signature move.
    let bc = BoundaryCondition::RigidRigid;
    registry.critical_rayleigh(bc, heating).map(|ra| (bc, ra))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_reads_every_bottom_heated_boundary_class() {
        let reg = RayleighCriticalRegistry::standard().expect("registry loads");
        for bc in [
            BoundaryCondition::FreeFree,
            BoundaryCondition::RigidRigid,
            BoundaryCondition::RigidFree,
        ] {
            assert!(
                reg.critical_rayleigh(bc, HeatingMode::BottomHeated)
                    .is_some(),
                "the bottom-heated {bc:?} eigenvalue is present"
            );
            assert!(reg
                .critical_wavenumber(bc, HeatingMode::BottomHeated)
                .is_some());
        }
    }

    #[test]
    fn the_free_free_eigenvalue_is_the_closed_form() {
        // A sanity anchor: free-free Ra_c = 27 pi^4 / 4 = 657.511.
        let reg = RayleighCriticalRegistry::standard().expect("registry loads");
        let ra = reg
            .critical_rayleigh(BoundaryCondition::FreeFree, HeatingMode::BottomHeated)
            .expect("free-free present");
        assert!(
            (ra - Fixed::from_ratio(657_511, 1000)).abs() < Fixed::from_ratio(1, 100),
            "free-free Ra_c is 657.511, got {}",
            ra.to_f64_lossy()
        );
    }

    #[test]
    fn the_conditioned_default_is_rigid_rigid_and_equals_the_onset() {
        // DEFAULTS-TAKEN: the conditioned row defaults to rigid-rigid / bottom-heated, the same eigenvalue the
        // convection onset and the boundary layer read, so the conditioned row and the onset latch cannot
        // disagree. Pinned to the convection_scaling rigid-rigid row (the diamond fix's one cited value).
        let reg = RayleighCriticalRegistry::standard().expect("registry loads");
        let (bc, ra) =
            conditioned_ra_crit(HeatingMode::BottomHeated, &reg).expect("default resolves");
        assert_eq!(bc, BoundaryCondition::RigidRigid);
        let onset = crate::convection_scaling::ConvectionScaling::standard()
            .expect("convection_scaling vendored")
            .critical_rayleigh(BoundaryCondition::RigidRigid)
            .expect("rigid-rigid onset row");
        assert!(
            (ra - onset).abs() < Fixed::from_ratio(1, 100),
            "the conditioned default {} equals the onset {}",
            ra.to_f64_lossy(),
            onset.to_f64_lossy()
        );
    }
}
