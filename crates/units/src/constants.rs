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

//! Canonical SI execution projection over representation definitions and the
//! admitted physical invariant coordinates.

use crate::bignum::BigRat;
use crate::fundamentals::{
    Composite, Fundamental, ATOMIC_VOLUME_CONVERSION, AVOGADRO, BOLTZMANN,
    CAESIUM_HYPERFINE_FREQUENCY, ELECTRON_MASS, ELEMENTARY_CHARGE, FINE_STRUCTURE, GAS_CONSTANT,
    GRAVITATIONAL_CONSTANT, LUMINOUS_EFFICACY, PLANCK, SPEED_OF_LIGHT, STEFAN_BOLTZMANN,
    VACUUM_PERMITTIVITY,
};
use crate::physics_floor::{sealed_absolute_physics_floor, verify_absolute_physics_floor};
use civsim_core::Fixed;
use civsim_ledger::AbsolutePhysicsFloor;
use std::cell::RefCell;
use std::fmt;
use std::sync::OnceLock;

// Repository representation policy, never a world input. The typed universal
// view keeps at least this many binary digits while preserving Q32.32 whenever
// the magnitude fits in an i128 at that scale.
const FLOOR_SIGNIFICAND_BITS: i64 = 30;
const MAX_I128_MAGNITUDE_LOG2: i64 = 126;

/// One repository-computed constant magnitude. Its fields are sealed so a
/// caller cannot construct or bind an arbitrary value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaledConstant {
    symbol: &'static str,
    bits: i128,
    scale_bits: u32,
}

impl ScaledConstant {
    /// Stable physical symbol bound to this magnitude.
    pub const fn symbol(self) -> &'static str {
        self.symbol
    }

    /// Signed integer mantissa.
    pub const fn bits(self) -> i128 {
        self.bits
    }

    /// Number of fractional binary places in the mantissa.
    pub const fn scale_bits(self) -> u32 {
        self.scale_bits
    }

    /// Exact rational represented by the published scaled integer.
    pub fn exact_rational(self) -> BigRat {
        BigRat::from_scaled_i128(self.bits, self.scale_bits)
    }
}

/// The noncausal SI representation view.
///
/// Seven exact SI definitions encode units. Three values are derived only from
/// those definitions. None is a causal physical degree of freedom, so this view
/// can be constructed without an admitted physical floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiRepresentationMagnitudes {
    caesium_hyperfine_frequency: ScaledConstant,
    speed_of_light: ScaledConstant,
    boltzmann: ScaledConstant,
    planck: ScaledConstant,
    elementary_charge: ScaledConstant,
    avogadro: ScaledConstant,
    luminous_efficacy: ScaledConstant,
    stefan_boltzmann: ScaledConstant,
    gas_constant: ScaledConstant,
    atomic_volume_conversion: ScaledConstant,
    seal: RepresentationSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepresentationSeal;

impl SiRepresentationMagnitudes {
    pub const fn len(&self) -> usize {
        10
    }

    pub const fn is_empty(&self) -> bool {
        false
    }

    pub fn get(&self, symbol: &str) -> Option<ScaledConstant> {
        [
            self.caesium_hyperfine_frequency,
            self.speed_of_light,
            self.boltzmann,
            self.planck,
            self.elementary_charge,
            self.avogadro,
            self.luminous_efficacy,
            self.stefan_boltzmann,
            self.gas_constant,
            self.atomic_volume_conversion,
        ]
        .into_iter()
        .find(|value| value.symbol() == symbol)
    }

    /// Exact SI-coordinate conversion `N_A * 10^-21` used to express an
    /// atomic number density per cubic nanometre.
    pub fn avogadro_per_nm3_fold(&self) -> Option<Fixed> {
        let value = self
            .avogadro
            .exact_rational()
            .mul(&decimal_rational("1e-21")?);
        fixed_from_rational(&value)
    }

    /// `hbar * 10^15 / (2*pi*k_B)` in femtosecond-kelvin units.
    pub fn scattering_time_fold_fs_k(&self) -> Option<Fixed> {
        let two_pi = BigRat::from_i64(2).mul(&crate::compute::pi(80));
        let value = self
            .planck
            .exact_rational()
            .div(&two_pi)
            .mul(&decimal_rational("1e15")?)
            .div(&two_pi)
            .div(&self.boltzmann.exact_rational());
        fixed_from_rational(&value)
    }

