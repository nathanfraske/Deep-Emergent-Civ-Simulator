//! Typed access to magnitudes carried by the sealed absolute physics floor.
//!
//! Ledger identity and provenance do not authorize a caller to attach a value.
//! This module instead pairs the repository-owned audited catalog with the
//! repository-owned units tables. Its types and constructor remain private to
//! the canonical planet runner, so there is no string-keyed or caller-binding
//! value surface.

use super::preflight::floor_catalog_mismatch;
use civsim_ledger::AbsolutePhysicsFloor;
use civsim_units::constants::{si_execution_magnitudes, ScaledConstant};
use std::{fmt, marker::PhantomData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuditedMagnitude<Kind> {
    symbol: &'static str,
    bits: i128,
    scale_bits: u32,
    kind: PhantomData<Kind>,
}

impl<Kind> AuditedMagnitude<Kind> {
    fn from_constant(
        expected_symbol: &'static str,
        value: ScaledConstant,
    ) -> Result<Self, FloorMagnitudeError> {
        debug_assert_ne!(value.bits(), 0);
        if value.symbol() != expected_symbol {
            return Err(FloorMagnitudeError::IdentityMismatch {
                expected: expected_symbol,
                found: value.symbol(),
            });
        }
        Ok(Self {
            symbol: value.symbol(),
            bits: value.bits(),
            scale_bits: value.scale_bits(),
            kind: PhantomData,
        })
    }

    pub(crate) fn symbol(self) -> &'static str {
        self.symbol
    }

    pub(crate) fn bits(self) -> i128 {
        self.bits
    }

    pub(crate) fn scale_bits(self) -> u32 {
        self.scale_bits
    }
}

macro_rules! magnitude_kind {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) struct $name;
    };
}

magnitude_kind!(SpeedOfLight);
magnitude_kind!(CaesiumHyperfineFrequency);
magnitude_kind!(BoltzmannConstant);
magnitude_kind!(PlanckConstant);
magnitude_kind!(ElementaryCharge);
magnitude_kind!(FineStructureConstant);
magnitude_kind!(VacuumPermittivity);
magnitude_kind!(AvogadroConstant);
magnitude_kind!(LuminousEfficacy);
magnitude_kind!(GravitationalConstant);
magnitude_kind!(ElectronMass);
magnitude_kind!(StefanBoltzmannConstant);
magnitude_kind!(GasConstant);
magnitude_kind!(AtomicVolumeConversion);

fn required_constant(
    constants: &civsim_units::constants::SiExecutionMagnitudes,
    symbol: &'static str,
) -> Result<ScaledConstant, FloorMagnitudeError> {
    constants.get(symbol).ok_or_else(|| {
        FloorMagnitudeError::InvalidMagnitudeTable(format!(
            "sealed SI execution view is missing '{symbol}'"
        ))
    })
}

/// The three measured magnitudes in the audited absolute physics floor.
///
/// Distinct marker types prevent one physical constant from being substituted
/// for another even though all use the same deterministic scaled-integer
/// representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuditedFloorMagnitudes {
    pub(crate) fine_structure: AuditedMagnitude<FineStructureConstant>,
    pub(crate) gravitational_constant: AuditedMagnitude<GravitationalConstant>,
    pub(crate) electron_mass: AuditedMagnitude<ElectronMass>,
}

/// Non-configurable SI execution projection paired with the physical floor.
/// The seven schema roots and four derived values are not floor leaves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuditedSiExecutionMagnitudes {
    sealed: civsim_units::constants::SiExecutionMagnitudes,
    pub(crate) caesium_hyperfine_frequency: AuditedMagnitude<CaesiumHyperfineFrequency>,
    pub(crate) speed_of_light: AuditedMagnitude<SpeedOfLight>,
    pub(crate) boltzmann: AuditedMagnitude<BoltzmannConstant>,
    pub(crate) planck: AuditedMagnitude<PlanckConstant>,
    pub(crate) elementary_charge: AuditedMagnitude<ElementaryCharge>,
    pub(crate) avogadro: AuditedMagnitude<AvogadroConstant>,
    pub(crate) luminous_efficacy: AuditedMagnitude<LuminousEfficacy>,
    pub(crate) fine_structure: AuditedMagnitude<FineStructureConstant>,
    pub(crate) gravitational_constant: AuditedMagnitude<GravitationalConstant>,
    pub(crate) electron_mass: AuditedMagnitude<ElectronMass>,
    pub(crate) vacuum_permittivity: AuditedMagnitude<VacuumPermittivity>,
    pub(crate) stefan_boltzmann: AuditedMagnitude<StefanBoltzmannConstant>,
    pub(crate) gas_constant: AuditedMagnitude<GasConstant>,
    pub(crate) atomic_volume_conversion: AuditedMagnitude<AtomicVolumeConversion>,
}

