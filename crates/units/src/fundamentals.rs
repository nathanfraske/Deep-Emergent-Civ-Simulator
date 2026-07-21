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

//! Versioned SI representation definitions, measured physical invariants, and
//! the exact relations that join them into the execution view.
//!
//! This module keeps three categories mechanically distinct:
//!
//! - SI defining constants are noncausal representation metadata. Their exact
//!   numerical values define how the repository spells quantities in SI; they
//!   are not `[M]` facts and never enter the admitted physics ledger.
//! - Physical invariants are measured information that remains after the
//!   derive-first search. The canonical planet crate separately admits only
//!   these rows, with Buckingham-Pi, Gap-Law, and Residual-Law receipts.
//! - Derived execution constants are recomputed from the first two categories.
//!   A cited reference decimal is only an off-path drift oracle.
//!
//! Decimal strings are retained because several values are far below Q32.32.
//! The scaled-integer execution projection lives in `constants`; no caller can
//! supply a replacement value or representation schema.

/// The uncertainty attached to a source value in the CODATA table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FundamentalUncertainty {
    /// A defining constant whose numerical value in SI is exact.
    Exact,
    /// A measured value with its standard uncertainty in the same unit.
    Standard(&'static str),
}

impl FundamentalUncertainty {
    /// Stable bitstream spelling for this uncertainty form.
    pub const fn kind_id(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Standard(_) => "standard",
        }
    }

    /// Exact decimal uncertainty in the value's unit. Exact definitions carry zero.
    pub const fn decimal(self) -> &'static str {
        match self {
            Self::Exact => "0",
            Self::Standard(value) => value,
        }
    }
}

/// Stable source identity for the 2018 CODATA complete ASCII table.
pub const CODATA_2018_SOURCE_ID: &str = "nist_codata_2018_ascii";

/// SHA-256 of both the live NIST table and its byte-identical archive witness.
pub const CODATA_2018_SOURCE_SHA256: &str =
    "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1";

/// Versioned noncausal representation contract used by the canonical runner.
/// A change to its base order or exact definitions requires a new schema ID.
pub const SI_REPRESENTATION_SCHEMA_ID: &str = "civsim.units.si-representation.v1";

/// Stable SI base-dimension order used by [`SiDimension`] and the bitstream.
pub const SI_BASE_DIMENSION_IDS: [&str; 7] = [
    "length",
    "mass",
    "time",
    "electric_current",
    "thermodynamic_temperature",
    "amount_of_substance",
    "luminous_intensity",
];

/// Whether a source decimal defines the repository representation or carries
/// independent physical information.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FundamentalRole {
    /// Exact SI definition used only to encode quantities.
    RepresentationDefinition,
    /// Measured physical information eligible for derive-first admission.
    PhysicalInvariant,
}

impl FundamentalRole {
    /// Stable bitstream spelling.
    pub const fn id(self) -> &'static str {
        match self {
            Self::RepresentationDefinition => "representation_definition",
            Self::PhysicalInvariant => "physical_invariant",
        }
    }
}

/// SI base-dimension exponents in the stable order length, mass, time,
/// electric current, thermodynamic temperature, amount of substance, and
/// luminous intensity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SiDimension {
    exponents: [i8; 7],
}

impl SiDimension {
    pub const DIMENSIONLESS: Self = Self::new(0, 0, 0, 0, 0, 0, 0);

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        length: i8,
        mass: i8,
        time: i8,
        current: i8,
        temperature: i8,
        amount: i8,
        luminous_intensity: i8,
    ) -> Self {
        Self {
            exponents: [
                length,
                mass,
                time,
                current,
                temperature,
                amount,
                luminous_intensity,
            ],
        }
    }

    /// Stable exponent vector for bitstream and validation code.
    pub const fn exponents(self) -> [i8; 7] {
        self.exponents
    }

    pub(crate) fn multiply(self, other: Self) -> Result<Self, String> {
        self.combine(other, false)
    }

    pub(crate) fn divide(self, other: Self) -> Result<Self, String> {
        self.combine(other, true)
    }

    pub(crate) fn pow(self, exponent: u32) -> Result<Self, String> {
        let exponent = i16::try_from(exponent)
            .map_err(|_| "dimension exponent does not fit the evaluator".to_owned())?;
        let mut out = [0_i8; 7];
        for (index, value) in self.exponents.into_iter().enumerate() {
            let product = i16::from(value) * exponent;
            out[index] = i8::try_from(product)
                .map_err(|_| "dimension exponent overflow in formula power".to_owned())?;
        }
        Ok(Self { exponents: out })
    }

    fn combine(self, other: Self, subtract: bool) -> Result<Self, String> {
        let mut out = [0_i8; 7];
        for ((slot, left), right) in out
            .iter_mut()
            .zip(self.exponents.into_iter())
            .zip(other.exponents.into_iter())
        {
            *slot = if subtract {
                left.checked_sub(right)
            } else {
                left.checked_add(right)
            }
            .ok_or_else(|| "dimension exponent overflow in formula".to_owned())?;
        }
        Ok(Self { exponents: out })
    }
}