    /// Exact representation conversion from eV/angstrom^3 to GPa.
    pub fn gpa_per_ev_per_angstrom_cubed(&self) -> Option<Fixed> {
        let value = self
            .elementary_charge
            .exact_rational()
            .mul(&decimal_rational("1e21")?);
        fixed_from_rational(&value)
    }

    /// Exact representation conversion from eV per particle to kJ/mol.
    pub fn ev_to_kj_per_mol(&self) -> Option<Fixed> {
        let value = self
            .elementary_charge
            .exact_rational()
            .mul(&self.avogadro.exact_rational())
            .mul(&decimal_rational("1e-3")?);
        fixed_from_rational(&value)
    }
}

/// The complete SI execution capability.
///
/// This type has no public value-binding constructor. Its sole producer first
/// verifies an [`AbsolutePhysicsFloor`] against the independent ordered seal.
/// Seven representation definitions and their three representation-only
/// relations then join three admitted physical coordinates and `eps_0`, which
/// is derived from those coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiExecutionMagnitudes {
    caesium_hyperfine_frequency: ScaledConstant,
    speed_of_light: ScaledConstant,
    boltzmann: ScaledConstant,
    planck: ScaledConstant,
    elementary_charge: ScaledConstant,
    avogadro: ScaledConstant,
    luminous_efficacy: ScaledConstant,
    fine_structure: ScaledConstant,
    gravitational_constant: ScaledConstant,
    electron_mass: ScaledConstant,
    vacuum_permittivity: ScaledConstant,
    stefan_boltzmann: ScaledConstant,
    gas_constant: ScaledConstant,
    atomic_volume_conversion: ScaledConstant,
    seal: ExecutionSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExecutionSeal;

impl SiExecutionMagnitudes {
    /// Number of fixed identities in this execution view.
    pub const fn len(&self) -> usize {
        14
    }

    /// The closed table is never empty.
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Resolve a stable symbol without exposing a value-binding constructor.
    pub fn get(&self, symbol: &str) -> Option<ScaledConstant> {
        [
            self.caesium_hyperfine_frequency,
            self.speed_of_light,
            self.boltzmann,
            self.planck,
            self.elementary_charge,
            self.avogadro,
            self.luminous_efficacy,
            self.fine_structure,
            self.gravitational_constant,
            self.electron_mass,
            self.vacuum_permittivity,
            self.stefan_boltzmann,
            self.gas_constant,
            self.atomic_volume_conversion,
        ]
        .into_iter()
        .find(|value| value.symbol() == symbol)
    }

    /// Representation-only subset carried by this verified execution view.
    pub const fn representation(&self) -> SiRepresentationMagnitudes {
        SiRepresentationMagnitudes {
            caesium_hyperfine_frequency: self.caesium_hyperfine_frequency,
            speed_of_light: self.speed_of_light,
            boltzmann: self.boltzmann,
            planck: self.planck,
            elementary_charge: self.elementary_charge,
            avogadro: self.avogadro,
            luminous_efficacy: self.luminous_efficacy,
            stefan_boltzmann: self.stefan_boltzmann,
            gas_constant: self.gas_constant,
            atomic_volume_conversion: self.atomic_volume_conversion,
            seal: RepresentationSeal,
        }
    }

