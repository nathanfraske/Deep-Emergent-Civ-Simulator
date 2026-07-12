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

//! The deterministic fixed-point composite compute (R-UNITS-PIN). A composite constant (the
//! Stefan-Boltzmann sigma is the first) is DERIVED from its fundamentals by evaluating its declared
//! formula EXACTLY as a rational and rounding ONCE to a fixed-point scale, never authored as a decimal.
//!
//! The value-authoring line and determinism drive every choice here. The formula STRING is parsed and
//! evaluated (the mechanism is fixed code; the formula and the fundamentals it names are data), so the
//! string a composite computes from is the same string the drift-check guards. The evaluation is exact
//! rational arithmetic over [`BigRat`] with a single terminal round-half-to-even, so it is
//! order-independent by construction (no per-operation rounding is a free choice, closing the
//! representation channel the framing panel flagged). A transcendental like pi is computed to a declared
//! working precision by a deterministic Machin series over integers (no hardware float), so the result is
//! bit-identical on every machine. The whole computation runs once at catalogue or manifest load, off the
//! per-tick canonical path.

use crate::bignum::{BigRat, BigUint};
use crate::fundamentals::{fundamental, Composite, Fundamental};

/// Evaluate a composite's declared formula over its named inputs, EXACTLY as a rational. `resolve` maps a
/// symbol (`pi`, `k_B`, ...) to its exact rational value. The returned rational is the composite's true
/// value up to the working precision used for any transcendental symbol; it carries no fixed-point
/// rounding, so a caller rounds it ONCE to whatever scale the value is consumed at.
pub fn evaluate_formula(
    formula: &str,
    resolve: &dyn Fn(&str) -> Result<BigRat, String>,
) -> Result<BigRat, String> {
    let tokens = tokenize(formula)?;
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
        resolve,
    };
    let value = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() {
        return Err(format!(
            "trailing tokens in formula at position {}",
            parser.pos
        ));
    }
    Ok(value)
}

/// Pi to `digits` significant decimal digits, as an exact rational `(pi * 10^digits) / 10^digits`, by
/// Machin's formula `pi = 16*arctan(1/5) - 4*arctan(1/239)`. Deterministic integer arithmetic only; no
/// float, so the value is bit-identical everywhere. `digits` is the working precision (a reserved value:
/// enough to resolve the composite at its scale plus a guard).
pub fn pi(digits: u32) -> BigRat {
    let scale = BigUint::ten_pow(digits);
    let a5 = arctan_reciprocal_scaled(5, &scale);
    let a239 = arctan_reciprocal_scaled(239, &scale);
    // pi * 10^digits = 16*a5 - 4*a239 (both scaled by 10^digits already).
    let pi_scaled = a5
        .mul(&BigRat::from_i64(16))
        .sub(&a239.mul(&BigRat::from_i64(4)));
    // Divide out the scale to recover pi as a rational.
    pi_scaled.div(&BigRat::new(false, scale, BigUint::from_u64(1)))
}

/// Derive a composite's value from the fundamentals table by evaluating its declared formula EXACTLY and
/// rounding ONCE to `scale_bits` (round-half-to-even). This is the composite's canonical derived value: no
/// authored decimal, no per-operation rounding, order-independent by construction.
///
/// It also runs the cross-check the units-arc forward-note requires: the parsed-and-evaluated formula must
/// reproduce the composite's stored reference value (the drift-check reference) to within one unit at
/// `scale_bits`, else the formula string and the recorded value have silently diverged and the compute
/// FAILS LOUD rather than emitting an unverified number. `pi` is computed to `working_digits` by a
/// deterministic series; every fundamental is read from the table as an exact rational. Run once at load,
/// off the canonical path.
pub fn compute_composite_at_scale(
    composite: &Composite,
    working_digits: u32,
    scale_bits: u32,
) -> Result<i128, String> {
    let resolve = |name: &str| -> Result<BigRat, String> {
        if name == "pi" {
            Ok(pi(working_digits))
        } else {
            match fundamental(name) {
                Some(f) => BigRat::from_decimal_str(f.value),
                None => Err(format!(
                    "composite '{}' names unknown symbol '{}'",
                    composite.symbol, name
                )),
            }
        }
    };
    let computed = evaluate_formula(composite.formula, &resolve)?;
    // Cross-check the parsed-and-evaluated formula against the stored reference, keyed off the REFERENCE's
    // own precision (its unit-in-the-last-place), not the derived scale: the stored decimal resolves no
    // finer than its significant figures, so a fixed one-ULP-at-scale tolerance would falsely fail at a fine
    // scale where the derived value out-resolves the reference. A genuine formula/value divergence exceeds
    // the reference ULP by orders of magnitude; a correct derivation sits within half a reference ULP.
    let reference = BigRat::from_decimal_str(composite.value)?;
    let reference_ulp = BigRat::decimal_ulp(composite.value)?;
    let divergence = computed.sub(&reference).abs();
    if divergence.cmp_rat(&reference_ulp) == std::cmp::Ordering::Greater {
        return Err(format!(
            "composite '{}' cross-check FAILED: formula '{}' computes a value more than one reference unit-in-the-last-place from the stored reference value '{}'; the formula string and the recorded value have diverged",
            composite.symbol, composite.formula, composite.value
        ));
    }
    computed.round_to_scale(scale_bits).ok_or_else(|| {
        format!(
            "composite '{}' overflows fixed-point scale {}",
            composite.symbol, scale_bits
        )
    })
}

