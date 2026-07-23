use super::{
    formula_digest, input_digest, ProjectionCertificate, ProjectionCoordinate, ProjectionInput,
    CERTIFICATE_SCHEMA_ID, FACTORED_CERTIFICATE_SCHEMA_ID, FACTORED_PRODUCER_IMPLEMENTATION_ID,
    FACTORED_WATCHDOG_IMPLEMENTATION_ID, PRODUCER_IMPLEMENTATION_ID, WATCHDOG_IMPLEMENTATION_ID,
};
use crate::bignum::{BigRat, BigUint};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::sync::OnceLock;

const MAX_TOKENS: usize = 256;
const MAX_FORMULA_BYTES: usize = 4096;
const MAX_INPUTS: usize = 128;
const MAX_SYMBOL_BYTES: usize = 64;
const MAX_DECIMAL_BYTES: usize = 256;
const MAX_DECIMAL_DIGITS: usize = 64;
const MAX_DECIMAL_POWER: u64 = 256;
const MAX_POWER_EXPONENT: u32 = 64;
const MAX_COORDINATE_SHIFT_BITS: u32 = 4096;
const MAX_INTERMEDIATE_COMPONENT_BITS: u64 = 65_536;
const MAX_NESTING_DEPTH: usize = 64;
const MAX_SIGN_RUN: usize = 64;
// Independently widen the watchdog's consecutive-partial-sum interval onto a
// compact dyadic grid. This avoids propagating giant Machin denominators
// through every runtime formula while retaining a strict outer enclosure.
const PI_DYADIC_ENCLOSURE_BITS: u32 = 120;

#[derive(Clone, Debug)]
enum Lexeme {
    Number(String),
    Symbol(String),
    Operator(Operator),
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Negate,
}

impl Operator {
    const fn precedence(self) -> u8 {
        match self {
            Self::Add | Self::Subtract => 1,
            Self::Multiply | Self::Divide => 2,
            Self::Negate => 3,
            Self::Power => 4,
        }
    }

    const fn right_associative(self) -> bool {
        matches!(self, Self::Power | Self::Negate)
    }
}

#[derive(Clone, Debug)]
enum Rpn {
    Number(String),
    Symbol(String),
    Operator(Operator),
}

pub(super) fn verify(
    certificate: &ProjectionCertificate,
    formula: &str,
    inputs: &[ProjectionInput],
) -> Result<i128, String> {
    validate_watchdog_resources(formula, inputs, certificate.target_binary_exponent2)?;
    if certificate.schema_id != CERTIFICATE_SCHEMA_ID
        || certificate.producer_implementation_id != PRODUCER_IMPLEMENTATION_ID
        || certificate.watchdog_implementation_id != WATCHDOG_IMPLEMENTATION_ID
    {
        return Err("projection certificate implementation identity differs".to_owned());
    }
    if certificate.formula_sha256 != formula_digest(formula)
        || certificate.inputs_sha256 != input_digest(inputs)
    {
        return Err("projection certificate input digest differs".to_owned());
    }
    let lexemes = scan(formula)?;
    if lexemes.len() > MAX_TOKENS {
        return Err("watchdog formula exceeds token resource cap".to_owned());
    }
    let rpn = shunting_yard(&lexemes)?;
    let found = evaluate_rpn(&rpn, inputs, certificate.pi_terms)?;
    if certificate.lower.cmp_rat(&found.lower) == Ordering::Greater
        || certificate.upper.cmp_rat(&found.upper) == Ordering::Less
    {
        return Err(
            "producer interval does not contain the independent watchdog interval".to_owned(),
        );
    }
    let zero = BigRat::from_i64(0);
    if found.lower.cmp_rat(&zero) != Ordering::Greater
        || found.upper.cmp_rat(&zero) != Ordering::Greater
        || found.lower.floor_log2() != found.upper.floor_log2()
        || found.lower.floor_log2() != certificate.magnitude_log2
    {
        return Err("watchdog could not reproduce one positive magnitude bracket".to_owned());
    }
    let lower_bits = found
        .lower
        .round_to_binary_exponent(certificate.target_binary_exponent2)
        .ok_or_else(|| "watchdog lower endpoint exceeds i128".to_owned())?;
    let upper_bits = found
        .upper
        .round_to_binary_exponent(certificate.target_binary_exponent2)
        .ok_or_else(|| "watchdog upper endpoint exceeds i128".to_owned())?;
    if lower_bits != upper_bits {
        return Err("watchdog interval straddles a rounding boundary".to_owned());
    }
    for endpoint in [&certificate.lower, &certificate.upper] {
        if endpoint.round_to_binary_exponent(certificate.target_binary_exponent2)
            != Some(certificate.producer_bits)
        {
            return Err("producer endpoint does not certify its published integer".to_owned());
        }
    }
    Ok(lower_bits)
}