    /// Exact source decimal for an admitted root, gated by this capability.
    pub fn source_decimal(&self, symbol: &str) -> Option<&'static str> {
        self.get(symbol)?;
        crate::fundamentals::execution_root(symbol).map(|root| root.value)
    }

    /// Source and uncertainty metadata for an admitted physical coordinate.
    pub fn physical_invariant_definition(&self, symbol: &str) -> Option<&'static Fundamental> {
        self.get(symbol)?;
        crate::fundamentals::execution_root(symbol)
            .filter(|root| root.role == crate::fundamentals::FundamentalRole::PhysicalInvariant)
    }

    /// `hbar * 10^15 / (2*pi*k_B)` in femtosecond-kelvin units.
    pub fn scattering_time_fold_fs_k(&self) -> Option<Fixed> {
        self.representation().scattering_time_fold_fs_k()
    }

    /// `e^2 * 10^12 / m_e`, mapping `/nm^3 * fs` to Drude S/m.
    pub fn drude_conductivity_fold(&self) -> Option<Fixed> {
        let e = self.elementary_charge.exact_rational();
        let m_e = self.electron_mass.exact_rational();
        let value = e.mul(&e).mul(&decimal_rational("1e12")?).div(&m_e);
        fixed_from_rational(&value)
    }

    /// `hbar * sqrt(10^27 / (eps_0*m_e))` in eV nm^(3/2).
    ///
    /// The elementary charge cancels between the plasma frequency and the
    /// joule-to-electron-volt conversion. The squared fold is formed exactly
    /// before its one fixed-point square root, avoiding tiny intermediates.
    pub fn plasma_energy_fold_ev_nm_three_halves(&self) -> Option<Fixed> {
        let h = self.planck.exact_rational();
        let hbar = h.div(&BigRat::from_i64(2).mul(&crate::compute::pi(80)));
        let squared = hbar.mul(&hbar).mul(&decimal_rational("1e27")?).div(
            &self
                .vacuum_permittivity
                .exact_rational()
                .mul(&self.electron_mass.exact_rational()),
        );
        let squared = fixed_from_rational(&squared)?;
        (squared > Fixed::ZERO).then(|| squared.sqrt())
    }

    /// `e^2 / (4*pi*eps_0)` at one angstrom, expressed in eV angstrom.
    pub fn coulomb_energy_ev_angstrom(&self) -> Option<Fixed> {
        let denominator = BigRat::from_i64(4)
            .mul(&crate::compute::pi(80))
            .mul(&self.vacuum_permittivity.exact_rational());
        let value = self
            .elementary_charge
            .exact_rational()
            .div(&denominator)
            .mul(&decimal_rational("1e10")?);
        fixed_from_rational(&value)
    }

    /// Bohr radius `4*pi*eps_0*hbar^2/(m_e*e^2)` in angstroms.
    pub fn bohr_radius_angstrom(&self) -> Option<Fixed> {
        let e = self.elementary_charge.exact_rational();
        let hbar = self
            .planck
            .exact_rational()
            .div(&BigRat::from_i64(2).mul(&crate::compute::pi(80)));
        let numerator = BigRat::from_i64(4)
            .mul(&crate::compute::pi(80))
            .mul(&self.vacuum_permittivity.exact_rational())
            .mul(&hbar)
            .mul(&hbar)
            .mul(&decimal_rational("1e10")?);
        let denominator = self.electron_mass.exact_rational().mul(&e).mul(&e);
        fixed_from_rational(&numerator.div(&denominator))
    }

    /// Harrison's `hbar^2/m_e` prefactor in eV angstrom squared.
    pub fn harrison_prefactor_ev_angstrom2(&self) -> Option<Fixed> {
        let hbar = self
            .planck
            .exact_rational()
            .div(&BigRat::from_i64(2).mul(&crate::compute::pi(80)));
        let denominator = self
            .electron_mass
            .exact_rational()
            .mul(&self.elementary_charge.exact_rational());
        let value = hbar
            .mul(&hbar)
            .mul(&decimal_rational("1e20")?)
            .div(&denominator);
        fixed_from_rational(&value)
    }

    /// Exact representation conversion from eV/angstrom^3 to GPa.
    pub fn gpa_per_ev_per_angstrom_cubed(&self) -> Option<Fixed> {
        self.representation().gpa_per_ev_per_angstrom_cubed()
    }

    /// Exact representation conversion from eV per particle to kJ/mol.
    pub fn ev_to_kj_per_mol(&self) -> Option<Fixed> {
        self.representation().ev_to_kj_per_mol()
    }
}

fn decimal_rational(value: &str) -> Option<BigRat> {
    BigRat::from_decimal_str(value).ok()
}

fn fixed_from_rational(value: &BigRat) -> Option<Fixed> {
    let bits = value.round_to_scale(Fixed::FRAC_BITS)?;
    Some(Fixed::from_bits(i64::try_from(bits).ok()?))
}