/// The per-quantity fixed-point scale a composite is stored at, derived from the composite's own magnitude
/// bracket and the caller's global significance target and guard (the R-UNITS-PIN reserved knobs), through
/// the crate's [`crate::derive_scale_bits`]. The magnitude bracket is read from the composite's known value
/// (its order of magnitude), so the scale is a function of the quantity's own data plus the two reserved
/// knobs, never an independent per-composite dial.
pub fn composite_scale_bits(
    composite: &Composite,
    sig_target: u32,
    guard: u32,
    canonical_scale: u32,
) -> Result<u32, String> {
    let value = BigRat::from_decimal_str(composite.value)?;
    let lg = value.floor_log2() as i32;
    Ok(crate::derive_scale_bits(lg, lg, sig_target, guard, canonical_scale).scale_bits)
}

/// The per-quantity fixed-point scale a raw fundamental is stored at when a consumer reads it at a
/// scale, derived from the fundamental's own magnitude bracket and the caller's global significance
/// target and guard (the R-UNITS-PIN reserved knobs), through the crate's [`crate::derive_scale_bits`].
/// The same mechanism as [`composite_scale_bits`], applied to a fundamental: a fundamental such as the
/// gravitational constant (magnitude about `2^-34`, below the canonical Q32.32 epsilon) derives a finer
/// scale that holds it, so it is representable rather than truncating to zero. The scale is a function
/// of the quantity's own magnitude plus the two reserved knobs, never an independent per-fundamental
/// dial.
pub fn fundamental_scale_bits(
    fund: &Fundamental,
    sig_target: u32,
    guard: u32,
    canonical_scale: u32,
) -> Result<u32, String> {
    let value = BigRat::from_decimal_str(fund.value)?;
    let lg = value.floor_log2() as i32;
    Ok(crate::derive_scale_bits(lg, lg, sig_target, guard, canonical_scale).scale_bits)
}

/// The working precision (decimal digits) to compute a transcendental to so the composite's exact value is
/// correct when rounded to `scale_bits`. Derived, not authored: a value of magnitude `2^magnitude_log2`
/// rounded to `scale_bits` needs about `(scale_bits + magnitude_log2) * log10(2)` significant decimal
/// digits, and a fixed guard covers the series-truncation error and the integer-power amplification, so the
/// precision follows from the scale rather than being a free knob. Computed in integers (no float).
pub fn working_digits_for_scale(scale_bits: u32, magnitude_log2: i64) -> u32 {
    const GUARD_DIGITS: i64 = 20;
    let net = scale_bits as i64 + magnitude_log2;
    let significant = if net > 0 { (net * 301) / 1000 } else { 0 };
    (significant + GUARD_DIGITS).max(1) as u32
}

/// Derive a composite's value as a fixed-point magnitude at its OWN canonical scale (the
/// [`composite_scale_bits`] scale), evaluating the formula exactly, computing any transcendental to the
/// derived working precision, rounding ONCE, and running the fail-loud cross-check against the stored
/// reference. The caller projects this to whatever narrower scale it consumes at. `sig_target` and `guard`
/// are the global reserved knobs; `canonical_scale` is the substrate's default fixed-point scale.
pub fn derived_composite_bits(
    composite: &Composite,
    sig_target: u32,
    guard: u32,
    canonical_scale: u32,
) -> Result<(i128, u32), String> {
    let scale_bits = composite_scale_bits(composite, sig_target, guard, canonical_scale)?;
    let value = BigRat::from_decimal_str(composite.value)?;
    let working = working_digits_for_scale(scale_bits, value.floor_log2());
    let bits = compute_composite_at_scale(composite, working, scale_bits)?;
    Ok((bits, scale_bits))
}

