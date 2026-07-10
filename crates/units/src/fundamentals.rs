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

//! The closed table of fundamental physical constants: the ONE authored universal layer of the
//! value-authoring line (AGENTIC_ADDENDUM section 9, the fundamental-constants floor). The universal authored
//! layer reduces to exactly the fundamental constants reality measures and cannot derive from anything deeper
//! (`c`, `k_B`, `h`, `e`, `eps_0`, `N_A`, the handful this engine's physics reaches). The list is small,
//! closed, and does not grow, because reality's list does not grow. Everything DERIVABLE derives from these:
//! a composite such as the Stefan-Boltzmann sigma is COMPUTED from the fundamentals, never authored as its own
//! number.
//!
//! Each value is the CODATA-measured magnitude, dictated by reality rather than owner-set, held as a DECIMAL
//! STRING because several fundamentals (`k_B` ~1.4e-23, `h` ~6.6e-34, `e` ~1.6e-19) are far below the Q32.32
//! epsilon (~2.3e-10) and cannot be held as a plain fixed-point magnitude. Forming a composite such as
//! `k_B^4 / h^3` underflows fixed-point, so the fixed-point COMPUTE of a composite is the split-out units /
//! R-UNITS-PIN follow-on (the scaled-exponent representation that arc builds); this table declares the closed
//! authored layer and records each composite's derive relation, it does not itself compute a composite.
//!
//! This is the deliberate exception to the crate's "ships no value" rule (see the crate docs): the crate ships
//! no owner or per-world value (a dimension, a quantity, a scale), but it does carry the closed CODATA
//! fundamentals, because they ARE the one authored universal layer the three-way test permits.

/// A fundamental physical constant: one entry of the closed authored universal layer. The value is a CODATA
/// decimal string (several fundamentals underflow Q32.32, so no plain fixed-point magnitude holds them; the
/// fixed-point representation is the units / R-UNITS-PIN follow-on).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fundamental {
    /// The symbol the derive relations reference, for example `k_B`.
    pub symbol: &'static str,
    /// The human-readable name.
    pub name: &'static str,
    /// The CODATA-measured magnitude as a decimal string (dictated by reality, never owner-set).
    pub value: &'static str,
    /// The unit the value is expressed in.
    pub unit: &'static str,
    /// The citation and provenance (which CODATA release, exact or measured).
    pub provenance: &'static str,
}

/// The speed of light in vacuum.
pub const SPEED_OF_LIGHT: Fundamental = Fundamental {
    symbol: "c",
    name: "speed of light in vacuum",
    value: "299792458",
    unit: "m/s",
    provenance: "CODATA 2018 (SI-defining, exact)",
};

/// The Boltzmann constant.
pub const BOLTZMANN: Fundamental = Fundamental {
    symbol: "k_B",
    name: "Boltzmann constant",
    value: "1.380649e-23",
    unit: "J/K",
    provenance: "CODATA 2018 (SI-defining since the 2019 SI redefinition, exact)",
};

/// The Planck constant.
pub const PLANCK: Fundamental = Fundamental {
    symbol: "h",
    name: "Planck constant",
    value: "6.62607015e-34",
    unit: "J*s",
    provenance: "CODATA 2018 (SI-defining, exact)",
};

/// The elementary charge.
pub const ELEMENTARY_CHARGE: Fundamental = Fundamental {
    symbol: "e",
    name: "elementary charge",
    value: "1.602176634e-19",
    unit: "C",
    provenance: "CODATA 2018 (SI-defining, exact)",
};

/// The vacuum electric permittivity.
pub const VACUUM_PERMITTIVITY: Fundamental = Fundamental {
    symbol: "eps_0",
    name: "vacuum electric permittivity",
    value: "8.8541878128e-12",
    unit: "F/m",
    provenance: "CODATA 2018 (measured, relative uncertainty 1.5e-10)",
};

/// The Avogadro constant.
pub const AVOGADRO: Fundamental = Fundamental {
    symbol: "N_A",
    name: "Avogadro constant",
    value: "6.02214076e23",
    unit: "1/mol",
    provenance: "CODATA 2018 (SI-defining since the 2019 SI redefinition, exact)",
};

/// The closed, non-growing list of the fundamental constants this engine's physics reaches. Reality's list
/// does not grow, so neither does this one (AGENTIC_ADDENDUM section 9).
pub const FUNDAMENTALS: [Fundamental; 6] = [
    SPEED_OF_LIGHT,
    BOLTZMANN,
    PLANCK,
    ELEMENTARY_CHARGE,
    VACUUM_PERMITTIVITY,
    AVOGADRO,
];

/// A composite physical constant: computed from the fundamentals, never authored as its own number
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
    /// The known CODATA magnitude as a decimal string, for the drift-check only (never the authored source).
    pub value: &'static str,
    /// The unit the value is expressed in.
    pub unit: &'static str,
    /// The citation and provenance.
    pub provenance: &'static str,
}

/// The Stefan-Boltzmann constant, the radiant-emission proportionality: a CODATA composite that DERIVES from
/// the fundamentals, retired here from its former authored decimal in `calibration/reserved.toml`.
pub const STEFAN_BOLTZMANN: Composite = Composite {
    symbol: "sigma",
    name: "Stefan-Boltzmann constant",
    formula: "2 * pi^5 * k_B^4 / (15 * h^3 * c^2)",
    fundamentals: &["k_B", "h", "c"],
    value: "5.670374419e-8",
    unit: "W/(m^2*K^4)",
    provenance: "CODATA 2018 (derived from k_B, h, c)",
};

/// The composites this table records as derive-targets over the fundamentals.
pub const COMPOSITES: [Composite; 1] = [STEFAN_BOLTZMANN];

/// Look up a fundamental by its symbol.
pub fn fundamental(symbol: &str) -> Option<&'static Fundamental> {
    FUNDAMENTALS.iter().find(|f| f.symbol == symbol)
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
    fn every_composite_reads_only_declared_fundamentals() {
        for comp in COMPOSITES {
            for symbol in comp.fundamentals {
                assert!(
                    fundamental(symbol).is_some(),
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
        // A magnitude guard over ALL six fundamentals, not only the three the sigma drift-check consumes: a
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
    fn the_fundamentals_are_deduplicated_authored_once() {
        for (i, a) in FUNDAMENTALS.iter().enumerate() {
            for b in &FUNDAMENTALS[i + 1..] {
                assert_ne!(
                    a.symbol, b.symbol,
                    "fundamental {} is authored twice",
                    a.symbol
                );
            }
        }
    }

    #[test]
    fn the_lookups_find_and_miss() {
        assert_eq!(fundamental("k_B"), Some(&BOLTZMANN));
        assert!(fundamental("not_a_constant").is_none());
        assert_eq!(composite("sigma"), Some(&STEFAN_BOLTZMANN));
    }
}