/// One root decimal used by the SI execution view.
///
/// The [`FundamentalRole`] is load-bearing: only `PhysicalInvariant` rows may
/// become `[M]` ledger leaves. `RepresentationDefinition` rows are exact SI
/// conventions and carry no physical provenance tag.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fundamental {
    /// The symbol the derive relations reference, for example `k_B`.
    pub symbol: &'static str,
    /// The human-readable name.
    pub name: &'static str,
    /// The source magnitude as a decimal string. Its role determines whether
    /// this is an exact representation definition or measured physical data.
    pub value: &'static str,
    /// The unit the value is expressed in.
    pub unit: &'static str,
    /// Typed SI dimension independent of the human-readable unit spelling.
    pub dimension: SiDimension,
    /// Noncausal representation definition or measured physical invariant.
    pub role: FundamentalRole,
    /// The citation and provenance (which CODATA release, exact or measured).
    pub provenance: &'static str,
    /// Registry identity of the byte-receipted source table.
    pub source_id: &'static str,
    /// SHA-256 of the exact source artifact read for this value.
    pub source_sha256: &'static str,
    /// Stable row label within the source table.
    pub source_anchor: &'static str,
    /// Exactness or standard uncertainty as printed by the source.
    pub uncertainty: FundamentalUncertainty,
}

// @sources: nist_codata_2018_ascii, codata_2018_adjustment

/// The caesium-133 ground-state hyperfine transition frequency. This exact
/// value fixes the SI second in the representation schema; it is not an
/// independently admitted fact about the simulated system.
pub const CAESIUM_HYPERFINE_FREQUENCY: Fundamental = Fundamental {
    symbol: "Delta_nu_Cs",
    name: "caesium-133 hyperfine transition frequency",
    value: "9192631770",
    unit: "Hz",
    dimension: SiDimension::new(0, 0, -1, 0, 0, 0, 0),
    role: FundamentalRole::RepresentationDefinition,
    provenance: "CODATA 2018 (SI-defining, exact)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "hyperfine transition frequency of Cs-133",
    uncertainty: FundamentalUncertainty::Exact,
};

/// The speed of light in vacuum.
pub const SPEED_OF_LIGHT: Fundamental = Fundamental {
    symbol: "c",
    name: "speed of light in vacuum",
    value: "299792458",
    unit: "m/s",
    dimension: SiDimension::new(1, 0, -1, 0, 0, 0, 0),
    role: FundamentalRole::RepresentationDefinition,
    provenance: "CODATA 2018 (SI-defining, exact)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "speed of light in vacuum",
    uncertainty: FundamentalUncertainty::Exact,
};

/// The Boltzmann constant.
pub const BOLTZMANN: Fundamental = Fundamental {
    symbol: "k_B",
    name: "Boltzmann constant",
    value: "1.380649e-23",
    unit: "J/K",
    dimension: SiDimension::new(2, 1, -2, 0, -1, 0, 0),
    role: FundamentalRole::RepresentationDefinition,
    provenance: "CODATA 2018 (SI-defining since the 2019 SI redefinition, exact)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "Boltzmann constant",
    uncertainty: FundamentalUncertainty::Exact,
};

/// The Planck constant.
pub const PLANCK: Fundamental = Fundamental {
    symbol: "h",
    name: "Planck constant",
    value: "6.62607015e-34",
    unit: "J*s",
    dimension: SiDimension::new(2, 1, -1, 0, 0, 0, 0),
    role: FundamentalRole::RepresentationDefinition,
    provenance: "CODATA 2018 (SI-defining, exact)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "Planck constant",
    uncertainty: FundamentalUncertainty::Exact,
};