fn validate_watchdog_resources(
    formula: &str,
    inputs: &[ProjectionInput],
    target_binary_exponent2: i32,
) -> Result<(), String> {
    if formula.len() > MAX_FORMULA_BYTES {
        return Err("watchdog formula exceeds the byte resource cap".to_owned());
    }
    if inputs.len() > MAX_INPUTS {
        return Err("watchdog input count exceeds the resource cap".to_owned());
    }
    if target_binary_exponent2.unsigned_abs() > MAX_COORDINATE_SHIFT_BITS {
        return Err("watchdog target exponent exceeds the resource cap".to_owned());
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
            return Err("watchdog projection symbol is invalid or reserved".to_owned());
        }
        if inputs[..index]
            .iter()
            .any(|prior| prior.symbol == input.symbol)
        {
            return Err("watchdog projection symbol is duplicated".to_owned());
        }
        match input.coordinate {
            ProjectionCoordinate::Scaled { scale_bits, .. }
                if scale_bits > MAX_COORDINATE_SHIFT_BITS =>
            {
                return Err("watchdog scaled input exceeds the coordinate resource cap".to_owned())
            }
            ProjectionCoordinate::Binary { exponent2, .. }
                if exponent2.unsigned_abs() > MAX_COORDINATE_SHIFT_BITS =>
            {
                return Err("watchdog binary input exceeds the coordinate resource cap".to_owned())
            }
            ProjectionCoordinate::BinaryInterval {
                lower_bits,
                upper_bits,
                exponent2,
            } => {
                if lower_bits > upper_bits {
                    return Err("watchdog projection input interval is reversed".to_owned());
                }
                if exponent2.unsigned_abs() > MAX_COORDINATE_SHIFT_BITS {
                    return Err(
                        "watchdog interval input exceeds the coordinate resource cap".to_owned(),
                    );
                }
            }
            ProjectionCoordinate::Decimal(value) => {
                validate_watchdog_decimal_resource(value)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_watchdog_decimal_resource(value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.len() > MAX_DECIMAL_BYTES {
        return Err("watchdog decimal exceeds the byte resource cap".to_owned());
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
        .map_err(|_| "watchdog decimal exponent is malformed".to_owned())?
        .unwrap_or(0);
    if exponent_split.next().is_some() {
        return Err("watchdog decimal contains multiple exponents".to_owned());
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
        return Err("watchdog decimal mantissa is malformed".to_owned());
    }
    if integer.len() + fraction.len() > MAX_DECIMAL_DIGITS {
        return Err("watchdog decimal exceeds the digit resource cap".to_owned());
    }
    let fractional_places = i64::try_from(fraction.len())
        .map_err(|_| "watchdog decimal fractional length exceeds i64".to_owned())?;
    let net = exponent
        .checked_sub(fractional_places)
        .ok_or_else(|| "watchdog decimal exponent arithmetic overflows".to_owned())?;
    if net.unsigned_abs() > MAX_DECIMAL_POWER {
        return Err("watchdog decimal exponent exceeds the power resource cap".to_owned());
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
    fn append_field(frame: &mut Vec<u8>, tag: u8, bytes: &[u8]) {
        frame.push(tag);
        frame.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        frame.extend_from_slice(bytes);
    }

    let mut frame = Vec::new();
    append_field(&mut frame, 1, FACTORED_CERTIFICATE_SCHEMA_ID.as_bytes());
    append_field(
        &mut frame,
        2,
        FACTORED_PRODUCER_IMPLEMENTATION_ID.as_bytes(),
    );
    append_field(
        &mut frame,
        3,
        FACTORED_WATCHDOG_IMPLEMENTATION_ID.as_bytes(),
    );
    append_field(&mut frame, 4, &coefficient_receipt_sha256);
    append_field(&mut frame, 5, &coefficient_bits.to_le_bytes());
    append_field(&mut frame, 6, &coefficient_binary_exponent2.to_le_bytes());
    append_field(&mut frame, 7, dynamic_formula.as_bytes());
    append_field(&mut frame, 8, &target_scale_bits.to_le_bytes());
    append_field(&mut frame, 9, &terminal_receipt_sha256);
    append_field(&mut frame, 10, &terminal_bits.to_le_bytes());
    Sha256::digest(frame).into()
}

#[derive(Clone, Debug)]
struct WatchdogBounds {
    lower: BigRat,
    upper: BigRat,
}

impl WatchdogBounds {
    fn exact(value: BigRat) -> Self {
        Self {
            lower: value.clone(),
            upper: value,
        }
    }

    fn plus(self, right: Self) -> Self {
        Self {
            lower: self.lower.add(&right.lower).reduce(),
            upper: self.upper.add(&right.upper).reduce(),
        }
    }

    fn checked_plus(self, right: Self) -> Result<Self, String> {
        watchdog_add_fits(&self.lower, &right.lower)?;
        watchdog_add_fits(&self.upper, &right.upper)?;
        Ok(self.plus(right))
    }

    fn minus(self, right: Self) -> Self {
        Self {
            lower: self.lower.sub(&right.upper).reduce(),
            upper: self.upper.sub(&right.lower).reduce(),
        }
    }

    fn checked_minus(self, right: Self) -> Result<Self, String> {
        watchdog_add_fits(&self.lower, &right.upper)?;
        watchdog_add_fits(&self.upper, &right.lower)?;
        Ok(self.minus(right))
    }

    fn times(self, right: Self) -> Self {
        let candidates = [
            self.lower.mul(&right.lower).reduce(),
            self.lower.mul(&right.upper).reduce(),
            self.upper.mul(&right.lower).reduce(),
            self.upper.mul(&right.upper).reduce(),
        ];
        let lower = candidates
            .iter()
            .min_by(|left, right| left.cmp_rat(right))
            .expect("four interval products")
            .clone();
        let upper = candidates
            .iter()
            .max_by(|left, right| left.cmp_rat(right))
            .expect("four interval products")
            .clone();
        Self { lower, upper }
    }

    fn checked_times(self, right: Self) -> Result<Self, String> {
        for left in [&self.lower, &self.upper] {
            for candidate in [&right.lower, &right.upper] {
                watchdog_mul_fits(left, candidate)?;
            }
        }
        Ok(self.times(right))
    }

    fn checked_divided_by(self, right: Self) -> Result<Self, String> {
        let zero = BigRat::from_i64(0);
        let lower_side = right.lower.cmp_rat(&zero);
        let upper_side = right.upper.cmp_rat(&zero);
        if lower_side != upper_side || lower_side == Ordering::Equal {
            return Err("watchdog divisor interval contains zero".to_owned());
        }
        let inverse = Self {
            lower: BigRat::from_i64(1).div(&right.upper).reduce(),
            upper: BigRat::from_i64(1).div(&right.lower).reduce(),
        };
        self.checked_times(inverse)
    }

    fn negative(self) -> Self {
        Self {
            lower: self.upper.negate(),
            upper: self.lower.negate(),
        }
    }

    fn checked_raised(self, exponent: u32) -> Result<Self, String> {
        let mut output = Self::exact(BigRat::from_i64(1));
        for _ in 0..exponent {
            output = output.checked_times(self.clone())?;
        }
        Ok(output)
    }
}

fn watchdog_mul_fits(left: &BigRat, right: &BigRat) -> Result<(), String> {
    let (left_num, left_den) = left.component_bit_lengths();
    let (right_num, right_den) = right.component_bit_lengths();
    let largest = (u64::from(left_num) + u64::from(right_num))
        .max(u64::from(left_den) + u64::from(right_den));
    if largest > MAX_INTERMEDIATE_COMPONENT_BITS {
        return Err("watchdog formula exceeds the intermediate resource cap".to_owned());
    }
    Ok(())
}

fn watchdog_add_fits(left: &BigRat, right: &BigRat) -> Result<(), String> {
    let (left_num, left_den) = left.component_bit_lengths();
    let (right_num, right_den) = right.component_bit_lengths();
    let numerator = (u64::from(left_num) + u64::from(right_den))
        .max(u64::from(right_num) + u64::from(left_den))
        .saturating_add(1);
    let denominator = u64::from(left_den) + u64::from(right_den);
    if numerator.max(denominator) > MAX_INTERMEDIATE_COMPONENT_BITS {
        return Err("watchdog formula exceeds the intermediate resource cap".to_owned());
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct StackValue {
    bounds: WatchdogBounds,
    unsigned_literal: Option<u32>,
}

fn evaluate_rpn(
    rpn: &[Rpn],
    inputs: &[ProjectionInput],
    pi_terms: u32,
) -> Result<WatchdogBounds, String> {
    let mut stack: Vec<StackValue> = Vec::new();
    for item in rpn {
        match item {
            Rpn::Number(raw) => stack.push(StackValue {
                bounds: WatchdogBounds::exact(parse_decimal_exact(raw)?),
                unsigned_literal: raw.parse::<u32>().ok(),
            }),
            Rpn::Symbol(symbol) if symbol == "pi" => stack.push(StackValue {
                bounds: pi_interval(pi_terms)?,
                unsigned_literal: None,
            }),
            Rpn::Symbol(symbol) => {
                let input = inputs
                    .iter()
                    .find(|input| input.symbol == symbol)
                    .ok_or_else(|| format!("watchdog names undeclared input '{symbol}'"))?;
                let value = match input.coordinate {
                    ProjectionCoordinate::Scaled { bits, scale_bits } => {
                        BigRat::from_scaled_i128(bits, scale_bits)
                    }
                    ProjectionCoordinate::Binary { bits, exponent2 } => {
                        BigRat::from_binary_i128(bits, exponent2)
                    }
                    ProjectionCoordinate::BinaryInterval {
                        lower_bits,
                        upper_bits,
                        exponent2,
                    } => {
                        if lower_bits > upper_bits {
                            return Err("watchdog projection input interval is reversed".to_owned());
                        }
                        stack.push(StackValue {
                            bounds: WatchdogBounds {
                                lower: BigRat::from_binary_i128(lower_bits, exponent2),
                                upper: BigRat::from_binary_i128(upper_bits, exponent2),
                            },
                            unsigned_literal: None,
                        });
                        continue;
                    }
                    ProjectionCoordinate::Decimal(value) => parse_decimal_exact(value)?,
                };
                stack.push(StackValue {
                    bounds: WatchdogBounds::exact(value),
                    unsigned_literal: None,
                });
            }
            Rpn::Operator(Operator::Negate) => {
                let value = stack
                    .pop()
                    .ok_or_else(|| "watchdog unary stack underflow".to_owned())?;
                stack.push(StackValue {
                    bounds: value.bounds.negative(),
                    unsigned_literal: None,
                });
            }
            Rpn::Operator(operator) => {
                let right = stack
                    .pop()
                    .ok_or_else(|| "watchdog binary stack underflow".to_owned())?;
                let left = stack
                    .pop()
                    .ok_or_else(|| "watchdog binary stack underflow".to_owned())?;
                let bounds = match operator {
                    Operator::Add => left.bounds.checked_plus(right.bounds)?,
                    Operator::Subtract => left.bounds.checked_minus(right.bounds)?,
                    Operator::Multiply => left.bounds.checked_times(right.bounds)?,
                    Operator::Divide => left.bounds.checked_divided_by(right.bounds)?,
                    Operator::Power => {
                        let exponent = right.unsigned_literal.ok_or_else(|| {
                            "watchdog exponent is not an unsigned integer literal".to_owned()
                        })?;
                        if exponent > MAX_POWER_EXPONENT {
                            return Err(
                                "watchdog exponent exceeds the power resource cap".to_owned()
                            );
                        }
                        left.bounds.checked_raised(exponent)?
                    }
                    Operator::Negate => unreachable!(),
                };
                stack.push(StackValue {
                    bounds,
                    unsigned_literal: None,
                });
            }
        }
    }
    if stack.len() != 1 {
        return Err("watchdog formula did not reduce to one value".to_owned());
    }
    Ok(stack.pop().expect("one checked stack value").bounds)
}

fn pi_interval(terms: u32) -> Result<WatchdogBounds, String> {
    if terms == 0 {
        return Err("watchdog Pi term count is zero".to_owned());
    }
    static FOUR: OnceLock<WatchdogBounds> = OnceLock::new();
    static EIGHT: OnceLock<WatchdogBounds> = OnceLock::new();
    static SIXTEEN: OnceLock<WatchdogBounds> = OnceLock::new();
    static THIRTY_TWO: OnceLock<WatchdogBounds> = OnceLock::new();
    static SIXTY_FOUR: OnceLock<WatchdogBounds> = OnceLock::new();
    static ONE_TWENTY_EIGHT: OnceLock<WatchdogBounds> = OnceLock::new();
    let slot = match terms {
        4 => &FOUR,
        8 => &EIGHT,
        16 => &SIXTEEN,
        32 => &THIRTY_TWO,
        64 => &SIXTY_FOUR,
        128 => &ONE_TWENTY_EIGHT,
        _ => {
            return Err(
                "watchdog Pi term count is outside the closed refinement schedule".to_owned(),
            )
        }
    };
    Ok(slot.get_or_init(|| compute_pi_interval(terms)).clone())
}

fn compute_pi_interval(terms: u32) -> WatchdogBounds {
    let five_first = atan_partial(5, terms);
    let five_second = atan_partial(5, terms + 1);
    let two_thirty_nine_first = atan_partial(239, terms);
    let two_thirty_nine_second = atan_partial(239, terms + 1);
    let five = ordered(five_first, five_second);
    let two_thirty_nine = ordered(two_thirty_nine_first, two_thirty_nine_second);
    let exact = five
        .times(WatchdogBounds::exact(BigRat::from_i64(16)))
        .minus(two_thirty_nine.times(WatchdogBounds::exact(BigRat::from_i64(4))));
    widen_to_dyadic_grid(exact)
}

fn widen_to_dyadic_grid(exact: WatchdogBounds) -> WatchdogBounds {
    let low = exact
        .lower
        .round_to_scale(PI_DYADIC_ENCLOSURE_BITS)
        .expect("watchdog Pi lower endpoint fits its dyadic grid");
    let high = exact
        .upper
        .round_to_scale(PI_DYADIC_ENCLOSURE_BITS)
        .expect("watchdog Pi upper endpoint fits its dyadic grid");
    WatchdogBounds {
        lower: BigRat::from_scaled_i128(
            low.checked_sub(1)
                .expect("watchdog Pi lower grid has signed-i128 headroom"),
            PI_DYADIC_ENCLOSURE_BITS,
        ),
        upper: BigRat::from_scaled_i128(
            high.checked_add(1)
                .expect("watchdog Pi upper grid has signed-i128 headroom"),
            PI_DYADIC_ENCLOSURE_BITS,
        ),
    }
}

fn ordered(first: BigRat, second: BigRat) -> WatchdogBounds {
    if first.cmp_rat(&second) == Ordering::Greater {
        WatchdogBounds {
            lower: second,
            upper: first,
        }
    } else {
        WatchdogBounds {
            lower: first,
            upper: second,
        }
    }
}

fn atan_partial(reciprocal: u64, terms: u32) -> BigRat {
    let mut sum = BigRat::from_i64(0);
    for index in 0..terms {
        let power = BigUint::from_u64(reciprocal).pow(index * 2 + 1);
        let denominator = BigUint::from_u64(u64::from(index) * 2 + 1).mul(&power);
        let term = BigRat::new(false, BigUint::from_u64(1), denominator);
        sum = if index.is_multiple_of(2) {
            sum.add(&term)
        } else {
            sum.sub(&term)
        }
        .reduce();
    }
    sum
}

fn shunting_yard(lexemes: &[Lexeme]) -> Result<Vec<Rpn>, String> {
    let mut output = Vec::new();
    let mut operators: Vec<Lexeme> = Vec::new();
    let mut expect_operand = true;
    for lexeme in lexemes {
        match lexeme {
            Lexeme::Number(value) => {
                output.push(Rpn::Number(value.clone()));
                expect_operand = false;
            }
            Lexeme::Symbol(value) => {
                output.push(Rpn::Symbol(value.clone()));
                expect_operand = false;
            }
            Lexeme::Left => {
                operators.push(Lexeme::Left);
                expect_operand = true;
            }
            Lexeme::Right => {
                while !matches!(operators.last(), Some(Lexeme::Left) | None) {
                    move_operator(&mut operators, &mut output)?;
                }
                if !matches!(operators.pop(), Some(Lexeme::Left)) {
                    return Err("watchdog formula has unmatched right parenthesis".to_owned());
                }
                expect_operand = false;
            }
            Lexeme::Operator(mut operator) => {
                if expect_operand {
                    operator = match operator {
                        Operator::Subtract => Operator::Negate,
                        Operator::Add => continue,
                        _ => {
                            return Err("watchdog formula has misplaced binary operator".to_owned())
                        }
                    };
                }
                while let Some(Lexeme::Operator(top)) = operators.last() {
                    let should_pop = top.precedence() > operator.precedence()
                        || (top.precedence() == operator.precedence()
                            && !operator.right_associative());
                    if !should_pop {
                        break;
                    }
                    move_operator(&mut operators, &mut output)?;
                }
                operators.push(Lexeme::Operator(operator));
                expect_operand = true;
            }
        }
    }
    if expect_operand && !lexemes.is_empty() {
        return Err("watchdog formula ends with an operator".to_owned());
    }
    while !operators.is_empty() {
        if matches!(operators.last(), Some(Lexeme::Left)) {
            return Err("watchdog formula has unmatched left parenthesis".to_owned());
        }
        move_operator(&mut operators, &mut output)?;
    }
    Ok(output)
}

fn move_operator(operators: &mut Vec<Lexeme>, output: &mut Vec<Rpn>) -> Result<(), String> {
    match operators.pop() {
        Some(Lexeme::Operator(operator)) => {
            output.push(Rpn::Operator(operator));
            Ok(())
        }
        _ => Err("watchdog operator stack is malformed".to_owned()),
    }
}

fn scan(formula: &str) -> Result<Vec<Lexeme>, String> {
    let bytes = formula.as_bytes();
    let mut output = Vec::new();
    let mut cursor = 0;
    let mut nesting_depth = 0usize;
    let mut sign_run = 0usize;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        let operator = match byte {
            b'+' => Some(Operator::Add),
            b'-' => Some(Operator::Subtract),
            b'*' => Some(Operator::Multiply),
            b'/' => Some(Operator::Divide),
            b'^' => Some(Operator::Power),
            _ => None,
        };
        if let Some(operator) = operator {
            if matches!(operator, Operator::Add | Operator::Subtract) {
                sign_run += 1;
                if sign_run > MAX_SIGN_RUN {
                    return Err("watchdog formula exceeds the unary-sign resource cap".to_owned());
                }
            } else {
                sign_run = 0;
            }
            output.push(Lexeme::Operator(operator));
            cursor += 1;
            continue;
        }
        if byte == b'(' || byte == b')' {
            sign_run = 0;
            if byte == b'(' {
                nesting_depth += 1;
                if nesting_depth > MAX_NESTING_DEPTH {
                    return Err(
                        "watchdog formula exceeds the grouping-depth resource cap".to_owned()
                    );
                }
            } else {
                nesting_depth = nesting_depth.saturating_sub(1);
            }
            output.push(if byte == b'(' {
                Lexeme::Left
            } else {
                Lexeme::Right
            });
            cursor += 1;
            continue;
        }
        if byte.is_ascii_digit() || byte == b'.' {
            sign_run = 0;
            let start = cursor;
            while cursor < bytes.len() && (bytes[cursor].is_ascii_digit() || bytes[cursor] == b'.')
            {
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] == b'e' || bytes[cursor] == b'E') {
                cursor += 1;
                if cursor < bytes.len() && (bytes[cursor] == b'+' || bytes[cursor] == b'-') {
                    cursor += 1;
                }
                let exponent_start = cursor;
                while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                if exponent_start == cursor {
                    return Err("watchdog numeric exponent has no digits".to_owned());
                }
            }
            let raw = formula[start..cursor].to_owned();
            parse_decimal_exact(&raw)?;
            output.push(Lexeme::Number(raw));
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            sign_run = 0;
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            output.push(Lexeme::Symbol(formula[start..cursor].to_owned()));
            continue;
        }
        return Err(format!("watchdog formula contains byte {byte}"));
    }
    Ok(output)
}

pub(super) fn parse_decimal_exact(value: &str) -> Result<BigRat, String> {
    validate_watchdog_decimal_resource(value)?;
    let (negative, digits, fractional_places, exponent) = decimal_parts(value)?;
    let mut numerator = BigUint::from_u64(0);
    let ten = BigUint::from_u64(10);
    for digit in digits.bytes() {
        numerator = numerator
            .mul(&ten)
            .add(&BigUint::from_u64(u64::from(digit - b'0')));
    }
    let net = exponent - fractional_places;
    if net >= 0 {
        Ok(BigRat::new(
            negative,
            numerator.mul(&BigUint::ten_pow(net as u32)),
            BigUint::from_u64(1),
        ))
    } else {
        Ok(BigRat::new(
            negative,
            numerator,
            BigUint::ten_pow((-net) as u32),
        ))
    }
}

pub(super) fn decimal_ulp_exact(value: &str) -> Result<BigRat, String> {
    validate_watchdog_decimal_resource(value)?;
    let (_, _, fractional_places, exponent) = decimal_parts(value)?;
    let net = exponent - fractional_places;
    if net >= 0 {
        Ok(BigRat::new(
            false,
            BigUint::ten_pow(net as u32),
            BigUint::from_u64(1),
        ))
    } else {
        Ok(BigRat::new(
            false,
            BigUint::from_u64(1),
            BigUint::ten_pow((-net) as u32),
        ))
    }
}

fn decimal_parts(value: &str) -> Result<(bool, String, i64, i64), String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("watchdog decimal is empty".to_owned());
    }
    let (negative, unsigned) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let mut split = unsigned.split(['e', 'E']);
    let mantissa = split.next().unwrap_or_default();
    let exponent = split
        .next()
        .map(|raw| {
            raw.parse::<i64>()
                .map_err(|_| "watchdog decimal exponent is malformed".to_owned())
        })
        .transpose()?
        .unwrap_or(0);
    if split.next().is_some() {
        return Err("watchdog decimal contains multiple exponents".to_owned());
    }
    let mut points = mantissa.split('.');
    let integer = points.next().unwrap_or_default();
    let fraction = points.next().unwrap_or_default();
    if points.next().is_some() {
        return Err("watchdog decimal contains multiple points".to_owned());
    }
    let digits = format!("{integer}{fraction}");
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("watchdog decimal mantissa is malformed".to_owned());
    }
    Ok((negative, digits, fraction.len() as i64, exponent))
}
