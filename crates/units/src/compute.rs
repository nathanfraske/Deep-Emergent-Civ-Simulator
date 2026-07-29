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
//! representation channel the framing panel flagged). Authority-bearing projections use the
//! `certified_projection` pair: distinct parsers and exact Machin enclosures refine the whole formula until
//! both endpoints occupy one rounding cell. The older point-valued evaluator remains a noncausal test and
//! confirmation utility. The projection runs at catalogue or manifest load, off the per-tick canonical path.

use crate::bignum::{BigRat, BigUint};
use crate::fundamentals::SiDimension;
#[cfg(test)]
use crate::fundamentals::{execution_root, Composite, Fundamental};

/// One exact coordinate supplied to a certified arithmetic projection.
///
/// This type proves representation and arithmetic only. Constructing one does
/// not admit a physical value, establish floor ancestry, or authorize it for a
/// canonical planet stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertifiedFormulaInput {
    /// An engine integer and its binary fractional scale.
    Scaled {
        symbol: &'static str,
        bits: i128,
        scale_bits: u32,
    },
    /// A bounded integer significand times a signed power of two.
    Binary {
        symbol: &'static str,
        bits: i128,
        exponent2: i32,
    },
    /// A closed interval of two significands at one signed binary exponent.
    BinaryInterval {
        symbol: &'static str,
        lower_bits: i128,
        upper_bits: i128,
        exponent2: i32,
    },
    /// A repository-owned exact decimal coordinate.
    Decimal {
        symbol: &'static str,
        value: &'static str,
    },
}

impl CertifiedFormulaInput {
    pub const fn scaled(symbol: &'static str, bits: i128, scale_bits: u32) -> Self {
        Self::Scaled {
            symbol,
            bits,
            scale_bits,
        }
    }

    pub const fn decimal(symbol: &'static str, value: &'static str) -> Self {
        Self::Decimal { symbol, value }
    }

    pub const fn binary(symbol: &'static str, bits: i128, exponent2: i32) -> Self {
        Self::Binary {
            symbol,
            bits,
            exponent2,
        }
    }

    const fn binary_interval(
        symbol: &'static str,
        lower_bits: i128,
        upper_bits: i128,
        exponent2: i32,
    ) -> Self {
        Self::BinaryInterval {
            symbol,
            lower_bits,
            upper_bits,
            exponent2,
        }
    }
}

/// A rounded integer plus the claim-scoped independent arithmetic receipt that
/// selected it. The receipt does not admit the formula or its coordinates as
/// physical inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CertifiedFormulaProjection {
    bits: i128,
    scale_bits: u32,
    binary_exponent2: i32,
    receipt_sha256: [u8; 32],
}

/// An opaque positive invariant factor certified for later interval
/// propagation. It cannot be read as a physical output on its own.
///
/// ```compile_fail
/// let factor = civsim_units::compute::certify_positive_formula_factor("1", &[], 32)
///     .expect("the unit factor certifies");
/// let leaked = format!("{factor:?}");
/// # drop(leaked);
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CertifiedPositiveFormulaFactor {
    bits: i128,
    binary_exponent2: i32,
    receipt_sha256: [u8; 32],
}

impl CertifiedFormulaProjection {
    pub const fn bits(self) -> i128 {
        self.bits
    }

    pub const fn scale_bits(self) -> u32 {
        self.scale_bits
    }

    pub const fn receipt_sha256(self) -> [u8; 32] {
        self.receipt_sha256
    }
}

/// Evaluate a formula through exact rational interval arithmetic.
///
/// A recursive-descent producer and a shunting-yard watchdog independently
/// parse the same formula and enclose Machin's Pi series. The result is emitted
/// only when both enclosures choose one positive magnitude bracket and the same
/// round-half-even integer at `target_scale_bits`. Canonical consumers must
/// separately prove floor or derived-state ancestry; the planet boundary gate
/// forbids this raw arithmetic API at that front door.
pub fn certify_formula_at_scale(
    formula: &str,
    inputs: &[CertifiedFormulaInput],
    target_scale_bits: u32,
) -> Result<CertifiedFormulaProjection, String> {
    let internal = projection_inputs(inputs);
    let certificate =
        crate::certified_projection::certify_at_scale(formula, &internal, target_scale_bits)?;
    Ok(CertifiedFormulaProjection {
        bits: certificate.producer_bits,
        scale_bits: target_scale_bits,
        binary_exponent2: certificate.target_binary_exponent2,
        receipt_sha256: certificate.receipt_sha256,
    })
}