/// The elementary charge.
pub const ELEMENTARY_CHARGE: Fundamental = Fundamental {
    symbol: "e",
    name: "elementary charge",
    value: "1.602176634e-19",
    unit: "C",
    dimension: SiDimension::new(0, 0, 1, 1, 0, 0, 0),
    role: FundamentalRole::RepresentationDefinition,
    provenance: "CODATA 2018 (SI-defining, exact)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "elementary charge",
    uncertainty: FundamentalUncertainty::Exact,
};

/// The Avogadro constant.
pub const AVOGADRO: Fundamental = Fundamental {
    symbol: "N_A",
    name: "Avogadro constant",
    value: "6.02214076e23",
    unit: "1/mol",
    dimension: SiDimension::new(0, 0, 0, 0, 0, -1, 0),
    role: FundamentalRole::RepresentationDefinition,
    provenance: "CODATA 2018 (SI-defining since the 2019 SI redefinition, exact)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "Avogadro constant",
    uncertainty: FundamentalUncertainty::Exact,
};

/// The luminous efficacy of monochromatic 540 THz radiation. This exact value
/// fixes the candela representation and contributes no causal floor degree of
/// freedom.
pub const LUMINOUS_EFFICACY: Fundamental = Fundamental {
    symbol: "K_cd",
    name: "luminous efficacy",
    value: "683",
    unit: "lm/W",
    dimension: SiDimension::new(-2, -1, 3, 0, 0, 0, 1),
    role: FundamentalRole::RepresentationDefinition,
    provenance: "CODATA 2018 (SI-defining, exact)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "luminous efficacy",
    uncertainty: FundamentalUncertainty::Exact,
};

/// The electromagnetic fine-structure constant. Unlike `eps_0`, its numerical
/// value is independent of the SI unit representation. The SI execution view
/// derives `eps_0 = e^2 / (2 * alpha * h * c)` from this measured invariant and
/// the exact representation definitions.
pub(crate) const FINE_STRUCTURE: Fundamental = Fundamental {
    symbol: "alpha",
    name: "fine-structure constant",
    value: "7.2973525693e-3",
    unit: "1",
    dimension: SiDimension::DIMENSIONLESS,
    role: FundamentalRole::PhysicalInvariant,
    provenance: "CODATA 2018 (measured, relative standard uncertainty 1.5e-10)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "fine-structure constant",
    uncertainty: FundamentalUncertainty::Standard("1.1e-12"),
};

/// The Newtonian constant of gravitation. Unlike the SI-defining fundamentals above, G is MEASURED,
/// not exact: it is the least precisely known fundamental (its value has resisted tightening for
/// decades), so its provenance carries the relative standard uncertainty. It is the fundamental the
/// genesis-forward stellar and orbital physics reaches (Kepler's third law, the escape velocity, the
/// mass-luminosity relation), joining the floor as its first consumer arrives.
pub(crate) const GRAVITATIONAL_CONSTANT: Fundamental = Fundamental {
    symbol: "G",
    name: "Newtonian constant of gravitation",
    value: "6.67430e-11",
    unit: "m^3/(kg*s^2)",
    dimension: SiDimension::new(3, -1, -2, 0, 0, 0, 0),
    role: FundamentalRole::PhysicalInvariant,
    provenance: "CODATA 2018 (measured, relative standard uncertainty 2.2e-5)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "Newtonian constant of gravitation",
    uncertainty: FundamentalUncertainty::Standard("1.5e-15"),
};

/// The electron rest mass. Like `G` it is MEASURED, not SI-defining-exact, so its provenance carries the
/// relative standard uncertainty. It is the fundamental the disk-opacity generator reaches: the Thomson cross
/// section `sigma_T = (8*pi/3) * (e^2 / (4*pi*eps_0*m_e*c^2))^2` (the electron-scattering opacity `kappa_es`) and
/// the Saha ionization balance (H- and Kramers opacity) both read it, joining the floor as its first consumer
/// arrives.
pub(crate) const ELECTRON_MASS: Fundamental = Fundamental {
    symbol: "m_e",
    name: "electron mass",
    value: "9.1093837015e-31",
    unit: "kg",
    dimension: SiDimension::new(0, 1, 0, 0, 0, 0, 0),
    role: FundamentalRole::PhysicalInvariant,
    provenance: "CODATA 2018 (measured, relative standard uncertainty 3.0e-10)",
    source_id: CODATA_2018_SOURCE_ID,
    source_sha256: CODATA_2018_SOURCE_SHA256,
    source_anchor: "electron mass",
    uncertainty: FundamentalUncertainty::Standard("2.8e-40"),
};