/// Compute the noncausal SI representation view.
///
/// Root decimal encodings are parsed only inside the units quarantine. Derived
/// representation magnitudes are evaluated from named exact definitions.
pub fn si_representation_magnitudes() -> Result<SiRepresentationMagnitudes, ConstantProjectionError>
{
    let caesium_hyperfine_frequency = project_fundamental(&CAESIUM_HYPERFINE_FREQUENCY)?;
    let speed_of_light = project_fundamental(&SPEED_OF_LIGHT)?;
    let boltzmann = project_fundamental(&BOLTZMANN)?;
    let planck = project_fundamental(&PLANCK)?;
    let elementary_charge = project_fundamental(&ELEMENTARY_CHARGE)?;
    let avogadro = project_fundamental(&AVOGADRO)?;
    let luminous_efficacy = project_fundamental(&LUMINOUS_EFFICACY)?;
    let definitions = [
        caesium_hyperfine_frequency,
        speed_of_light,
        boltzmann,
        planck,
        elementary_charge,
        avogadro,
        luminous_efficacy,
    ];

    Ok(SiRepresentationMagnitudes {
        caesium_hyperfine_frequency,
        speed_of_light,
        boltzmann,
        planck,
        elementary_charge,
        avogadro,
        luminous_efficacy,
        stefan_boltzmann: project_composite(&STEFAN_BOLTZMANN, &definitions)?,
        gas_constant: project_composite(&GAS_CONSTANT, &definitions)?,
        atomic_volume_conversion: project_composite(&ATOMIC_VOLUME_CONVERSION, &definitions)?,
        seal: RepresentationSeal,
    })
}

/// Project physical magnitudes only after the supplied floor passes the
/// independent ordered identity and receipt seal.
pub fn si_execution_magnitudes(
    floor: &AbsolutePhysicsFloor,
) -> Result<SiExecutionMagnitudes, ConstantProjectionError> {
    verify_absolute_physics_floor(floor)
        .map_err(|error| ConstantProjectionError::UnauditedFloor(error.to_string()))?;
    let representation = si_representation_magnitudes()?;
    let fine_structure = project_fundamental(&FINE_STRUCTURE)?;
    let gravitational_constant = project_fundamental(&GRAVITATIONAL_CONSTANT)?;
    let electron_mass = project_fundamental(&ELECTRON_MASS)?;
    let fundamentals = [
        representation.caesium_hyperfine_frequency,
        representation.speed_of_light,
        representation.boltzmann,
        representation.planck,
        representation.elementary_charge,
        representation.avogadro,
        representation.luminous_efficacy,
        fine_structure,
        gravitational_constant,
        electron_mass,
    ];

    Ok(SiExecutionMagnitudes {
        caesium_hyperfine_frequency: representation.caesium_hyperfine_frequency,
        speed_of_light: representation.speed_of_light,
        boltzmann: representation.boltzmann,
        planck: representation.planck,
        elementary_charge: representation.elementary_charge,
        avogadro: representation.avogadro,
        luminous_efficacy: representation.luminous_efficacy,
        fine_structure,
        gravitational_constant,
        electron_mass,
        vacuum_permittivity: project_composite(&VACUUM_PERMITTIVITY, &fundamentals)?,
        stefan_boltzmann: representation.stefan_boltzmann,
        gas_constant: representation.gas_constant,
        atomic_volume_conversion: representation.atomic_volume_conversion,
        seal: ExecutionSeal,
    })
}

/// Construct the canonical execution capability through the sealed floor.
/// This convenience path accepts no identity or magnitude from its caller.
pub fn canonical_si_execution_magnitudes() -> Result<SiExecutionMagnitudes, ConstantProjectionError>
{
    let floor = sealed_absolute_physics_floor()
        .map_err(|error| ConstantProjectionError::UnauditedFloor(error.to_string()))?;
    si_execution_magnitudes(&floor)
}

fn representation_scale_at_least(
    value: &BigRat,
    minimum_scale: i64,
) -> Result<u32, ConstantProjectionError> {
    let magnitude_log2 = value.floor_log2();
    let significant_scale = FLOOR_SIGNIFICAND_BITS.saturating_sub(magnitude_log2);
    let scale = i64::from(Fixed::FRAC_BITS)
        .max(significant_scale)
        .max(minimum_scale)
        .max(0);
    let maximum_scale = MAX_I128_MAGNITUDE_LOG2.saturating_sub(magnitude_log2);
    if scale > maximum_scale {
        return Err(ConstantProjectionError::RepresentationOverflow {
            magnitude_log2,
            scale,
        });
    }
    u32::try_from(scale).map_err(|_| ConstantProjectionError::RepresentationOverflow {
        magnitude_log2,
        scale,
    })
}

fn representation_scale(value: &BigRat) -> Result<u32, ConstantProjectionError> {
    representation_scale_at_least(value, 0)
}