/// `arctan(1/x) * scale` as an integer-valued rational, summing the alternating series
/// `arctan(1/x) = sum_k (-1)^k / ((2k+1) * x^(2k+1))` term by term with integer division at scale `scale`,
/// stopping once a term truncates to zero (when `(2k+1)*x^(2k+1) > scale`). The alternating tail and each
/// term's truncation are each below one scale unit, so with the working precision's guard digits the
/// result is exact to the digits that matter.
fn arctan_reciprocal_scaled(x: u64, scale: &BigUint) -> BigRat {
    let x_big = BigUint::from_u64(x);
    let x_sq = x_big.mul(&x_big);
    let mut acc = BigUint::zero(); // running sum of magnitudes, sign handled below
    let mut acc_neg = BigUint::zero();
    // term magnitude at k is scale / ((2k+1) * x^(2k+1)); x^(2k+1) built incrementally.
    let mut x_pow = x_big.clone(); // x^(2k+1), starts at x^1
    let mut k: u64 = 0;
    loop {
        let denom = BigUint::from_u64(2 * k + 1).mul(&x_pow);
        if denom.cmp_big(scale) == std::cmp::Ordering::Greater {
            break; // term truncates to zero, and the alternating tail is bounded below one unit
        }
        let (term, _r) = scale.divmod(&denom);
        if k.is_multiple_of(2) {
            acc = acc.add(&term);
        } else {
            acc_neg = acc_neg.add(&term);
        }
        x_pow = x_pow.mul(&x_sq); // advance to x^(2(k+1)+1)
        k += 1;
    }
    // acc - acc_neg, as a signed integer rational (arctan is positive, so acc >= acc_neg).
    let (neg, mag) = if acc.cmp_big(&acc_neg) != std::cmp::Ordering::Less {
        (false, acc.sub(&acc_neg))
    } else {
        (true, acc_neg.sub(&acc))
    };
    BigRat::new(neg, mag, BigUint::from_u64(1))
}