/// Certify a positive formula as an opaque dyadic factor for a terminal scale.
///
/// The fractional scale is derived from the independently agreed magnitude
/// bracket. Its significance budget is `i128::BITS - terminal_scale_bits`, so
/// callers cannot tune coefficient precision independently of the terminal
/// representation. The factor can only be consumed through
/// [`certify_positive_factored_formula_at_scale`].
pub fn certify_positive_formula_factor(
    formula: &str,
    inputs: &[CertifiedFormulaInput],
    terminal_scale_bits: u32,
) -> Result<CertifiedPositiveFormulaFactor, String> {
    let significant_bits = i128::BITS
        .checked_sub(terminal_scale_bits)
        .ok_or_else(|| "terminal scale exceeds the i128 factor representation".to_owned())?;
    let internal = projection_inputs(inputs);
    let certificate =
        crate::certified_projection::certify_at_significance(formula, &internal, significant_bits)?;
    Ok(CertifiedPositiveFormulaFactor {
        bits: certificate.producer_bits,
        binary_exponent2: certificate.target_binary_exponent2,
        receipt_sha256: certificate.receipt_sha256,
    })
}

/// Project `coefficient * dynamic_formula` without recomputing the invariant
/// transcendental coefficient for every member of a sweep or quadrature.
///
/// A certified rounded coefficient encloses its source value inside its
/// rounding cell. The implementation widens that cell to the adjacent dyadic
/// integers, then sends the closed interval through the complete dynamic
/// formula and the independent parser pair. One terminal integer proves every
/// coefficient value inside the cell selects that same result. The returned
/// receipt binds the factor receipt and terminal interval receipt.
pub fn certify_positive_factored_formula_at_scale(
    coefficient: CertifiedPositiveFormulaFactor,
    dynamic_formula: &str,
    inputs: &[CertifiedFormulaInput],
    target_scale_bits: u32,
) -> Result<CertifiedFormulaProjection, String> {
    const COEFFICIENT_SYMBOL: &str = "__civsim_certified_coefficient";
    if coefficient.bits <= 1 {
        return Err("certified factored coefficient is not strictly positive".to_owned());
    }
    if inputs.iter().any(|input| match input {
        CertifiedFormulaInput::Scaled { symbol, .. }
        | CertifiedFormulaInput::Binary { symbol, .. }
        | CertifiedFormulaInput::BinaryInterval { symbol, .. }
        | CertifiedFormulaInput::Decimal { symbol, .. } => *symbol == COEFFICIENT_SYMBOL,
    }) {
        return Err("dynamic inputs use the reserved coefficient symbol".to_owned());
    }
    let combined_formula = format!("{COEFFICIENT_SYMBOL} * ({dynamic_formula})");
    let mut combined_inputs = Vec::with_capacity(inputs.len() + 1);
    combined_inputs.push(CertifiedFormulaInput::binary_interval(
        COEFFICIENT_SYMBOL,
        coefficient
            .bits
            .checked_sub(1)
            .ok_or_else(|| "certified coefficient lower endpoint underflows".to_owned())?,
        coefficient
            .bits
            .checked_add(1)
            .ok_or_else(|| "certified coefficient upper endpoint overflows".to_owned())?,
        coefficient.binary_exponent2,
    ));
    combined_inputs.extend_from_slice(inputs);
    let projection =
        certify_formula_at_scale(&combined_formula, &combined_inputs, target_scale_bits).map_err(
            |error| {
                format!("certified coefficient cell did not select one terminal integer: {error}")
            },
        )?;
    let receipt_sha256 = crate::certified_projection::factored_receipt_digest(
        coefficient.receipt_sha256,
        coefficient.bits,
        coefficient.binary_exponent2,
        dynamic_formula,
        target_scale_bits,
        projection.receipt_sha256,
        projection.bits,
    )?;
    Ok(CertifiedFormulaProjection {
        bits: projection.bits,
        scale_bits: target_scale_bits,
        binary_exponent2: projection.binary_exponent2,
        receipt_sha256,
    })
}