fn project_fundamental(constant: &Fundamental) -> Result<ScaledConstant, ConstantProjectionError> {
    let value = BigRat::from_decimal_str(constant.value).map_err(|detail| {
        ConstantProjectionError::InvalidMagnitude {
            symbol: constant.symbol,
            detail,
        }
    })?;
    // Preserve at least the decimal source's last printed place. This derives
    // extra representation bits from the source coordinate itself, so a small
    // root such as m_e is never coarsened beyond its published precision merely
    // because the generic significand floor was lower.
    let source_ulp = BigRat::decimal_ulp(constant.value).map_err(|detail| {
        ConstantProjectionError::InvalidMagnitude {
            symbol: constant.symbol,
            detail,
        }
    })?;
    let source_scale = 0_i64.max(-source_ulp.floor_log2());
    let scale_bits = representation_scale_at_least(&value, source_scale)?;
    let bits =
        value
            .round_to_scale(scale_bits)
            .ok_or(ConstantProjectionError::ProjectionOverflow {
                symbol: constant.symbol,
                scale_bits,
            })?;
    projected(constant.symbol, bits, scale_bits)
}

fn project_composite(
    constant: &Composite,
    fundamentals: &[ScaledConstant],
) -> Result<ScaledConstant, ConstantProjectionError> {
    validate_composite_contract(constant)?;
    // Solve representation scale and transcendental working precision from the
    // derived value itself. The stored reference decimal is deliberately absent
    // from this path: it remains an off-path drift oracle, never a value input.
    let mut magnitude_log2 = 0_i64;
    let mut scale_bits = Fixed::FRAC_BITS;
    for _ in 0..4 {
        let working_digits = crate::compute::working_digits_for_scale(scale_bits, magnitude_log2);
        let value = evaluate_projected_composite(constant, fundamentals, working_digits)?;
        let next_magnitude_log2 = value.floor_log2();
        let next_scale_bits = representation_scale(&value)?;
        if next_magnitude_log2 == magnitude_log2 && next_scale_bits == scale_bits {
            let bits = value.round_to_scale(scale_bits).ok_or(
                ConstantProjectionError::ProjectionOverflow {
                    symbol: constant.symbol,
                    scale_bits,
                },
            )?;

            // A second, finer transcendental evaluation must produce the same
            // published bits. This is a validator only; it cannot choose a value.
            let verification_digits = working_digits.saturating_add(20);
            let verification =
                evaluate_projected_composite(constant, fundamentals, verification_digits)?;
            let verification_bits = verification.round_to_scale(scale_bits).ok_or(
                ConstantProjectionError::ProjectionOverflow {
                    symbol: constant.symbol,
                    scale_bits,
                },
            )?;
            if verification_bits != bits {
                return Err(ConstantProjectionError::UnstableProjection {
                    symbol: constant.symbol,
                    scale_bits,
                    working_digits,
                    verification_digits,
                });
            }
            return projected(constant.symbol, bits, scale_bits);
        }
        magnitude_log2 = next_magnitude_log2;
        scale_bits = next_scale_bits;
    }

    Err(ConstantProjectionError::RepresentationDidNotConverge {
        symbol: constant.symbol,
    })
}

fn validate_composite_contract(constant: &Composite) -> Result<(), ConstantProjectionError> {
    let reads = RefCell::new(Vec::<String>::new());
    let resolve = |name: &str| -> Result<crate::fundamentals::SiDimension, String> {
        if name == "pi" {
            return Ok(crate::fundamentals::SiDimension::DIMENSIONLESS);
        }
        let candidate = crate::fundamentals::execution_root(name).ok_or_else(|| {
            format!(
                "composite '{}' names unknown dimension symbol '{name}'",
                constant.symbol
            )
        })?;
        let mut reads = reads.borrow_mut();
        if !reads.iter().any(|seen| seen == name) {
            reads.push(name.to_owned());
        }
        Ok(candidate.dimension)
    };
    let derived = crate::compute::evaluate_formula_dimension(constant.formula, &resolve).map_err(
        |detail| ConstantProjectionError::InvalidMagnitude {
            symbol: constant.symbol,
            detail,
        },
    )?;
    if derived != constant.dimension {
        return Err(ConstantProjectionError::InvalidMagnitude {
            symbol: constant.symbol,
            detail: format!(
                "formula dimension {:?} does not match declared output dimension {:?}",
                derived.exponents(),
                constant.dimension.exponents()
            ),
        });
    }
    let declared: Vec<_> = constant
        .fundamentals
        .iter()
        .map(|symbol| (*symbol).to_owned())
        .collect();
    if *reads.borrow() != declared {
        return Err(ConstantProjectionError::InvalidMagnitude {
            symbol: constant.symbol,
            detail: format!(
                "formula reads {:?} but declares ancestry {:?}",
                reads.borrow().as_slice(),
                constant.fundamentals
            ),
        });
    }
    Ok(())
}

