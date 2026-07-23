use super::{
    formula_digest, input_digest, ProjectionCertificate, ProjectionInput, CERTIFICATE_SCHEMA_ID,
    FACTORED_CERTIFICATE_SCHEMA_ID, FACTORED_PRODUCER_IMPLEMENTATION_ID,
    FACTORED_WATCHDOG_IMPLEMENTATION_ID, PRODUCER_IMPLEMENTATION_ID, WATCHDOG_IMPLEMENTATION_ID,
};
use crate::bignum::{BigRat, BigUint};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::sync::OnceLock;

const TERM_STEPS: [u32; 6] = [4, 8, 16, 32, 64, 128];
const MAX_AST_NODES: usize = 128;
const MAX_TOKENS: usize = 256;
const MAX_FORMULA_BYTES: usize = 4096;
const MAX_INPUTS: usize = 128;
const MAX_SYMBOL_BYTES: usize = 64;
const MAX_DECIMAL_BYTES: usize = 256;
const MAX_DECIMAL_DIGITS: usize = 64;
const MAX_DECIMAL_POWER: u64 = 256;
const MAX_POWER_EXPONENT: u32 = 64;
const MAX_NESTING_DEPTH: usize = 64;
const MAX_SIGN_RUN: usize = 64;
const MAX_NORMALIZATION_SHIFT_BITS: u32 = 4096;
const MAX_INTERMEDIATE_COMPONENT_BITS: u64 = 65_536;
// Machin-series endpoints carry denominators hundreds of digits wide. Keeping
// those exact fractions inside every downstream multiply is correct but turns
// a table quadrature into minutes of denominator arithmetic. Each cached Pi
// enclosure is therefore widened outward onto a dyadic grid before formula
// evaluation. The grid is far finer than any signed-i128 projection this API
// can emit, and widening by one unit on each side preserves the proof.
const PI_DYADIC_ENCLOSURE_BITS: u32 = 120;

#[derive(Clone, Debug)]
enum Expr {
    Number(BigRat),
    Symbol(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, u32),
    Neg(Box<Expr>),
}