fn projection_inputs(
    inputs: &[CertifiedFormulaInput],
) -> Vec<crate::certified_projection::ProjectionInput> {
    inputs
        .iter()
        .map(|input| match *input {
            CertifiedFormulaInput::Scaled {
                symbol,
                bits,
                scale_bits,
            } => crate::certified_projection::ProjectionInput::new(symbol, bits, scale_bits),
            CertifiedFormulaInput::Binary {
                symbol,
                bits,
                exponent2,
            } => crate::certified_projection::ProjectionInput::binary(symbol, bits, exponent2),
            CertifiedFormulaInput::BinaryInterval {
                symbol,
                lower_bits,
                upper_bits,
                exponent2,
            } => crate::certified_projection::ProjectionInput::binary_interval(
                symbol, lower_bits, upper_bits, exponent2,
            ),
            CertifiedFormulaInput::Decimal { symbol, value } => {
                crate::certified_projection::ProjectionInput::decimal(symbol, value)
            }
        })
        .collect()
}

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

/// Evaluate the same formula grammar over typed SI dimensions.
///
/// Numeric literals and `pi` are dimensionless. Addition and subtraction require
/// equal dimensions; multiplication, division, and integer powers transform the
/// exponent vector exactly. Canonical composite projection calls this before any
/// magnitude is emitted, so a unit label cannot drift away from its formula.
pub fn evaluate_formula_dimension(
    formula: &str,
    resolve: &dyn Fn(&str) -> Result<SiDimension, String>,
) -> Result<SiDimension, String> {
    let tokens = tokenize(formula)?;
    let mut parser = DimensionParser {
        tokens: &tokens,
        pos: 0,
        resolve,
    };
    let value = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() {
        return Err(format!(
            "trailing tokens in dimension formula at position {}",
            parser.pos
        ));
    }
    Ok(value)
}