fn evaluate_projected_composite(
    constant: &Composite,
    fundamentals: &[ScaledConstant],
    working_digits: u32,
) -> Result<BigRat, ConstantProjectionError> {
    let resolve = |name: &str| -> Result<BigRat, String> {
        if name == "pi" {
            return Ok(crate::compute::pi(working_digits));
        }
        if !constant.fundamentals.contains(&name) {
            return Err(format!(
                "composite '{}' reads undeclared floor symbol '{name}'",
                constant.symbol
            ));
        }
        fundamentals
            .iter()
            .find(|value| value.symbol == name)
            .copied()
            .map(ScaledConstant::exact_rational)
            .ok_or_else(|| {
                format!(
                    "composite '{}' names missing projected floor symbol '{name}'",
                    constant.symbol
                )
            })
    };
    crate::compute::evaluate_formula(constant.formula, &resolve).map_err(|detail| {
        ConstantProjectionError::InvalidMagnitude {
            symbol: constant.symbol,
            detail,
        }
    })
}

fn projected(
    symbol: &'static str,
    bits: i128,
    scale_bits: u32,
) -> Result<ScaledConstant, ConstantProjectionError> {
    if bits == 0 {
        return Err(ConstantProjectionError::ZeroAfterProjection { symbol });
    }
    Ok(ScaledConstant {
        symbol,
        bits,
        scale_bits,
    })
}

/// Why the fixed repository constant table could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantProjectionError {
    UnauditedFloor(String),
    InvalidMagnitude {
        symbol: &'static str,
        detail: String,
    },
    ProjectionOverflow {
        symbol: &'static str,
        scale_bits: u32,
    },
    RepresentationOverflow {
        magnitude_log2: i64,
        scale: i64,
    },
    ZeroAfterProjection {
        symbol: &'static str,
    },
    UnstableProjection {
        symbol: &'static str,
        scale_bits: u32,
        working_digits: u32,
        verification_digits: u32,
    },
    RepresentationDidNotConverge {
        symbol: &'static str,
    },
}

impl fmt::Display for ConstantProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnauditedFloor(detail) => {
                write!(f, "physical magnitudes require the sealed absolute floor: {detail}")
            }
            Self::InvalidMagnitude { symbol, detail } => {
                write!(f, "invalid magnitude for {symbol}: {detail}")
            }
            Self::ProjectionOverflow { symbol, scale_bits } => write!(
                f,
                "magnitude {symbol} does not fit its derived scale {scale_bits}"
            ),
            Self::RepresentationOverflow {
                magnitude_log2,
                scale,
            } => write!(
                f,
                "magnitude at log2 {magnitude_log2} cannot fit derived scale {scale}"
            ),
            Self::ZeroAfterProjection { symbol } => {
                write!(f, "audited magnitude {symbol} projected to zero")
            }
            Self::UnstableProjection {
                symbol,
                scale_bits,
                working_digits,
                verification_digits,
            } => write!(
                f,
                "magnitude {symbol} is not stable at scale {scale_bits} between {working_digits} and {verification_digits} working digits"
            ),
            Self::RepresentationDidNotConverge { symbol } => write!(
                f,
                "magnitude {symbol} did not converge on a derived representation scale"
            ),
        }
    }
}

impl std::error::Error for ConstantProjectionError {}

/// Stefan-Boltzmann sigma derived from the fundamental constants and projected
/// once to the canonical fixed-point scale.
pub fn derived_stefan_boltzmann() -> Fixed {
    static SIGMA: OnceLock<Fixed> = OnceLock::new();
    *SIGMA.get_or_init(|| {
        let (bits, scale) = derived_stefan_boltzmann_fine();
        let q32 = crate::rescale_bits(bits, scale, Fixed::FRAC_BITS)
            .expect("sigma rescale to Q32.32 must not overflow");
        Fixed::from_bits(q32)
    })
}

/// Stefan-Boltzmann sigma at the full scale selected by the deterministic
/// composite evaluator, before projection to Q32.32.
pub fn derived_stefan_boltzmann_fine() -> (i64, u32) {
    static SIGMA_FINE: OnceLock<(i64, u32)> = OnceLock::new();
    *SIGMA_FINE.get_or_init(|| {
        let value = si_representation_magnitudes()
            .expect("the SI representation view must derive")
            .stefan_boltzmann;
        (
            i64::try_from(value.bits()).expect("sigma at its derived scale fits i64"),
            value.scale_bits(),
        )
    })
}