impl Expr {
    fn contains_pi(&self) -> bool {
        match self {
            Self::Symbol(symbol) => symbol == "pi",
            Self::Number(_) => false,
            Self::Add(left, right)
            | Self::Sub(left, right)
            | Self::Mul(left, right)
            | Self::Div(left, right) => left.contains_pi() || right.contains_pi(),
            Self::Pow(value, _) | Self::Neg(value) => value.contains_pi(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Number(String),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Left,
    Right,
}

pub(super) fn produce(
    formula: &str,
    inputs: &[ProjectionInput],
    target_scale_bits: u32,
) -> Result<ProjectionCertificate, String> {
    produce_for_request(formula, inputs, ScaleRequest::Fixed(target_scale_bits))
}

pub(super) fn produce_significant(
    formula: &str,
    inputs: &[ProjectionInput],
    significant_bits: u32,
) -> Result<ProjectionCertificate, String> {
    if !(1..=120).contains(&significant_bits) {
        return Err(
            "certified coefficient significance must be from 1 through 120 bits".to_owned(),
        );
    }
    produce_for_request(formula, inputs, ScaleRequest::Significant(significant_bits))
}

#[derive(Clone, Copy)]
enum ScaleRequest {
    Fixed(u32),
    Significant(u32),
}

fn produce_for_request(
    formula: &str,
    inputs: &[ProjectionInput],
    scale_request: ScaleRequest,
) -> Result<ProjectionCertificate, String> {
    validate_projection_resources(formula, inputs, scale_request)?;
    let tokens = lex(formula)?;
    if tokens.len() > MAX_TOKENS {
        return Err("formula exceeds the certified token resource cap".to_owned());
    }
    let mut parser = Parser::new(&tokens);
    let expression = parser.expression()?;
    if parser.position != tokens.len() {
        return Err("producer parser found trailing formula tokens".to_owned());
    }
    if parser.nodes > MAX_AST_NODES {
        return Err("formula exceeds the certified AST resource cap".to_owned());
    }
    let steps: &[u32] = if expression.contains_pi() {
        &TERM_STEPS
    } else {
        &[0]
    };
    for &pi_terms in steps {
        let interval = evaluate(&expression, inputs, pi_terms)?;
        let zero = BigRat::from_i64(0);
        if interval.lower.cmp_rat(&zero) != Ordering::Greater
            || interval.upper.cmp_rat(&zero) != Ordering::Greater
        {
            continue;
        }
        let lower_log2 = interval.lower.floor_log2();
        let upper_log2 = interval.upper.floor_log2();
        if lower_log2 != upper_log2 {
            continue;
        }
        let target_binary_exponent2 = match scale_request {
            ScaleRequest::Fixed(scale) => i32::try_from(scale)
                .ok()
                .and_then(i32::checked_neg)
                .ok_or_else(|| "fixed projection scale exceeds signed exponent range".to_owned())?,
            ScaleRequest::Significant(bits) => {
                let exponent = lower_log2
                    .checked_sub(i64::from(bits) - 1)
                    .ok_or_else(|| "derived coefficient exponent overflows i64".to_owned())?;
                i32::try_from(exponent)
                    .map_err(|_| "derived coefficient exponent exceeds i32".to_owned())?
            }
        };
        if target_binary_exponent2.unsigned_abs() > MAX_NORMALIZATION_SHIFT_BITS {
            return Err(
                "derived coefficient exponent exceeds the normalization resource cap".to_owned(),
            );
        }
        let Some(lower_bits) = interval
            .lower
            .round_to_binary_exponent(target_binary_exponent2)
        else {
            return Err("certified projection lower endpoint exceeds i128".to_owned());
        };
        let Some(upper_bits) = interval
            .upper
            .round_to_binary_exponent(target_binary_exponent2)
        else {
            return Err("certified projection upper endpoint exceeds i128".to_owned());
        };
        if lower_bits != upper_bits {
            continue;
        }
        return Ok(ProjectionCertificate {
            schema_id: CERTIFICATE_SCHEMA_ID,
            producer_implementation_id: PRODUCER_IMPLEMENTATION_ID,
            watchdog_implementation_id: WATCHDOG_IMPLEMENTATION_ID,
            formula_sha256: formula_digest(formula),
            inputs_sha256: input_digest(inputs),
            target_binary_exponent2,
            pi_terms,
            magnitude_log2: lower_log2,
            lower: interval.lower,
            upper: interval.upper,
            producer_bits: lower_bits,
            watchdog_bits: 0,
            receipt_sha256: [0; 32],
        });
    }
    Err("formula interval did not certify one magnitude bracket and rounded integer within the Pi resource cap".to_owned())
}

fn validate_projection_resources(
    formula: &str,
    inputs: &[ProjectionInput],
    scale_request: ScaleRequest,
) -> Result<(), String> {
    if formula.len() > MAX_FORMULA_BYTES {
        return Err("formula exceeds the certified byte resource cap".to_owned());
    }
    if inputs.len() > MAX_INPUTS {
        return Err("projection input count exceeds the certified resource cap".to_owned());
    }
    if matches!(scale_request, ScaleRequest::Fixed(scale) if scale > MAX_NORMALIZATION_SHIFT_BITS) {
        return Err("fixed projection scale exceeds the normalization resource cap".to_owned());
    }
    for (index, input) in inputs.iter().enumerate() {
        let mut bytes = input.symbol.bytes();
        let valid_first = bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
        if !valid_first
            || input.symbol.len() > MAX_SYMBOL_BYTES
            || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || input.symbol == "pi"
        {
            return Err(format!(
                "projection input '{}' has an invalid or reserved symbol",
                input.symbol
            ));
        }
        if inputs[..index]
            .iter()
            .any(|prior| prior.symbol == input.symbol)
        {
            return Err(format!("projection input '{}' is duplicated", input.symbol));
        }
        match input.coordinate {
            super::ProjectionCoordinate::Scaled { scale_bits, .. }
                if scale_bits > MAX_NORMALIZATION_SHIFT_BITS =>
            {
                return Err(
                    "scaled projection input exceeds the coordinate resource cap".to_owned(),
                )
            }
            super::ProjectionCoordinate::Binary { exponent2, .. }
                if exponent2.unsigned_abs() > MAX_NORMALIZATION_SHIFT_BITS =>
            {
                return Err(
                    "binary projection input exceeds the coordinate resource cap".to_owned(),
                )
            }
            super::ProjectionCoordinate::BinaryInterval {
                lower_bits,
                upper_bits,
                exponent2,
            } => {
                if lower_bits > upper_bits {
                    return Err("projection input interval is reversed".to_owned());
                }
                if exponent2.unsigned_abs() > MAX_NORMALIZATION_SHIFT_BITS {
                    return Err(
                        "interval projection input exceeds the coordinate resource cap".to_owned(),
                    );
                }
            }
            super::ProjectionCoordinate::Decimal(value) => {
                validate_decimal_resource(value)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_decimal_resource(value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.len() > MAX_DECIMAL_BYTES {
        return Err("decimal exceeds the certified byte resource cap".to_owned());
    }
    let unsigned = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    let mut exponent_split = unsigned.split(['e', 'E']);
    let mantissa = exponent_split.next().unwrap_or_default();
    let exponent = exponent_split
        .next()
        .map(|raw| raw.parse::<i64>())
        .transpose()
        .map_err(|_| "decimal exponent is malformed".to_owned())?
        .unwrap_or(0);
    if exponent_split.next().is_some() {
        return Err("decimal contains multiple exponents".to_owned());
    }
    let mut point_split = mantissa.split('.');
    let integer = point_split.next().unwrap_or_default();
    let fraction = point_split.next().unwrap_or_default();
    if point_split.next().is_some()
        || (integer.is_empty() && fraction.is_empty())
        || !integer
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return Err("decimal mantissa is malformed".to_owned());
    }
    let digit_count = integer.len() + fraction.len();
    if digit_count > MAX_DECIMAL_DIGITS {
        return Err("decimal exceeds the certified digit resource cap".to_owned());
    }
    let fractional_places = i64::try_from(fraction.len())
        .map_err(|_| "decimal fractional length exceeds i64".to_owned())?;
    let net = exponent
        .checked_sub(fractional_places)
        .ok_or_else(|| "decimal exponent arithmetic overflows".to_owned())?;
    if net.unsigned_abs() > MAX_DECIMAL_POWER {
        return Err("decimal exponent exceeds the certified power resource cap".to_owned());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn factored_receipt_digest(
    coefficient_receipt_sha256: [u8; 32],
    coefficient_bits: i128,
    coefficient_binary_exponent2: i32,
    dynamic_formula: &str,
    target_scale_bits: u32,
    terminal_receipt_sha256: [u8; 32],
    terminal_bits: i128,
) -> [u8; 32] {
    fn field(hash: &mut Sha256, tag: u8, bytes: &[u8]) {
        hash.update([tag]);
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }

    let mut hash = Sha256::new();
    field(&mut hash, 1, FACTORED_CERTIFICATE_SCHEMA_ID.as_bytes());
    field(&mut hash, 2, FACTORED_PRODUCER_IMPLEMENTATION_ID.as_bytes());
    field(&mut hash, 3, FACTORED_WATCHDOG_IMPLEMENTATION_ID.as_bytes());
    field(&mut hash, 4, &coefficient_receipt_sha256);
    field(&mut hash, 5, &coefficient_bits.to_le_bytes());
    field(&mut hash, 6, &coefficient_binary_exponent2.to_le_bytes());
    field(&mut hash, 7, dynamic_formula.as_bytes());
    field(&mut hash, 8, &target_scale_bits.to_le_bytes());
    field(&mut hash, 9, &terminal_receipt_sha256);
    field(&mut hash, 10, &terminal_bits.to_le_bytes());
    hash.finalize().into()
}

#[derive(Clone, Debug)]
struct Bounds {
    lower: BigRat,
    upper: BigRat,
}

impl Bounds {
    fn point(value: BigRat) -> Self {
        Self {
            lower: value.clone(),
            upper: value,
        }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            lower: self.lower.add(&other.lower).reduce(),
            upper: self.upper.add(&other.upper).reduce(),
        }
    }

    fn checked_add(&self, other: &Self) -> Result<Self, String> {
        ensure_add_fits(&self.lower, &other.lower)?;
        ensure_add_fits(&self.upper, &other.upper)?;
        Ok(self.add(other))
    }

    fn sub(&self, other: &Self) -> Self {
        Self {
            lower: self.lower.sub(&other.upper).reduce(),
            upper: self.upper.sub(&other.lower).reduce(),
        }
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, String> {
        ensure_add_fits(&self.lower, &other.upper)?;
        ensure_add_fits(&self.upper, &other.lower)?;
        Ok(self.sub(other))
    }

    fn mul(&self, other: &Self) -> Self {
        let products = [
            self.lower.mul(&other.lower).reduce(),
            self.lower.mul(&other.upper).reduce(),
            self.upper.mul(&other.lower).reduce(),
            self.upper.mul(&other.upper).reduce(),
        ];
        let mut lower = products[0].clone();
        let mut upper = products[0].clone();
        for value in products.iter().skip(1) {
            if value.cmp_rat(&lower) == Ordering::Less {
                lower = value.clone();
            }
            if value.cmp_rat(&upper) == Ordering::Greater {
                upper = value.clone();
            }
        }
        Self { lower, upper }
    }

    fn checked_mul(&self, other: &Self) -> Result<Self, String> {
        for left in [&self.lower, &self.upper] {
            for right in [&other.lower, &other.upper] {
                ensure_mul_fits(left, right)?;
            }
        }
        Ok(self.mul(other))
    }

    fn checked_div(&self, other: &Self) -> Result<Self, String> {
        let zero = BigRat::from_i64(0);
        if other.lower.cmp_rat(&zero) != other.upper.cmp_rat(&zero)
            || other.lower.is_zero()
            || other.upper.is_zero()
        {
            return Err("formula divisor interval contains zero".to_owned());
        }
        let reciprocal = Self {
            lower: BigRat::from_i64(1).div(&other.upper).reduce(),
            upper: BigRat::from_i64(1).div(&other.lower).reduce(),
        };
        self.checked_mul(&reciprocal)
    }

    fn negate(&self) -> Self {
        Self {
            lower: self.upper.negate(),
            upper: self.lower.negate(),
        }
    }

    fn checked_pow(&self, exponent: u32) -> Result<Self, String> {
        let mut result = Self::point(BigRat::from_i64(1));
        let mut base = self.clone();
        let mut remaining = exponent;
        while remaining > 0 {
            if remaining & 1 == 1 {
                result = result.checked_mul(&base)?;
            }
            remaining >>= 1;
            if remaining > 0 {
                base = base.checked_mul(&base)?;
            }
        }
        Ok(result)
    }
}

fn ensure_mul_fits(left: &BigRat, right: &BigRat) -> Result<(), String> {
    let (left_num, left_den) = left.component_bit_lengths();
    let (right_num, right_den) = right.component_bit_lengths();
    let largest = (u64::from(left_num) + u64::from(right_num))
        .max(u64::from(left_den) + u64::from(right_den));
    if largest > MAX_INTERMEDIATE_COMPONENT_BITS {
        return Err("formula exceeds the certified intermediate resource cap".to_owned());
    }
    Ok(())
}

fn ensure_add_fits(left: &BigRat, right: &BigRat) -> Result<(), String> {
    let (left_num, left_den) = left.component_bit_lengths();
    let (right_num, right_den) = right.component_bit_lengths();
    let numerator = (u64::from(left_num) + u64::from(right_den))
        .max(u64::from(right_num) + u64::from(left_den))
        .saturating_add(1);
    let denominator = u64::from(left_den) + u64::from(right_den);
    if numerator.max(denominator) > MAX_INTERMEDIATE_COMPONENT_BITS {
        return Err("formula exceeds the certified intermediate resource cap".to_owned());
    }
    Ok(())
}

fn evaluate(
    expression: &Expr,
    inputs: &[ProjectionInput],
    pi_terms: u32,
) -> Result<Bounds, String> {
    match expression {
        Expr::Number(value) => Ok(Bounds::point(value.clone())),
        Expr::Symbol(symbol) if symbol == "pi" => pi_bounds(pi_terms),
        Expr::Symbol(symbol) => {
            let input = inputs
                .iter()
                .find(|input| input.symbol == symbol)
                .ok_or_else(|| format!("formula names undeclared projection input '{symbol}'"))?;
            match input.coordinate {
                super::ProjectionCoordinate::BinaryInterval {
                    lower_bits,
                    upper_bits,
                    exponent2,
                } => {
                    if lower_bits > upper_bits {
                        return Err("projection input interval is reversed".to_owned());
                    }
                    Ok(Bounds {
                        lower: BigRat::from_binary_i128(lower_bits, exponent2),
                        upper: BigRat::from_binary_i128(upper_bits, exponent2),
                    })
                }
                _ => input.producer_rational().map(Bounds::point),
            }
        }
        Expr::Add(left, right) => {
            evaluate(left, inputs, pi_terms)?.checked_add(&evaluate(right, inputs, pi_terms)?)
        }
        Expr::Sub(left, right) => {
            evaluate(left, inputs, pi_terms)?.checked_sub(&evaluate(right, inputs, pi_terms)?)
        }
        Expr::Mul(left, right) => {
            evaluate(left, inputs, pi_terms)?.checked_mul(&evaluate(right, inputs, pi_terms)?)
        }
        Expr::Div(left, right) => {
            evaluate(left, inputs, pi_terms)?.checked_div(&evaluate(right, inputs, pi_terms)?)
        }
        Expr::Pow(value, exponent) => evaluate(value, inputs, pi_terms)?.checked_pow(*exponent),
        Expr::Neg(value) => Ok(evaluate(value, inputs, pi_terms)?.negate()),
    }
}

fn pi_bounds(terms: u32) -> Result<Bounds, String> {
    if terms == 0 {
        return Err("Pi requires at least one alternating-series term".to_owned());
    }
    static FOUR: OnceLock<Bounds> = OnceLock::new();
    static EIGHT: OnceLock<Bounds> = OnceLock::new();
    static SIXTEEN: OnceLock<Bounds> = OnceLock::new();
    static THIRTY_TWO: OnceLock<Bounds> = OnceLock::new();
    static SIXTY_FOUR: OnceLock<Bounds> = OnceLock::new();
    static ONE_TWENTY_EIGHT: OnceLock<Bounds> = OnceLock::new();
    let slot = match terms {
        4 => &FOUR,
        8 => &EIGHT,
        16 => &SIXTEEN,
        32 => &THIRTY_TWO,
        64 => &SIXTY_FOUR,
        128 => &ONE_TWENTY_EIGHT,
        _ => {
            return Err(
                "producer Pi term count is outside the closed refinement schedule".to_owned(),
            )
        }
    };
    Ok(slot.get_or_init(|| compute_pi_bounds(terms)).clone())
}

fn compute_pi_bounds(terms: u32) -> Bounds {
    let five = atan_bounds(5, terms);
    let two_thirty_nine = atan_bounds(239, terms);
    let exact = five
        .mul(&Bounds::point(BigRat::from_i64(16)))
        .sub(&two_thirty_nine.mul(&Bounds::point(BigRat::from_i64(4))));
    outward_dyadic_enclosure(exact)
}

fn outward_dyadic_enclosure(exact: Bounds) -> Bounds {
    let lower_nearest = exact
        .lower
        .round_to_scale(PI_DYADIC_ENCLOSURE_BITS)
        .expect("Pi lower endpoint fits the closed dyadic enclosure");
    let upper_nearest = exact
        .upper
        .round_to_scale(PI_DYADIC_ENCLOSURE_BITS)
        .expect("Pi upper endpoint fits the closed dyadic enclosure");
    Bounds {
        lower: BigRat::from_scaled_i128(
            lower_nearest
                .checked_sub(1)
                .expect("Pi lower enclosure has signed-i128 headroom"),
            PI_DYADIC_ENCLOSURE_BITS,
        ),
        upper: BigRat::from_scaled_i128(
            upper_nearest
                .checked_add(1)
                .expect("Pi upper enclosure has signed-i128 headroom"),
            PI_DYADIC_ENCLOSURE_BITS,
        ),
    }
}

fn atan_bounds(reciprocal: u64, terms: u32) -> Bounds {
    let x = BigUint::from_u64(reciprocal);
    let x_squared = x.mul(&x);
    let mut x_power = x.clone();
    let mut sum = BigRat::from_i64(0);
    for index in 0..terms {
        let denominator = BigUint::from_u64(u64::from(index) * 2 + 1).mul(&x_power);
        let term = BigRat::new(false, BigUint::from_u64(1), denominator);
        sum = if index.is_multiple_of(2) {
            sum.add(&term)
        } else {
            sum.sub(&term)
        }
        .reduce();
        x_power = x_power.mul(&x_squared);
    }
    let next_denominator = BigUint::from_u64(u64::from(terms) * 2 + 1).mul(&x_power);
    let next = BigRat::new(false, BigUint::from_u64(1), next_denominator);
    if terms.is_multiple_of(2) {
        Bounds {
            lower: sum.clone(),
            upper: sum.add(&next).reduce(),
        }
    } else {
        Bounds {
            lower: sum.sub(&next).reduce(),
            upper: sum,
        }
    }
}

fn lex(formula: &str) -> Result<Vec<Token>, String> {
    let bytes = formula.as_bytes();
    let mut tokens = Vec::new();
    let mut position = 0;
    let mut nesting_depth = 0usize;
    let mut sign_run = 0usize;
    while position < bytes.len() {
        match bytes[position] {
            byte if byte.is_ascii_whitespace() => position += 1,
            b'+' => {
                sign_run += 1;
                if sign_run > MAX_SIGN_RUN {
                    return Err("formula exceeds the unary-sign resource cap".to_owned());
                }
                tokens.push(Token::Plus);
                position += 1;
            }
            b'-' => {
                sign_run += 1;
                if sign_run > MAX_SIGN_RUN {
                    return Err("formula exceeds the unary-sign resource cap".to_owned());
                }
                tokens.push(Token::Minus);
                position += 1;
            }
            b'*' => {
                sign_run = 0;
                tokens.push(Token::Star);
                position += 1;
            }
            b'/' => {
                sign_run = 0;
                tokens.push(Token::Slash);
                position += 1;
            }
            b'^' => {
                sign_run = 0;
                tokens.push(Token::Caret);
                position += 1;
            }
            b'(' => {
                sign_run = 0;
                nesting_depth += 1;
                if nesting_depth > MAX_NESTING_DEPTH {
                    return Err("formula exceeds the grouping-depth resource cap".to_owned());
                }
                tokens.push(Token::Left);
                position += 1;
            }
            b')' => {
                sign_run = 0;
                nesting_depth = nesting_depth.saturating_sub(1);
                tokens.push(Token::Right);
                position += 1;
            }
            byte if byte.is_ascii_digit() || byte == b'.' => {
                sign_run = 0;
                let start = position;
                let mut saw_digit = false;
                while position < bytes.len()
                    && (bytes[position].is_ascii_digit() || bytes[position] == b'.')
                {
                    saw_digit |= bytes[position].is_ascii_digit();
                    position += 1;
                }
                if !saw_digit {
                    return Err("numeric literal contains no digit".to_owned());
                }
                if position < bytes.len() && matches!(bytes[position], b'e' | b'E') {
                    position += 1;
                    if position < bytes.len() && matches!(bytes[position], b'+' | b'-') {
                        position += 1;
                    }
                    let exponent_start = position;
                    while position < bytes.len() && bytes[position].is_ascii_digit() {
                        position += 1;
                    }
                    if exponent_start == position {
                        return Err("numeric exponent contains no digit".to_owned());
                    }
                }
                let raw = &formula[start..position];
                validate_decimal_resource(raw)?;
                tokens.push(Token::Number(raw.to_owned()));
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                sign_run = 0;
                let start = position;
                position += 1;
                while position < bytes.len()
                    && (bytes[position].is_ascii_alphanumeric() || bytes[position] == b'_')
                {
                    position += 1;
                }
                tokens.push(Token::Ident(formula[start..position].to_owned()));
            }
            byte => return Err(format!("unexpected formula byte {byte}")),
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
    nodes: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            position: 0,
            nodes: 0,
        }
    }

    fn node(&mut self, expression: Expr) -> Result<Expr, String> {
        self.nodes += 1;
        if self.nodes > MAX_AST_NODES {
            Err("formula exceeds the certified AST resource cap".to_owned())
        } else {
            Ok(expression)
        }
    }

    fn expression(&mut self) -> Result<Expr, String> {
        let mut value = self.term()?;
        loop {
            match self.tokens.get(self.position) {
                Some(Token::Plus) => {
                    self.position += 1;
                    let right = self.term()?;
                    value = self.node(Expr::Add(Box::new(value), Box::new(right)))?;
                }
                Some(Token::Minus) => {
                    self.position += 1;
                    let right = self.term()?;
                    value = self.node(Expr::Sub(Box::new(value), Box::new(right)))?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut value = self.power()?;
        loop {
            match self.tokens.get(self.position) {
                Some(Token::Star) => {
                    self.position += 1;
                    let right = self.power()?;
                    value = self.node(Expr::Mul(Box::new(value), Box::new(right)))?;
                }
                Some(Token::Slash) => {
                    self.position += 1;
                    let right = self.power()?;
                    value = self.node(Expr::Div(Box::new(value), Box::new(right)))?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn power(&mut self) -> Result<Expr, String> {
        let mut value = self.unary()?;
        if matches!(self.tokens.get(self.position), Some(Token::Caret)) {
            self.position += 1;
            let Some(Token::Number(raw)) = self.tokens.get(self.position) else {
                return Err("formula exponent must be an unsigned integer literal".to_owned());
            };
            if !raw.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("formula exponent must be an unsigned integer literal".to_owned());
            }
            let exponent = raw
                .parse::<u32>()
                .map_err(|_| "formula exponent exceeds u32".to_owned())?;
            if exponent > MAX_POWER_EXPONENT {
                return Err("formula exponent exceeds the certified power resource cap".to_owned());
            }
            self.position += 1;
            value = self.node(Expr::Pow(Box::new(value), exponent))?;
        }
        Ok(value)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if matches!(self.tokens.get(self.position), Some(Token::Minus)) {
            self.position += 1;
            let value = self.unary()?;
            return self.node(Expr::Neg(Box::new(value)));
        }
        if matches!(self.tokens.get(self.position), Some(Token::Plus)) {
            self.position += 1;
            return self.unary();
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let token = self
            .tokens
            .get(self.position)
            .ok_or_else(|| "formula ended before a primary expression".to_owned())?
            .clone();
        self.position += 1;
        match token {
            Token::Number(raw) => self.node(Expr::Number(BigRat::from_decimal_str(&raw)?)),
            Token::Ident(symbol) => self.node(Expr::Symbol(symbol)),
            Token::Left => {
                let value = self.expression()?;
                if !matches!(self.tokens.get(self.position), Some(Token::Right)) {
                    return Err("formula has an unmatched left parenthesis".to_owned());
                }
                self.position += 1;
                Ok(value)
            }
            _ => Err("formula primary is malformed".to_owned()),
        }
    }
}