/// Exact SI definitions that encode quantities but contribute no causal degree
/// of freedom. They are outside the provenance ledger by construction.
pub const REPRESENTATION_DEFINITIONS: [Fundamental; 7] = [
    CAESIUM_HYPERFINE_FREQUENCY,
    SPEED_OF_LIGHT,
    PLANCK,
    ELEMENTARY_CHARGE,
    BOLTZMANN,
    AVOGADRO,
    LUMINOUS_EFFICACY,
];

/// Measured information admitted only after derive-first exhaustion.
pub(crate) const PHYSICAL_INVARIANTS: [Fundamental; 3] =
    [FINE_STRUCTURE, GRAVITATIONAL_CONSTANT, ELECTRON_MASS];

/// All roots used to build the SI execution view. This union is not the
/// absolute physics floor: only [`PHYSICAL_INVARIANTS`] are eligible for that
/// ledger, while [`REPRESENTATION_DEFINITIONS`] belong to the versioned schema.
pub(crate) const FUNDAMENTALS: [Fundamental; 10] = [
    CAESIUM_HYPERFINE_FREQUENCY,
    SPEED_OF_LIGHT,
    PLANCK,
    ELEMENTARY_CHARGE,
    BOLTZMANN,
    AVOGADRO,
    LUMINOUS_EFFICACY,
    FINE_STRUCTURE,
    GRAVITATIONAL_CONSTANT,
    ELECTRON_MASS,
];

/// A composite physical constant: computed from the fundamentals, never admitted as an independent magnitude
/// (AGENTIC_ADDENDUM section 9). This records the DERIVE RELATION (the formula and the fundamentals it reads);
/// the fixed-point COMPUTE is the split-out units / R-UNITS-PIN follow-on, because forming the relation (for
/// sigma, `k_B^4 / h^3`) underflows Q32.32 without the scaled-exponent representation that arc builds. The
/// `value` is the known CODATA magnitude, carried only so a drift-check can confirm the stored fundamentals
/// reproduce it. The `formula` string is the human-readable and units-arc record of the relation (that arc
/// parses it to compute); a test cross-checks it against the `fundamentals` list so the two cannot drift
/// apart, but the drift-check itself validates the VALUE through an independent re-encoding of the relation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Composite {
    /// The symbol, for example `sigma`.
    pub symbol: &'static str,
    /// The human-readable name.
    pub name: &'static str,
    /// The derive relation over the fundamentals, as a formula string.
    pub formula: &'static str,
    /// The fundamentals the relation reads (by symbol); each must be a member of [`FUNDAMENTALS`].
    pub fundamentals: &'static [&'static str],
    /// The known CODATA magnitude as a decimal string, for the drift-check only (never the derive source).
    pub(crate) value: &'static str,
    /// The unit the value is expressed in.
    pub unit: &'static str,
    /// Typed SI output dimension that the formula must derive.
    pub dimension: SiDimension,
    /// The citation and provenance.
    pub provenance: &'static str,
}

/// Vacuum electric permittivity in the SI execution representation. The
/// measured electromagnetic input is the dimensionless fine-structure
/// constant; `eps_0` is therefore not an independently admitted leaf.
pub const VACUUM_PERMITTIVITY: Composite = Composite {
    symbol: "eps_0",
    name: "vacuum electric permittivity",
    formula: "e^2 / (2 * alpha * h * c)",
    fundamentals: &["e", "alpha", "h", "c"],
    value: "8.8541878128e-12",
    unit: "F/m",
    dimension: SiDimension::new(-3, -1, 4, 2, 0, 0, 0),
    provenance: "derived from alpha and the exact SI definitions e, h, and c",
};