/// A borrow of the exact repository floor paired with its typed magnitudes.
///
/// Keeping the identity graph and magnitudes in one private view prevents a
/// stage from combining magnitudes from one admitted floor with identities
/// from another.
#[derive(Debug)]
pub(crate) struct AuditedFloorView<'a> {
    _floor: &'a AbsolutePhysicsFloor,
    pub(crate) magnitudes: AuditedFloorMagnitudes,
    pub(crate) execution: AuditedSiExecutionMagnitudes,
}

impl<'a> AuditedFloorView<'a> {
    pub(crate) fn from_floor(floor: &'a AbsolutePhysicsFloor) -> Result<Self, FloorMagnitudeError> {
        let (magnitudes, execution) = AuditedFloorMagnitudes::from_floor(floor)?;
        Ok(Self {
            _floor: floor,
            magnitudes,
            execution,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.magnitudes.len()
    }
}

impl AuditedFloorMagnitudes {
    /// Construct the typed view only when the admitted floor is exactly the
    /// repository-owned audited catalog. No caller magnitude is accepted.
    fn from_floor(
        floor: &AbsolutePhysicsFloor,
    ) -> Result<(Self, AuditedSiExecutionMagnitudes), FloorMagnitudeError> {
        if let Some(detail) = floor_catalog_mismatch(floor) {
            return Err(FloorMagnitudeError::UnauditedFloor(detail));
        }

        // The units crate exposes one sealed, repository-computed table. It has
        // no constructor that accepts caller identities or magnitudes.
        let constants = si_execution_magnitudes(floor)
            .map_err(|error| FloorMagnitudeError::InvalidMagnitudeTable(error.to_string()))?;
        let floor_magnitudes = Self {
            fine_structure: AuditedMagnitude::from_constant(
                "alpha",
                required_constant(&constants, "alpha")?,
            )?,
            gravitational_constant: AuditedMagnitude::from_constant(
                "G",
                required_constant(&constants, "G")?,
            )?,
            electron_mass: AuditedMagnitude::from_constant(
                "m_e",
                required_constant(&constants, "m_e")?,
            )?,
        };
        let execution = AuditedSiExecutionMagnitudes {
            sealed: constants,
            caesium_hyperfine_frequency: AuditedMagnitude::from_constant(
                "Delta_nu_Cs",
                required_constant(&constants, "Delta_nu_Cs")?,
            )?,
            speed_of_light: AuditedMagnitude::from_constant(
                "c",
                required_constant(&constants, "c")?,
            )?,
            boltzmann: AuditedMagnitude::from_constant(
                "k_B",
                required_constant(&constants, "k_B")?,
            )?,
            planck: AuditedMagnitude::from_constant("h", required_constant(&constants, "h")?)?,
            elementary_charge: AuditedMagnitude::from_constant(
                "e",
                required_constant(&constants, "e")?,
            )?,
            avogadro: AuditedMagnitude::from_constant(
                "N_A",
                required_constant(&constants, "N_A")?,
            )?,
            luminous_efficacy: AuditedMagnitude::from_constant(
                "K_cd",
                required_constant(&constants, "K_cd")?,
            )?,
            fine_structure: AuditedMagnitude::from_constant(
                "alpha",
                required_constant(&constants, "alpha")?,
            )?,
            gravitational_constant: AuditedMagnitude::from_constant(
                "G",
                required_constant(&constants, "G")?,
            )?,
            electron_mass: AuditedMagnitude::from_constant(
                "m_e",
                required_constant(&constants, "m_e")?,
            )?,
            vacuum_permittivity: AuditedMagnitude::from_constant(
                "eps_0",
                required_constant(&constants, "eps_0")?,
            )?,
            stefan_boltzmann: AuditedMagnitude::from_constant(
                "sigma",
                required_constant(&constants, "sigma")?,
            )?,
            gas_constant: AuditedMagnitude::from_constant(
                "R",
                required_constant(&constants, "R")?,
            )?,
            atomic_volume_conversion: AuditedMagnitude::from_constant(
                "A3_per_cm3_mol",
                required_constant(&constants, "A3_per_cm3_mol")?,
            )?,
        };
        Ok((floor_magnitudes, execution))
    }

    /// Ensure every current catalog member has a non-zero typed projection.
    pub(crate) fn len(&self) -> usize {
        let representations = [
            (self.fine_structure.bits(), self.fine_structure.scale_bits()),
            (
                self.gravitational_constant.bits(),
                self.gravitational_constant.scale_bits(),
            ),
            (self.electron_mass.bits(), self.electron_mass.scale_bits()),
        ];
        debug_assert!(representations.iter().all(|(bits, _scale_bits)| *bits != 0));
        representations.len()
    }
}

impl AuditedSiExecutionMagnitudes {
    pub(crate) fn physical_invariant_definition(
        &self,
        symbol: &str,
    ) -> Option<&'static civsim_units::fundamentals::Fundamental> {
        self.sealed.physical_invariant_definition(symbol)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FloorMagnitudeError {
    UnauditedFloor(String),
    InvalidMagnitudeTable(String),
    IdentityMismatch {
        expected: &'static str,
        found: &'static str,
    },
}

impl fmt::Display for FloorMagnitudeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnauditedFloor(detail) => write!(f, "unaudited absolute floor: {detail}"),
            Self::InvalidMagnitudeTable(detail) => {
                write!(f, "invalid repository magnitude table: {detail}")
            }
            Self::IdentityMismatch { expected, found } => write!(
                f,
                "repository magnitude identity '{found}' does not match sealed field '{expected}'"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::sealed_absolute_physics_floor;
    use civsim_core::Fixed;
    use civsim_ledger::{
        ChaosProtocolReceipt, DerivationExhaustionReceipt, Entry, GapLawReceipt, Ledger,
        Provenance, ResidualLawReceipt, Tier,
    };

    fn audited_floor() -> AbsolutePhysicsFloor {
        sealed_absolute_physics_floor().expect("the audited physical floor is admissible")
    }

    #[test]
    fn physical_floor_and_si_execution_view_remain_distinct() {
        let (magnitudes, execution) = AuditedFloorMagnitudes::from_floor(&audited_floor())
            .expect("the audited catalog has typed magnitudes");

        assert_eq!(magnitudes.len(), 3);
        let representations = [
            (
                magnitudes.fine_structure.bits(),
                magnitudes.fine_structure.scale_bits(),
            ),
            (
                magnitudes.gravitational_constant.bits(),
                magnitudes.gravitational_constant.scale_bits(),
            ),
            (
                magnitudes.electron_mass.bits(),
                magnitudes.electron_mass.scale_bits(),
            ),
        ];
        assert!(representations.iter().all(|(bits, _)| *bits != 0));
        assert!(execution.planck.scale_bits() > Fixed::FRAC_BITS);
        assert!(magnitudes.electron_mass.scale_bits() > Fixed::FRAC_BITS);
        assert_eq!(
            execution.fine_structure.bits(),
            magnitudes.fine_structure.bits()
        );
        assert_ne!(execution.vacuum_permittivity.bits(), 0);
    }

    #[test]
    fn an_admitted_but_unaudited_floor_cannot_construct_the_view() {
        let receipt = DerivationExhaustionReceipt {
            entry_id: "fundamental.cited_knob".into(),
            phenomenon: "fixture".into(),
            derivation_attempts: vec!["derive-first fixture attempt".into()],
            residual_slot: "fixture.slot".into(),
            buckingham_pi_groups: 1,
            gap_law: GapLawReceipt {
                reference_validity: "fixture evidence".into(),
                gap_dispatch: "fixture evidence".into(),
                smooth_systematics: "fixture evidence".into(),
                scale_free_limit: "fixture evidence".into(),
                chaos_protocol: ChaosProtocolReceipt::NotApplicable {
                    basis: "fixture has no dynamical branch".into(),
                },
            },
            residual_law: ResidualLawReceipt {
                conservation: "fixture evidence".into(),
                disequilibrium: "fixture evidence".into(),
                fluctuation_dissipation: "fixture evidence".into(),
                dimensional_analysis: "fixture evidence".into(),
            },
        };
        let floor = AbsolutePhysicsFloor::admit(
            Ledger::build([Entry {
                id: "fundamental.cited_knob".into(),
                tier: Tier::Universal,
                provenance: Provenance::Measured,
                inputs: vec![],
            }])
            .expect("the fixture ledger is structurally valid"),
            [receipt],
        )
        .expect("the fixture passes structural admission only");

        assert!(matches!(
            AuditedFloorView::from_floor(&floor),
            Err(FloorMagnitudeError::UnauditedFloor(_))
        ));
    }

    #[test]
    fn typed_fields_preserve_the_sealed_units_projections() {
        let (magnitudes, execution) = AuditedFloorMagnitudes::from_floor(&audited_floor())
            .expect("the audited catalog has typed magnitudes");
        let floor = audited_floor();
        let constants = si_execution_magnitudes(&floor).unwrap();

        assert_eq!(
            (
                magnitudes.gravitational_constant.bits(),
                magnitudes.gravitational_constant.scale_bits(),
            ),
            (
                constants.get("G").unwrap().bits(),
                constants.get("G").unwrap().scale_bits(),
            )
        );
        assert_eq!(
            (
                execution.stefan_boltzmann.bits(),
                execution.stefan_boltzmann.scale_bits(),
            ),
            (
                constants.get("sigma").unwrap().bits(),
                constants.get("sigma").unwrap().scale_bits(),
            )
        );
    }

    #[test]
    fn a_magnitude_cannot_be_relabelled_by_parallel_array_order() {
        let floor = audited_floor();
        let constants = si_execution_magnitudes(&floor).unwrap();
        let error = AuditedMagnitude::<BoltzmannConstant>::from_constant(
            "k_B",
            constants.get("c").unwrap(),
        )
        .expect_err("the c magnitude cannot bind to the k_B field");
        assert_eq!(
            error,
            FloorMagnitudeError::IdentityMismatch {
                expected: "k_B",
                found: "c",
            }
        );
    }
}