/// Cubic angstroms per atom for one cubic centimetre per mole, derived as
/// `10^24 / N_A` and rounded once to the canonical fixed-point scale.
pub fn derived_atomic_volume_conversion() -> Fixed {
    static CONVERSION: OnceLock<Fixed> = OnceLock::new();
    *CONVERSION.get_or_init(|| {
        let value = si_representation_magnitudes()
            .expect("the SI representation view must derive")
            .atomic_volume_conversion;
        let bits = crate::rescale_bits(
            i64::try_from(value.bits()).expect("the atomic-volume conversion projection fits i64"),
            value.scale_bits(),
            Fixed::FRAC_BITS,
        )
        .expect("the atomic-volume conversion rescales to Q32.32");
        Fixed::from_bits(bits)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fundamentals::COMPOSITES;

    fn close(value: Fixed, expected: f64, tolerance: f64) -> bool {
        (value.to_f64_lossy() - expected).abs() <= tolerance
    }

    #[test]
    fn canonical_projection_retains_the_existing_bits() {
        assert_eq!(derived_stefan_boltzmann(), Fixed::from_bits(244));
        let (bits, scale) = derived_stefan_boltzmann_fine();
        assert!(bits > 0);
        assert!(scale > Fixed::FRAC_BITS);
    }

    #[test]
    fn atomic_volume_conversion_is_computed_not_authored() {
        assert_eq!(
            derived_atomic_volume_conversion(),
            Fixed::from_decimal_str("1.66053906717").expect("legacy comparison parses")
        );
    }

    #[test]
    fn the_closed_si_execution_view_projects_all_fourteen_magnitudes() {
        let magnitudes = canonical_si_execution_magnitudes().unwrap();
        assert_eq!(magnitudes.len(), 14);
        let values = [
            magnitudes.caesium_hyperfine_frequency,
            magnitudes.speed_of_light,
            magnitudes.boltzmann,
            magnitudes.planck,
            magnitudes.elementary_charge,
            magnitudes.avogadro,
            magnitudes.luminous_efficacy,
            magnitudes.fine_structure,
            magnitudes.gravitational_constant,
            magnitudes.electron_mass,
            magnitudes.vacuum_permittivity,
            magnitudes.stefan_boltzmann,
            magnitudes.gas_constant,
            magnitudes.atomic_volume_conversion,
        ];
        assert!(values.into_iter().all(|value| value.bits() != 0));
        assert!(magnitudes.planck.scale_bits() > Fixed::FRAC_BITS);
        assert!(magnitudes.electron_mass.scale_bits() > Fixed::FRAC_BITS);
    }

    #[test]
    fn physical_working_unit_folds_derive_only_from_the_sealed_execution_view() {
        let execution = canonical_si_execution_magnitudes().unwrap();
        let representation = execution.representation();

        assert!(close(
            representation.avogadro_per_nm3_fold().unwrap(),
            602.214_076,
            1e-6
        ));
        assert!(close(
            execution.scattering_time_fold_fs_k().unwrap(),
            1_215.7,
            0.2
        ));
        assert!(close(
            execution.drude_conductivity_fold().unwrap(),
            28_179.4,
            0.2
        ));
        assert!(close(
            execution.plasma_energy_fold_ev_nm_three_halves().unwrap(),
            1.174,
            0.002
        ));
        assert!(close(
            execution.coulomb_energy_ev_angstrom().unwrap(),
            14.399_6,
            0.001
        ));
        assert!(close(
            execution.bohr_radius_angstrom().unwrap(),
            0.529_177,
            1e-6
        ));
        assert!(close(
            execution.harrison_prefactor_ev_angstrom2().unwrap(),
            7.619_96,
            0.001
        ));
        assert!(close(
            representation.gpa_per_ev_per_angstrom_cubed().unwrap(),
            160.217_663_4,
            1e-6
        ));
        assert!(close(
            representation.ev_to_kj_per_mol().unwrap(),
            96.485_332,
            1e-6
        ));
    }

    #[test]
    fn representation_view_cannot_resolve_a_physical_coordinate() {
        let representation = si_representation_magnitudes().unwrap();
        assert_eq!(representation.len(), 10);
        assert!(representation.get("alpha").is_none());
        assert!(representation.get("G").is_none());
        assert!(representation.get("m_e").is_none());
        assert!(representation.get("eps_0").is_none());
    }

    #[test]
    fn every_fundamental_projection_is_bound_to_its_symbol_and_source_decimal() {
        let magnitudes = canonical_si_execution_magnitudes().unwrap();
        for (fundamental, projected) in [
            (
                CAESIUM_HYPERFINE_FREQUENCY,
                magnitudes.caesium_hyperfine_frequency,
            ),
            (SPEED_OF_LIGHT, magnitudes.speed_of_light),
            (BOLTZMANN, magnitudes.boltzmann),
            (PLANCK, magnitudes.planck),
            (ELEMENTARY_CHARGE, magnitudes.elementary_charge),
            (AVOGADRO, magnitudes.avogadro),
            (LUMINOUS_EFFICACY, magnitudes.luminous_efficacy),
            (FINE_STRUCTURE, magnitudes.fine_structure),
            (GRAVITATIONAL_CONSTANT, magnitudes.gravitational_constant),
            (ELECTRON_MASS, magnitudes.electron_mass),
        ] {
            assert_eq!(projected.symbol(), fundamental.symbol);
            let source = BigRat::from_decimal_str(fundamental.value).unwrap();
            assert_eq!(
                source.round_to_scale(projected.scale_bits()),
                Some(projected.bits())
            );
            let projection_error = source.sub(&projected.exact_rational()).abs();
            let source_half_ulp = BigRat::decimal_ulp(fundamental.value)
                .unwrap()
                .div(&BigRat::from_i64(2));
            assert!(
                projection_error.cmp_rat(&source_half_ulp) != std::cmp::Ordering::Greater,
                "{} projection exceeds half of its source decimal ULP",
                fundamental.symbol
            );
        }
    }

    #[test]
    fn the_four_derived_values_replay_from_the_published_projected_inputs() {
        fn pow(value: &BigRat, exponent: u32) -> BigRat {
            (0..exponent).fold(BigRat::from_i64(1), |acc, _| acc.mul(value))
        }

        let magnitudes = canonical_si_execution_magnitudes().unwrap();
        let c = magnitudes.speed_of_light.exact_rational();
        let kb = magnitudes.boltzmann.exact_rational();
        let h = magnitudes.planck.exact_rational();
        let e = magnitudes.elementary_charge.exact_rational();
        let na = magnitudes.avogadro.exact_rational();
        let alpha = magnitudes.fine_structure.exact_rational();
        let pi = crate::compute::pi(90);

        let eps0 = pow(&e, 2).div(&BigRat::from_i64(2).mul(&alpha).mul(&h).mul(&c));
        let sigma = BigRat::from_i64(2)
            .mul(&pow(&pi, 5))
            .mul(&pow(&kb, 4))
            .div(&BigRat::from_i64(15).mul(&pow(&h, 3)).mul(&pow(&c, 2)));
        let gas = na.mul(&kb);
        let atomic_volume = BigRat::from_decimal_str("1e24").unwrap().div(&na);

        for (replayed, projected) in [
            (eps0, magnitudes.vacuum_permittivity),
            (sigma, magnitudes.stefan_boltzmann),
            (gas, magnitudes.gas_constant),
            (atomic_volume, magnitudes.atomic_volume_conversion),
        ] {
            assert_eq!(
                replayed.round_to_scale(projected.scale_bits()),
                Some(projected.bits()),
                "{} must replay from the published ancestry bits",
                projected.symbol()
            );
        }
    }

    #[test]
    fn every_composite_derives_its_declared_dimension_and_exact_ancestry() {
        for composite in COMPOSITES {
            validate_composite_contract(&composite)
                .unwrap_or_else(|error| panic!("{}: {error}", composite.symbol));
        }

        let mut wrong_dimension = GAS_CONSTANT;
        wrong_dimension.dimension = crate::fundamentals::SiDimension::DIMENSIONLESS;
        assert!(matches!(
            validate_composite_contract(&wrong_dimension),
            Err(ConstantProjectionError::InvalidMagnitude { .. })
        ));

        let mut wrong_ancestry = GAS_CONSTANT;
        wrong_ancestry.fundamentals = &["N_A"];
        assert!(matches!(
            validate_composite_contract(&wrong_ancestry),
            Err(ConstantProjectionError::InvalidMagnitude { .. })
        ));
    }
}