/// The Stefan-Boltzmann constant, the radiant-emission proportionality: a CODATA composite that DERIVES from
/// the fundamentals. Its former standalone value remains only in the parked legacy archive.
pub const STEFAN_BOLTZMANN: Composite = Composite {
    symbol: "sigma",
    name: "Stefan-Boltzmann constant",
    formula: "2 * pi^5 * k_B^4 / (15 * h^3 * c^2)",
    fundamentals: &["k_B", "h", "c"],
    value: "5.670374419e-8",
    unit: "W/(m^2*K^4)",
    dimension: SiDimension::new(0, 1, -3, 0, -4, 0, 0),
    provenance: "CODATA 2018 (derived from k_B, h, c)",
};

/// The molar gas constant `R = N_A * k_B`, the product of the Avogadro and Boltzmann fundamentals, DERIVED and
/// never admitted as its own number. Both fundamentals are SI-defining and exact, so the product is exact. It is
/// the constant the volatile-thermodynamics saturation curve and the ideal-gas laws read (`R_v = R / M` for a
/// substance's specific gas constant, and the Rankine-Kirchhoff constants for a volatile's saturation curve).
pub const GAS_CONSTANT: Composite = Composite {
    symbol: "R",
    name: "molar gas constant",
    formula: "N_A * k_B",
    fundamentals: &["N_A", "k_B"],
    value: "8.314462618",
    unit: "J/(mol*K)",
    dimension: SiDimension::new(2, 1, -2, 0, -1, -1, 0),
    provenance: "CODATA 2018 (derived from N_A, k_B; exact since the 2019 SI redefinition)",
};

/// Cubic angstroms per atom contributed by one cubic centimetre per mole.
///
/// This is the exact unit conversion `10^24 / N_A`: one cubic centimetre is
/// `10^24` cubic angstroms, and the Avogadro constant supplies particles per
/// mole. It replaces a rounded decimal formerly embedded in the materials
/// thermoelastic path.
pub const ATOMIC_VOLUME_CONVERSION: Composite = Composite {
    symbol: "A3_per_cm3_mol",
    name: "atomic-volume molar conversion",
    formula: "10^24 / N_A",
    fundamentals: &["N_A"],
    value: "1.6605390671738466",
    unit: "angstrom^3/(cm^3/mol)",
    dimension: SiDimension::new(0, 0, 0, 0, 0, 1, 0),
    provenance: "derived exactly from the SI centimetre, angstrom, mole, and N_A",
};

/// Derived constants in the SI execution view.
pub const COMPOSITES: [Composite; 4] = [
    VACUUM_PERMITTIVITY,
    STEFAN_BOLTZMANN,
    GAS_CONSTANT,
    ATOMIC_VOLUME_CONVERSION,
];

/// Look up an exact SI representation definition by its symbol.
///
/// Measured physical coordinates are deliberately absent from this public
/// lookup. They become readable only through a verified SI execution
/// capability.
pub fn fundamental(symbol: &str) -> Option<&'static Fundamental> {
    REPRESENTATION_DEFINITIONS
        .iter()
        .find(|definition| definition.symbol == symbol)
}

pub(crate) fn execution_root(symbol: &str) -> Option<&'static Fundamental> {
    FUNDAMENTALS.iter().find(|root| root.symbol == symbol)
}