/// Pi to `digits` significant decimal digits, as an exact rational `(pi * 10^digits) / 10^digits`, by
/// Machin's formula `pi = 16*arctan(1/5) - 4*arctan(1/239)`. Deterministic integer arithmetic only; no
/// float, so the value is bit-identical everywhere.
///
/// This point approximation is retained as a confirmation and test oracle. It
/// does not certify its own truncation or a final rounding decision and must not
/// mint a production value. Authority-bearing callers use
/// [`certify_formula_at_scale`] so independent interval implementations prove
/// the projection.
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
#[cfg(test)]
pub fn compute_composite_at_scale(
    composite: &Composite,
    working_digits: u32,
    scale_bits: u32,
) -> Result<i128, String> {
    let resolve = |name: &str| -> Result<BigRat, String> {
        if name == "pi" {
            Ok(pi(working_digits))
        } else {
            match execution_root(name) {
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
/// bracket and a supplied integer representation target and headroom, through
/// the crate's [`crate::derive_scale_bits`]. The magnitude bracket is read from the composite's known value
/// (its order of magnitude), so the scale is a function of the quantity and one representation policy,
/// never an independent per-composite dial. Canonical production obtains the target from execution types.
#[cfg(test)]
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
/// target and headroom, through the crate's [`crate::derive_scale_bits`].
/// The same mechanism as [`composite_scale_bits`], applied to a fundamental: a fundamental such as the
/// gravitational constant (magnitude about `2^-34`, below the canonical Q32.32 epsilon) derives a finer
/// scale that holds it, so it is representable rather than truncating to zero. The scale is a function
/// of the quantity's own magnitude plus one representation policy, never an independent
/// per-fundamental dial. Canonical production obtains that policy from execution types.
#[cfg(test)]
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
#[cfg(test)]
pub fn working_digits_for_scale(scale_bits: u32, magnitude_log2: i64) -> u32 {
    let required_bits = (i128::from(scale_bits) + i128::from(magnitude_log2) + 1).max(1);
    let required_digits = (required_bits * 302 + 999) / 1000;
    u32::try_from(required_digits.saturating_add(12)).unwrap_or(u32::MAX)
}

/// Derive a composite's value as a fixed-point magnitude at its OWN canonical scale (the
/// [`composite_scale_bits`] scale), evaluating the formula exactly, computing any transcendental to the
/// derived working precision, rounding ONCE, and running the fail-loud cross-check against the stored
/// reference. The caller projects this to whatever narrower scale it consumes at. `sig_target`, `guard`,
/// and `canonical_scale` describe the integer format; canonical production derives them from execution
/// types rather than accepting them as physical inputs.
#[cfg(test)]
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

struct DimensionParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    resolve: &'a dyn Fn(&str) -> Result<SiDimension, String>,
}

impl DimensionParser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn parse_expr(&mut self) -> Result<SiDimension, String> {
        let acc = self.parse_term()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Plus | Token::Minus => {
                    self.bump();
                    let right = self.parse_term()?;
                    if acc != right {
                        return Err("addition or subtraction joins unequal dimensions".to_owned());
                    }
                }
                _ => break,
            }
        }
        Ok(acc)
    }

    fn parse_term(&mut self) -> Result<SiDimension, String> {
        let mut acc = self.parse_factor()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Star => {
                    self.bump();
                    acc = acc.multiply(self.parse_factor()?)?;
                }
                Token::Slash => {
                    self.bump();
                    acc = acc.divide(self.parse_factor()?)?;
                }
                _ => break,
            }
        }
        Ok(acc)
    }

    fn parse_factor(&mut self) -> Result<SiDimension, String> {
        let base = self.parse_base()?;
        if let Some(Token::Caret) = self.peek() {
            self.bump();
            let exponent = match self.bump() {
                Some(Token::Number(value)) => value
                    .parse::<u32>()
                    .map_err(|_| format!("exponent '{value}' is not an unsigned integer"))?,
                other => return Err(format!("expected an integer exponent, found {other:?}")),
            };
            base.pow(exponent)
        } else {
            Ok(base)
        }
    }

    fn parse_base(&mut self) -> Result<SiDimension, String> {
        let token = self.bump().cloned();
        match token {
            Some(Token::Number(_)) => Ok(SiDimension::DIMENSIONLESS),
            Some(Token::Ident(name)) => (self.resolve)(&name),
            Some(Token::LParen) => {
                let inner = self.parse_expr()?;
                match self.bump() {
                    Some(Token::RParen) => Ok(inner),
                    other => Err(format!("expected ')', found {other:?}")),
                }
            }
            other => Err(format!("expected a dimension value, found {other:?}")),
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
    fn factored_projection_matches_the_complete_certified_formula() {
        let pi_factor = certify_positive_formula_factor("1 / pi^2", &[], 32).unwrap();
        let inputs = [CertifiedFormulaInput::scaled("x", 3, 1)];
        let factored =
            certify_positive_factored_formula_at_scale(pi_factor, "x^2 + 1", &inputs, 32).unwrap();
        let complete = certify_formula_at_scale("(x^2 + 1) / pi^2", &inputs, 32).unwrap();
        assert_eq!(factored.bits(), complete.bits());
        assert_eq!(factored.scale_bits(), complete.scale_bits());

        let changed_formula =
            certify_positive_factored_formula_at_scale(pi_factor, "x^2 + 2", &inputs, 32).unwrap();
        let changed_inputs = certify_positive_factored_formula_at_scale(
            pi_factor,
            "x^2 + 1",
            &[CertifiedFormulaInput::scaled("x", 4, 1)],
            32,
        )
        .unwrap();
        assert_ne!(factored.receipt_sha256(), changed_formula.receipt_sha256());
        assert_ne!(factored.receipt_sha256(), changed_inputs.receipt_sha256());

        let large = certify_positive_formula_factor("1e50 / pi^5", &[], 32).unwrap();
        assert!(large.binary_exponent2 > 0);
        let normalized =
            certify_positive_factored_formula_at_scale(large, "x / 1e50", &inputs, 32).unwrap();
        let normalized_complete = certify_formula_at_scale("x / pi^5", &inputs, 32).unwrap();
        assert_eq!(normalized.bits(), normalized_complete.bits());
    }

    #[test]
    fn factored_projection_refuses_unproved_or_ambiguous_cells() {
        assert!(certify_positive_formula_factor("pi", &[], 0).is_err());
        assert!(certify_positive_formula_factor("pi", &[], 128).is_err());

        let factor = certify_positive_formula_factor("1", &[], 126).unwrap();
        let straddle =
            certify_positive_factored_formula_at_scale(factor, "1 / 2", &[], 0).unwrap_err();
        assert!(straddle.contains("one terminal integer"));

        let reserved = [CertifiedFormulaInput::scaled(
            "__civsim_certified_coefficient",
            1,
            0,
        )];
        assert!(certify_positive_factored_formula_at_scale(factor, "1", &reserved, 32).is_err());

        let invalid = CertifiedPositiveFormulaFactor {
            bits: 1,
            binary_exponent2: 0,
            receipt_sha256: [0; 32],
        };
        assert!(certify_positive_factored_formula_at_scale(invalid, "1", &[], 32).is_err());
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
            dimension: crate::fundamentals::STEFAN_BOLTZMANN.dimension,
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
        // Generic mechanism fixture: a 30-bit significance target with one headroom bit.
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
        // The generic scale mechanism derives a finer scale from G's own magnitude under this test format:
        // a 30-bit significance target and one headroom bit. G is then representable and non-zero.
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