// ---- formula tokenizer and recursive-descent parser over BigRat ----

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(String),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            b'-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            b'*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            b'/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            b'^' => {
                tokens.push(Token::Caret);
                i += 1;
            }
            b'(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            b')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            _ if c.is_ascii_digit() => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                tokens.push(Token::Number(s[start..i].to_string()));
            }
            _ if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                tokens.push(Token::Ident(s[start..i].to_string()));
            }
            other => {
                return Err(format!(
                    "unexpected character '{}' in formula",
                    other as char
                ))
            }
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    resolve: &'a dyn Fn(&str) -> Result<BigRat, String>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    // expr := term (('+'|'-') term)*
    fn parse_expr(&mut self) -> Result<BigRat, String> {
        let mut acc = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.bump();
                    acc = acc.add(&self.parse_term()?);
                }
                Token::Minus => {
                    self.bump();
                    acc = acc.sub(&self.parse_term()?);
                }
                _ => break,
            }
        }
        Ok(acc)
    }

    // term := factor (('*'|'/') factor)*
    fn parse_term(&mut self) -> Result<BigRat, String> {
        let mut acc = self.parse_factor()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.bump();
                    acc = acc.mul(&self.parse_factor()?);
                }
                Token::Slash => {
                    self.bump();
                    acc = acc.div(&self.parse_factor()?);
                }
                _ => break,
            }
        }
        Ok(acc)
    }

    // factor := base ('^' unsigned-integer)?
    fn parse_factor(&mut self) -> Result<BigRat, String> {
        let base = self.parse_base()?;
        if let Some(Token::Caret) = self.peek() {
            self.bump();
            let exp = match self.bump() {
                Some(Token::Number(n)) => n
                    .parse::<u32>()
                    .map_err(|_| format!("exponent '{n}' is not an unsigned integer"))?,
                other => return Err(format!("expected an integer exponent, found {other:?}")),
            };
            // Exact integer power by repeated multiplication of the rational.
            let mut result = BigRat::from_i64(1);
            for _ in 0..exp {
                result = result.mul(&base);
            }
            Ok(result)
        } else {
            Ok(base)
        }
    }

    // base := number | ident | '(' expr ')'
    fn parse_base(&mut self) -> Result<BigRat, String> {
        // Clone the token out so the mutable borrow from `bump` is released before we call `resolve` or
        // recurse into `parse_expr`.
        let tok = self.bump().cloned();
        match tok {
            Some(Token::Number(n)) => BigRat::from_decimal_str(&n),
            Some(Token::Ident(name)) => (self.resolve)(&name),
            Some(Token::LParen) => {
                let inner = self.parse_expr()?;
                match self.bump() {
                    Some(Token::RParen) => Ok(inner),
                    other => Err(format!("expected ')', found {other:?}")),
                }
            }
            other => Err(format!("expected a value, found {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    // Resolver for the fundamentals sigma needs, as exact rationals from their CODATA decimal strings.
    fn sigma_resolver(working_digits: u32) -> impl Fn(&str) -> Result<BigRat, String> {
        move |name: &str| match name {
            "pi" => Ok(pi(working_digits)),
            "k_B" => BigRat::from_decimal_str("1.380649e-23"),
            "h" => BigRat::from_decimal_str("6.62607015e-34"),
            "c" => BigRat::from_decimal_str("299792458"),
            other => Err(format!("unknown symbol {other}")),
        }
    }

    #[test]
    fn pi_matches_known_digits() {
        // Independently-computed round-half-even references for pi at three scales confirm the Machin
        // series across magnitudes: round(pi * 2^scale) for scale 30, 32, 40.
        let p = pi(40);
        assert_eq!(p.round_to_scale(30), Some(3_373_259_426));
        assert_eq!(p.round_to_scale(32), Some(13_493_037_705));
        assert_eq!(p.round_to_scale(40), Some(3_454_217_652_358));
    }

    #[test]
    fn pi_is_deterministic_across_working_precisions_at_a_fixed_scale() {
        // Beyond enough guard digits, the rounded value at a modest scale is stable.
        let at50 = pi(50).round_to_scale(40);
        let at60 = pi(60).round_to_scale(40);
        assert_eq!(at50, at60);
    }

    #[test]
    fn parser_evaluates_simple_expressions() {
        let noresolve = |_: &str| Err("no symbols".to_string());
        // 2 * (3 + 4) ^ 2 / 7 = 2*49/7 = 14
        let v = evaluate_formula("2 * (3 + 4) ^ 2 / 7", &noresolve).unwrap();
        assert_eq!(v.cmp_rat(&BigRat::from_i64(14)), Ordering::Equal);
        // 15 - 2^3 = 7
        let v = evaluate_formula("15 - 2 ^ 3", &noresolve).unwrap();
        assert_eq!(v.cmp_rat(&BigRat::from_i64(7)), Ordering::Equal);
    }

    #[test]
    fn evaluation_is_order_independent() {
        // The same formula written in two algebraically-equal groupings gives the identical exact rational.
        let r = sigma_resolver(50);
        let a = evaluate_formula("2 * pi^5 * k_B^4 / (15 * h^3 * c^2)", &r).unwrap();
        let b = evaluate_formula("(2 * k_B^4 * pi^5) / (c^2 * 15 * h^3)", &r).unwrap();
        assert_eq!(a.cmp_rat(&b), Ordering::Equal);
    }

    #[test]
    fn sigma_derives_to_the_codata_value() {
        // sigma = 2*pi^5*k_B^4/(15*h^3*c^2). Rounded to a 55-bit scale it must match round(CODATA * 2^55).
        // CODATA sigma = 5.670374419e-8; sigma * 2^55 = 2042913741.7... check exact below.
        let r = sigma_resolver(50);
        let sigma = evaluate_formula("2 * pi^5 * k_B^4 / (15 * h^3 * c^2)", &r).unwrap();
        let reference = BigRat::from_decimal_str("5.670374419e-8").unwrap();
        // The derived value and the CODATA reference agree to within one ULP at scale 55.
        let derived_55 = sigma.round_to_scale(55).unwrap();
        let reference_55 = reference.round_to_scale(55).unwrap();
        assert!(
            (derived_55 - reference_55).abs() <= 1,
            "derived {derived_55} vs reference {reference_55} at scale 55 differ by more than 1 ULP"
        );
    }

    #[test]
    fn sigma_projects_to_q32_32_as_244() {
        // At the sim's raw Q32.32 consumption scale, round-half-even of derived sigma is 244.
        let r = sigma_resolver(50);
        let sigma = evaluate_formula("2 * pi^5 * k_B^4 / (15 * h^3 * c^2)", &r).unwrap();
        assert_eq!(sigma.round_to_scale(32), Some(244));
    }

    #[test]
    fn compute_composite_derives_stefan_boltzmann_and_cross_checks() {
        use crate::fundamentals::STEFAN_BOLTZMANN;
        // The real composite from the table derives and passes the cross-check, at the derived scale 55
        // and at the Q32.32 consumption scale.
        let at55 = compute_composite_at_scale(&STEFAN_BOLTZMANN, 50, 55).unwrap();
        let reference_55 = BigRat::from_decimal_str("5.670374419e-8")
            .unwrap()
            .round_to_scale(55)
            .unwrap();
        assert!((at55 - reference_55).abs() <= 1);
        assert_eq!(
            compute_composite_at_scale(&STEFAN_BOLTZMANN, 50, 32),
            Ok(244)
        );
    }

    #[test]
    fn cross_check_passes_at_a_fine_scale_beyond_reference_precision() {
        use crate::fundamentals::STEFAN_BOLTZMANN;
        // The reference decimal carries ~10 significant figures; at a fine scale (62 bits) the derived value
        // out-resolves it, but the reference-precision-keyed cross-check must still PASS (a scale-coupled
        // one-ULP tolerance would falsely fail here). Regression guard for the section-9 finding.
        assert!(compute_composite_at_scale(&STEFAN_BOLTZMANN, 45, 62).is_ok());
    }

    #[test]
    fn cross_check_fails_loud_on_a_diverged_formula() {
        // A composite whose formula does NOT compute its stored reference value must FAIL, not emit a number.
        let bogus = Composite {
            symbol: "sigma",
            name: "bogus",
            // Missing the pi^5 factor: computes a value far from the stored reference.
            formula: "2 * k_B^4 / (15 * h^3 * c^2)",
            fundamentals: &["k_B", "h", "c"],
            value: "5.670374419e-8",
            unit: "W/(m^2*K^4)",
            provenance: "test",
        };
        let err = compute_composite_at_scale(&bogus, 50, 55).unwrap_err();
        assert!(
            err.contains("cross-check FAILED"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn derived_composite_bits_scale_and_q32_projection() {
        use crate::fundamentals::STEFAN_BOLTZMANN;
        use crate::rescale_bits;
        // Global reserved knobs (dev-fixture values): significance target 30 bits, guard 1.
        let (bits, scale) = derived_composite_bits(&STEFAN_BOLTZMANN, 30, 1, 32).unwrap();
        // sigma's magnitude is ~2^-25, so with sig_target 30 the derived scale is 55.
        assert_eq!(scale, 55);
        // Projected once more to the sim's Q32.32 consumption scale, sigma is 244 x 2^-32.
        let q32 = rescale_bits(bits as i64, scale, 32).unwrap();
        assert_eq!(q32, 244);
    }

    #[test]
    fn derived_working_precision_agrees_with_far_higher_precision() {
        use crate::fundamentals::STEFAN_BOLTZMANN;
        // The auto-derived working precision reproduces the value a much higher precision gives, at the
        // derived scale, so the derived precision is sufficient (not a fabricated cutoff).
        let (bits, scale) = derived_composite_bits(&STEFAN_BOLTZMANN, 30, 1, 32).unwrap();
        let high = compute_composite_at_scale(&STEFAN_BOLTZMANN, 90, scale).unwrap();
        assert_eq!(bits, high);
    }

    #[test]
    fn compute_is_deterministic_and_working_precision_stable() {
        use crate::fundamentals::STEFAN_BOLTZMANN;
        // Beyond enough working precision, the derived fixed-point value is stable and reproducible.
        let a = compute_composite_at_scale(&STEFAN_BOLTZMANN, 45, 55).unwrap();
        let b = compute_composite_at_scale(&STEFAN_BOLTZMANN, 60, 55).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn the_gravitational_constant_derives_a_scale_that_represents_it() {
        use crate::fundamentals::GRAVITATIONAL_CONSTANT;
        // G ~ 6.674e-11 is below the canonical Q32.32 epsilon (~2.3e-10), so at the canonical scale it
        // truncates to zero: that is why a raw fundamental needs its own derived per-quantity scale.
        let g = BigRat::from_decimal_str(GRAVITATIONAL_CONSTANT.value).unwrap();
        assert_eq!(
            g.round_to_scale(32),
            Some(0),
            "G underflows Q32.32, which is what the derived scale fixes"
        );
        // The R-UNITS-PIN scale mechanism derives a finer scale from G's own magnitude (dev-fixture knobs:
        // significance target 30 bits, guard 1), at which G is representable as a non-zero magnitude.
        let scale = fundamental_scale_bits(&GRAVITATIONAL_CONSTANT, 30, 1, 32).unwrap();
        assert!(
            scale > 32,
            "G derives a scale finer than the canonical 32, got {scale}"
        );
        let bits = g.round_to_scale(scale).unwrap();
        assert!(
            bits > 0,
            "G is representable (non-zero) at its derived scale {scale}, got {bits}"
        );
    }
}