/// Look up a composite by its symbol.
pub fn composite(symbol: &str) -> Option<&'static Composite> {
    COMPOSITES.iter().find(|c| c.symbol == symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The drift-check computes the composite from the stored fundamentals and confirms it reproduces the
    // recorded CODATA value. It uses `f64` DELIBERATELY and ONLY here, in a test, to validate the recorded
    // relation: no float touches the crate's canonical integer path, and the fixed-point compute of a
    // composite is the split-out units / R-UNITS-PIN follow-on (the relation underflows Q32.32).
    fn parse(f: &Fundamental) -> f64 {
        f.value
            .parse()
            .expect("a fundamental's value parses as a decimal")
    }

    #[test]
    fn the_stored_fundamentals_reproduce_stefan_boltzmann_the_drift_check() {
        let k_b = parse(&BOLTZMANN);
        let h = parse(&PLANCK);
        let c = parse(&SPEED_OF_LIGHT);
        let pi = std::f64::consts::PI;
        let derived = 2.0 * pi.powi(5) * k_b.powi(4) / (15.0 * h.powi(3) * c.powi(2));
        let recorded: f64 = STEFAN_BOLTZMANN
            .value
            .parse()
            .expect("the composite's recorded value parses");
        let relative = (derived - recorded).abs() / recorded;
        // The stored fundamentals reproduce sigma to ~3.3e-11; the bound is tight enough to catch any
        // mistyped fundamental (which perturbs sigma by orders of magnitude) with headroom over the achieved
        // agreement, and does not depend on the recorded value's last digit.
        assert!(
            relative < 1e-8,
            "sigma derived from the fundamentals ({derived:e}) drifts from the recorded CODATA value ({recorded:e}), relative {relative:e}"
        );
    }

    #[test]
    fn fine_structure_derives_vacuum_permittivity_in_si() {
        let e = parse(&ELEMENTARY_CHARGE);
        let alpha = parse(&FINE_STRUCTURE);
        let h = parse(&PLANCK);
        let c = parse(&SPEED_OF_LIGHT);
        let derived = e.powi(2) / (2.0 * alpha * h * c);
        let recorded: f64 = VACUUM_PERMITTIVITY
            .value
            .parse()
            .expect("the permittivity drift oracle parses");
        let relative = (derived - recorded).abs() / recorded;
        assert!(
            relative < 1e-9,
            "eps_0 derived from alpha and the SI schema ({derived:e}) drifts from the CODATA oracle ({recorded:e}), relative {relative:e}"
        );
    }

    #[test]
    fn the_stored_fundamentals_reproduce_the_gas_constant_the_drift_check() {
        // R = N_A * k_B, both SI-defining and exact since 2019, so the product is exact to the recorded value.
        // f64 DELIBERATELY and ONLY here (a test), the same convention as the sigma drift-check.
        let n_a = parse(&AVOGADRO);
        let k_b = parse(&BOLTZMANN);
        let derived = n_a * k_b;
        let recorded: f64 = GAS_CONSTANT
            .value
            .parse()
            .expect("the composite's recorded value parses");
        let relative = (derived - recorded).abs() / recorded;
        assert!(
            relative < 1e-8,
            "R derived from N_A and k_B ({derived:e}) drifts from the recorded value ({recorded:e}), relative {relative:e}"
        );
    }

    #[test]
    fn avogadro_derives_the_atomic_volume_conversion() {
        let n_a = parse(&AVOGADRO);
        let derived = 1.0e24 / n_a;
        let recorded: f64 = ATOMIC_VOLUME_CONVERSION
            .value
            .parse()
            .expect("the conversion's recorded value parses");
        let relative = (derived - recorded).abs() / recorded;
        assert!(
            relative < 1e-14,
            "10^24 / N_A ({derived:e}) drifts from the recorded conversion ({recorded:e}), relative {relative:e}"
        );
    }

    #[test]
    fn every_composite_reads_only_declared_fundamentals() {
        for comp in COMPOSITES {
            for symbol in comp.fundamentals {
                assert!(
                    execution_root(symbol).is_some(),
                    "composite {} reads an undeclared fundamental {symbol}",
                    comp.symbol
                );
            }
        }
    }

    #[test]
    fn a_composite_formula_and_its_fundamentals_list_agree() {
        // The `formula` string and the `fundamentals` list are both load-bearing for the units-arc compute,
        // so they must not drift apart: every declared fundamental appears in the formula (no padded list),
        // and every fundamental whose symbol appears in the formula is declared (no missing list entry). The
        // symbol matching is a substring test, which suits the current symbol set (`k_B`, `h`, `c`, and the
        // rest are distinct within each formula); a future symbol that is a substring of another would need a
        // tokenizing check.
        for comp in COMPOSITES {
            for symbol in comp.fundamentals {
                assert!(
                    comp.formula.contains(symbol),
                    "composite {}: declared fundamental {symbol} is absent from the formula '{}'",
                    comp.symbol,
                    comp.formula
                );
            }
            for f in FUNDAMENTALS {
                if comp.formula.contains(f.symbol) {
                    assert!(
                        comp.fundamentals.contains(&f.symbol),
                        "composite {}: the formula reads {} but it is not in the declared fundamentals list",
                        comp.symbol,
                        f.symbol
                    );
                }
            }
        }
    }

    #[test]
    fn every_fundamental_value_parses_finite_and_positive() {
        // A magnitude guard over every execution root, not only the three the sigma drift-check consumes: a
        // raw fundamental has no internal derivation to check a wrong DIGIT against (that is human review
        // against the recorded provenance), but a malformed, non-finite, or non-positive value is a defect a
        // cheap self-consistent test catches for the whole table.
        for f in FUNDAMENTALS {
            let v: f64 = f.value.parse().unwrap_or_else(|_| {
                panic!(
                    "fundamental {} value '{}' does not parse",
                    f.symbol, f.value
                )
            });
            assert!(
                v.is_finite() && v > 0.0,
                "fundamental {} value {v:e} is not finite and positive",
                f.symbol
            );
        }
    }

    #[test]
    fn the_fundamentals_are_stored_once() {
        for (i, a) in FUNDAMENTALS.iter().enumerate() {
            for b in &FUNDAMENTALS[i + 1..] {
                assert_ne!(
                    a.symbol, b.symbol,
                    "fundamental {} is stored twice",
                    a.symbol
                );
            }
        }
    }

    #[test]
    fn the_lookups_find_and_miss() {
        assert_eq!(fundamental("k_B"), Some(&BOLTZMANN));
        assert_eq!(fundamental("alpha"), None);
        assert_eq!(execution_root("alpha"), Some(&FINE_STRUCTURE));
        assert!(fundamental("eps_0").is_none());
        assert!(fundamental("not_a_constant").is_none());
        assert_eq!(composite("eps_0"), Some(&VACUUM_PERMITTIVITY));
        assert_eq!(composite("sigma"), Some(&STEFAN_BOLTZMANN));
        assert_eq!(composite("A3_per_cm3_mol"), Some(&ATOMIC_VOLUME_CONVERSION));
    }

    #[test]
    fn representation_definitions_and_physical_invariants_do_not_overlap() {
        assert_eq!(REPRESENTATION_DEFINITIONS.len(), 7);
        assert_eq!(PHYSICAL_INVARIANTS.len(), 3);
        assert!(REPRESENTATION_DEFINITIONS
            .iter()
            .all(|entry| entry.role == FundamentalRole::RepresentationDefinition));
        assert!(PHYSICAL_INVARIANTS
            .iter()
            .all(|entry| entry.role == FundamentalRole::PhysicalInvariant));
        for representation in REPRESENTATION_DEFINITIONS {
            assert!(PHYSICAL_INVARIANTS
                .iter()
                .all(|physical| physical.symbol != representation.symbol));
        }
    }

    #[test]
    fn the_gravitational_constant_is_a_measured_physical_invariant() {
        let g = execution_root("G").expect("G is in the internal execution-root table");
        assert_eq!(g, &GRAVITATIONAL_CONSTANT);
        assert_eq!(g.role, FundamentalRole::PhysicalInvariant);
        assert_eq!(g.name, "Newtonian constant of gravitation");
        assert_eq!(g.unit, "m^3/(kg*s^2)");
        // G is MEASURED, not SI-defining-exact: its provenance carries the uncertainty, unlike c/h/k_B/e/N_A.
        assert!(
            g.provenance.contains("measured"),
            "G's provenance must record it as measured, not exact: {}",
            g.provenance
        );
        // The value parses and sits at the expected order of magnitude (~6.674e-11). f64 in a test only.
        let v: f64 = g.value.parse().expect("G's value parses");
        assert!(
            (6.6e-11..6.8e-11).contains(&v),
            "G is about 6.674e-11, got {v:e}"
        );
    }

    #[test]
    fn the_electron_mass_is_a_measured_physical_invariant() {
        let m_e = execution_root("m_e").expect("m_e is in the internal execution-root table");
        assert_eq!(m_e, &ELECTRON_MASS);
        assert_eq!(m_e.role, FundamentalRole::PhysicalInvariant);
        assert_eq!(m_e.name, "electron mass");
        assert_eq!(m_e.unit, "kg");
        assert!(
            m_e.provenance.contains("measured"),
            "m_e's provenance must record it as measured, not exact: {}",
            m_e.provenance
        );
        // The value parses and sits at the expected order of magnitude (~9.109e-31). f64 in a test only.
        let v: f64 = m_e.value.parse().expect("m_e's value parses");
        assert!(
            (9.0e-31..9.2e-31).contains(&v),
            "m_e is about 9.109e-31, got {v:e}"
        );
    }
}
