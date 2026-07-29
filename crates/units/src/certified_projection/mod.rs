//! Exact interval certification for authority-bearing SI projections.
//!
//! The producer and watchdog share only canonical formula bytes, scaled input
//! integers, exact rational primitives, and SHA-256. They use separate parsers,
//! interval implementations, and Machin-series loops.

mod producer;
mod watchdog;

use crate::bignum::BigRat;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

pub(crate) const CERTIFICATE_SCHEMA_ID: &str = "civsim.units.certified-formula-projection.v5";
pub(crate) const PRODUCER_IMPLEMENTATION_ID: &str =
    "civsim.units.recursive-descent-alternating-interval.v5";
pub(crate) const WATCHDOG_IMPLEMENTATION_ID: &str =
    "civsim.units.shunting-yard-consecutive-interval.v6";
pub(crate) const POSITIVE_SQRT_CERTIFICATE_SCHEMA_ID: &str =
    "civsim.units.certified-positive-sqrt-formula-projection.v2";
pub(crate) const POSITIVE_SQRT_PRODUCER_IMPLEMENTATION_ID: &str =
    "civsim.units.recursive-descent-binary-search-root-floor.v2";
pub(crate) const POSITIVE_SQRT_WATCHDOG_IMPLEMENTATION_ID: &str =
    "civsim.units.shunting-yard-squared-root-cell.v2";
pub(crate) const FACTORED_CERTIFICATE_SCHEMA_ID: &str =
    "civsim.units.certified-factored-formula-projection.v2";
pub(crate) const FACTORED_PRODUCER_IMPLEMENTATION_ID: &str =
    "civsim.units.factored-binding-stream.v1";
pub(crate) const FACTORED_WATCHDOG_IMPLEMENTATION_ID: &str =
    "civsim.units.factored-binding-frame.v1";

#[derive(Debug, Clone)]
pub(crate) struct ProjectionInput {
    pub(crate) symbol: &'static str,
    pub(crate) coordinate: ProjectionCoordinate,
}

#[derive(Debug, Clone)]
pub(crate) enum ProjectionCoordinate {
    Scaled {
        bits: i128,
        scale_bits: u32,
    },
    Binary {
        bits: i128,
        exponent2: i32,
    },
    BinaryInterval {
        lower_bits: i128,
        upper_bits: i128,
        exponent2: i32,
    },
    Decimal(&'static str),
}

impl ProjectionInput {
    pub(crate) const fn new(symbol: &'static str, bits: i128, scale_bits: u32) -> Self {
        Self {
            symbol,
            coordinate: ProjectionCoordinate::Scaled { bits, scale_bits },
        }
    }

    pub(crate) const fn decimal(symbol: &'static str, value: &'static str) -> Self {
        Self {
            symbol,
            coordinate: ProjectionCoordinate::Decimal(value),
        }
    }

    pub(crate) const fn binary(symbol: &'static str, bits: i128, exponent2: i32) -> Self {
        Self {
            symbol,
            coordinate: ProjectionCoordinate::Binary { bits, exponent2 },
        }
    }

    pub(crate) const fn binary_interval(
        symbol: &'static str,
        lower_bits: i128,
        upper_bits: i128,
        exponent2: i32,
    ) -> Self {
        Self {
            symbol,
            coordinate: ProjectionCoordinate::BinaryInterval {
                lower_bits,
                upper_bits,
                exponent2,
            },
        }
    }

    pub(crate) fn producer_rational(&self) -> Result<BigRat, String> {
        match self.coordinate {
            ProjectionCoordinate::Scaled { bits, scale_bits } => {
                Ok(BigRat::from_scaled_i128(bits, scale_bits))
            }
            ProjectionCoordinate::Binary { bits, exponent2 } => {
                Ok(BigRat::from_binary_i128(bits, exponent2))
            }
            ProjectionCoordinate::BinaryInterval { .. } => {
                Err("interval projection input cannot be resolved as a point".to_owned())
            }
            ProjectionCoordinate::Decimal(value) => BigRat::from_decimal_str(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionRequest {
    FixedScale { scale_bits: u32 },
    Significant { bits: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanaryAttestation {
    pub(crate) suite_schema_id: &'static str,
    pub(crate) suite_id: &'static str,
    pub(crate) case_count: u32,
    pub(crate) transcript_sha256: [u8; 32],
}

impl CanaryAttestation {
    const UNATTESTED: Self = Self {
        suite_schema_id: "",
        suite_id: "",
        case_count: 0,
        transcript_sha256: [0; 32],
    };
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectionCertificate {
    pub(crate) schema_id: &'static str,
    pub(crate) producer_implementation_id: &'static str,
    pub(crate) watchdog_implementation_id: &'static str,
    pub(crate) formula_sha256: [u8; 32],
    pub(crate) inputs_sha256: [u8; 32],
    pub(crate) request: ProjectionRequest,
    pub(crate) target_binary_exponent2: i32,
    pub(crate) pi_terms: u32,
    pub(crate) magnitude_log2: i64,
    pub(crate) lower: BigRat,
    pub(crate) upper: BigRat,
    pub(crate) producer_bits: i128,
    pub(crate) watchdog_bits: i128,
    pub(crate) producer_canary_attestation: CanaryAttestation,
    pub(crate) watchdog_canary_attestation: CanaryAttestation,
    pub(crate) receipt_sha256: [u8; 32],
}

fn attach_production_canary_attestations(
    certificate: &mut ProjectionCertificate,
) -> Result<(), String> {
    let producer_attestation = producer::production_canary_attestation()?;
    let watchdog_attestation = watchdog::production_canary_attestation()?;
    if producer_attestation == CanaryAttestation::UNATTESTED
        || watchdog_attestation == CanaryAttestation::UNATTESTED
    {
        return Err("formula authority canary suite returned an empty attestation".to_owned());
    }
    certificate.producer_canary_attestation = producer_attestation;
    certificate.watchdog_canary_attestation = watchdog_attestation;
    Ok(())
}

pub(crate) fn certify_at_scale(
    formula: &str,
    inputs: &[ProjectionInput],
    target_scale_bits: u32,
) -> Result<ProjectionCertificate, String> {
    let request = ProjectionRequest::FixedScale {
        scale_bits: target_scale_bits,
    };
    let mut certificate = producer::produce(formula, inputs, target_scale_bits)?;
    let watchdog_bits = watchdog::verify(&certificate, formula, inputs, request)?;
    if watchdog_bits != certificate.producer_bits {
        return Err(format!(
            "certified projection disagrees: producer {} watchdog {}",
            certificate.producer_bits, watchdog_bits
        ));
    }
    certificate.watchdog_bits = watchdog_bits;
    attach_production_canary_attestations(&mut certificate)?;
    certificate.receipt_sha256 = receipt_digest(&certificate);
    Ok(certificate)
}

pub(crate) fn certify_at_significance(
    formula: &str,
    inputs: &[ProjectionInput],
    significant_bits: u32,
) -> Result<ProjectionCertificate, String> {
    let request = ProjectionRequest::Significant {
        bits: significant_bits,
    };
    let mut certificate = producer::produce_significant(formula, inputs, significant_bits)?;
    let watchdog_bits = watchdog::verify(&certificate, formula, inputs, request)?;
    if watchdog_bits != certificate.producer_bits {
        return Err(format!(
            "certified coefficient disagrees: producer {} watchdog {}",
            certificate.producer_bits, watchdog_bits
        ));
    }
    certificate.watchdog_bits = watchdog_bits;
    attach_production_canary_attestations(&mut certificate)?;
    certificate.receipt_sha256 = receipt_digest(&certificate);
    Ok(certificate)
}

/// Certify the fixed-scale floor of the positive square root of one complete
/// formula.
///
/// The producer selects the terminal integer directly from its exact radicand
/// interval. The watchdog independently proves that the candidate's squared
/// fixed-point cell contains both independently evaluated endpoints. No
/// fixed-point radicand or unaudited square-root implementation enters this
/// boundary.
pub(crate) fn certify_positive_sqrt_at_scale(
    radicand_formula: &str,
    inputs: &[ProjectionInput],
    target_scale_bits: u32,
) -> Result<ProjectionCertificate, String> {
    let request = ProjectionRequest::FixedScale {
        scale_bits: target_scale_bits,
    };
    let mut certificate =
        producer::produce_positive_sqrt(radicand_formula, inputs, target_scale_bits)?;
    let watchdog_bits =
        watchdog::verify_positive_sqrt(&certificate, radicand_formula, inputs, request)?;
    if watchdog_bits != certificate.producer_bits {
        return Err(format!(
            "certified positive square root disagrees: producer {} watchdog {}",
            certificate.producer_bits, watchdog_bits
        ));
    }
    certificate.watchdog_bits = watchdog_bits;
    attach_production_canary_attestations(&mut certificate)?;
    certificate.receipt_sha256 = receipt_digest(&certificate);
    Ok(certificate)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn factored_receipt_digest(
    coefficient_receipt_sha256: [u8; 32],
    coefficient_bits: i128,
    coefficient_binary_exponent2: i32,
    dynamic_formula: &str,
    target_scale_bits: u32,
    terminal_receipt_sha256: [u8; 32],
    terminal_bits: i128,
) -> Result<[u8; 32], String> {
    let produced = producer::factored_receipt_digest(
        coefficient_receipt_sha256,
        coefficient_bits,
        coefficient_binary_exponent2,
        dynamic_formula,
        target_scale_bits,
        terminal_receipt_sha256,
        terminal_bits,
    );
    let checked = watchdog::factored_receipt_digest(
        coefficient_receipt_sha256,
        coefficient_bits,
        coefficient_binary_exponent2,
        dynamic_formula,
        target_scale_bits,
        terminal_receipt_sha256,
        terminal_bits,
    );
    if produced != checked {
        return Err("factored projection receipt implementations disagree".to_owned());
    }
    Ok(produced)
}

pub(crate) fn certified_decimal(value: &str) -> Result<BigRat, String> {
    let checked = watchdog::parse_decimal_exact(value)?;
    let producer = BigRat::from_decimal_str(value)?;
    if producer.cmp_rat(&checked) != Ordering::Equal {
        return Err(format!(
            "independent decimal parsers disagree for '{value}'"
        ));
    }
    Ok(producer)
}

pub(crate) fn certified_decimal_ulp(value: &str) -> Result<BigRat, String> {
    let checked = watchdog::decimal_ulp_exact(value)?;
    let producer = BigRat::decimal_ulp(value)?;
    if producer.cmp_rat(&checked) != Ordering::Equal {
        return Err(format!(
            "independent decimal ULP parsers disagree for '{value}'"
        ));
    }
    Ok(producer)
}

pub(crate) fn interval_has_stable_magnitude(certificate: &ProjectionCertificate) -> bool {
    certificate.lower.cmp_rat(&BigRat::from_i64(0)) == Ordering::Greater
        && certificate.upper.cmp_rat(&BigRat::from_i64(0)) == Ordering::Greater
        && certificate.lower.floor_log2() == certificate.upper.floor_log2()
        && certificate.lower.floor_log2() == certificate.magnitude_log2
}

fn formula_digest(formula: &str) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update((formula.len() as u64).to_le_bytes());
    hash.update(formula.as_bytes());
    hash.finalize().into()
}

fn input_digest(inputs: &[ProjectionInput]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update((inputs.len() as u64).to_le_bytes());
    for input in inputs {
        hash.update((input.symbol.len() as u64).to_le_bytes());
        hash.update(input.symbol.as_bytes());
        match input.coordinate {
            ProjectionCoordinate::Scaled { bits, scale_bits } => {
                hash.update([0]);
                hash.update(bits.to_le_bytes());
                hash.update(scale_bits.to_le_bytes());
            }
            ProjectionCoordinate::Binary { bits, exponent2 } => {
                hash.update([2]);
                hash.update(bits.to_le_bytes());
                hash.update(exponent2.to_le_bytes());
            }
            ProjectionCoordinate::BinaryInterval {
                lower_bits,
                upper_bits,
                exponent2,
            } => {
                hash.update([3]);
                hash.update(lower_bits.to_le_bytes());
                hash.update(upper_bits.to_le_bytes());
                hash.update(exponent2.to_le_bytes());
            }
            ProjectionCoordinate::Decimal(value) => {
                hash.update([1]);
                hash.update((value.len() as u64).to_le_bytes());
                hash.update(value.as_bytes());
            }
        }
    }
    hash.finalize().into()
}

fn receipt_digest(certificate: &ProjectionCertificate) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(certificate.schema_id.as_bytes());
    hash.update(certificate.producer_implementation_id.as_bytes());
    hash.update(certificate.watchdog_implementation_id.as_bytes());
    hash.update(certificate.formula_sha256);
    hash.update(certificate.inputs_sha256);
    match certificate.request {
        ProjectionRequest::FixedScale { scale_bits } => {
            hash.update([0]);
            hash.update(scale_bits.to_le_bytes());
        }
        ProjectionRequest::Significant { bits } => {
            hash.update([1]);
            hash.update(bits.to_le_bytes());
        }
    }
    hash.update(certificate.target_binary_exponent2.to_le_bytes());
    hash.update(certificate.pi_terms.to_le_bytes());
    hash.update(certificate.magnitude_log2.to_le_bytes());
    for endpoint in [&certificate.lower, &certificate.upper] {
        let bytes = endpoint.canonical_bytes();
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    hash.update(certificate.producer_bits.to_le_bytes());
    hash.update(certificate.watchdog_bits.to_le_bytes());
    for attestation in [
        certificate.producer_canary_attestation,
        certificate.watchdog_canary_attestation,
    ] {
        hash.update((attestation.suite_schema_id.len() as u64).to_le_bytes());
        hash.update(attestation.suite_schema_id.as_bytes());
        hash.update((attestation.suite_id.len() as u64).to_le_bytes());
        hash.update(attestation.suite_id.as_bytes());
        hash.update(attestation.case_count.to_le_bytes());
        hash.update(attestation.transcript_sha256);
    }
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_fifth_power_is_certified_to_q32_32() {
        let certificate = certify_at_scale("2 * pi^5 / 15", &[], 32).unwrap();
        assert_eq!(certificate.producer_bits, certificate.watchdog_bits);
        assert!(interval_has_stable_magnitude(&certificate));
        assert_ne!(certificate.receipt_sha256, [0; 32]);
    }

    #[test]
    fn issued_formula_certificates_carry_both_cached_production_canary_transcripts() {
        let raw = producer::produce("3", &[], 0).unwrap();
        assert_eq!(
            raw.producer_canary_attestation,
            CanaryAttestation::UNATTESTED
        );
        assert_eq!(
            raw.watchdog_canary_attestation,
            CanaryAttestation::UNATTESTED
        );

        let issued = certify_at_scale("3", &[], 0).unwrap();
        let producer_attestation = producer::production_canary_attestation().unwrap();
        let watchdog_attestation = watchdog::production_canary_attestation().unwrap();
        assert_eq!(issued.producer_canary_attestation, producer_attestation);
        assert_eq!(issued.watchdog_canary_attestation, watchdog_attestation);
        assert!(producer_attestation.case_count >= 40);
        assert!(watchdog_attestation.case_count >= 40);
        assert_ne!(
            producer_attestation.suite_schema_id,
            watchdog_attestation.suite_schema_id
        );
        assert_ne!(producer_attestation.suite_id, watchdog_attestation.suite_id);
        assert_ne!(
            producer_attestation.transcript_sha256,
            watchdog_attestation.transcript_sha256
        );
        assert_eq!(
            producer::production_canary_attestation().unwrap(),
            producer_attestation
        );
        assert_eq!(
            watchdog::production_canary_attestation().unwrap(),
            watchdog_attestation
        );

        let significant = certify_at_significance("3", &[], 16).unwrap();
        assert_eq!(
            significant.producer_canary_attestation,
            producer_attestation
        );
        assert_eq!(
            significant.watchdog_canary_attestation,
            watchdog_attestation
        );

        let sqrt = certify_positive_sqrt_at_scale("4", &[], 0).unwrap();
        assert_eq!(sqrt.producer_canary_attestation, producer_attestation);
        assert_eq!(sqrt.watchdog_canary_attestation, watchdog_attestation);
    }

    #[test]
    fn formula_receipt_binds_each_production_canary_attestation() {
        let certificate = certify_at_scale("3", &[], 0).unwrap();

        let mut producer_mutation = certificate.clone();
        producer_mutation
            .producer_canary_attestation
            .transcript_sha256[0] ^= 1;
        assert_ne!(
            receipt_digest(&producer_mutation),
            certificate.receipt_sha256
        );

        let mut watchdog_mutation = certificate.clone();
        watchdog_mutation.watchdog_canary_attestation.case_count += 1;
        assert_ne!(
            receipt_digest(&watchdog_mutation),
            certificate.receipt_sha256
        );
    }

    #[test]
    fn reciprocal_and_cancellation_are_certified() {
        for formula in ["1 / pi^2", "pi - 3"] {
            let certificate = certify_at_scale(formula, &[], 64).unwrap();
            assert_eq!(certificate.producer_bits, certificate.watchdog_bits);
        }
    }

    #[test]
    fn unary_signs_have_lower_precedence_than_power_in_both_parsers() {
        for (formula, expected) in [
            ("5 + -2^2", 1),
            ("(-2)^2", 4),
            ("5 + -(2^2)", 1),
            ("7 + 2*-3", 1),
            ("5 + ---2", 3),
        ] {
            let certificate = certify_at_scale(formula, &[], 0).unwrap();
            assert_eq!(certificate.producer_bits, expected, "{formula}");
            assert_eq!(certificate.watchdog_bits, expected, "{formula}");
        }
    }

    #[test]
    fn power_exponents_require_one_unsigned_literal_in_both_parsers() {
        for formula in ["2^+3", "2^-3", "2^(3)", "2^3.0", "2^3^2"] {
            assert!(producer::produce(formula, &[], 0).is_err(), "{formula}");

            let mut certificate = producer::produce("2^3", &[], 0).unwrap();
            certificate.formula_sha256 = formula_digest(formula);
            let error = watchdog::verify(
                &certificate,
                formula,
                &[],
                ProjectionRequest::FixedScale { scale_bits: 0 },
            )
            .unwrap_err();
            assert!(
                error.contains("unsigned integer literal"),
                "{formula}: {error}"
            );
        }
    }

    #[test]
    fn positive_sqrt_is_certified_without_an_intermediate_fixed_projection() {
        let exact_square = certify_positive_sqrt_at_scale("4", &[], 32).unwrap();
        assert_eq!(exact_square.producer_bits, 2_i128 << 32);
        assert_eq!(exact_square.schema_id, POSITIVE_SQRT_CERTIFICATE_SCHEMA_ID);
        assert_eq!(
            exact_square.producer_implementation_id,
            POSITIVE_SQRT_PRODUCER_IMPLEMENTATION_ID
        );
        assert_eq!(
            exact_square.watchdog_implementation_id,
            POSITIVE_SQRT_WATCHDOG_IMPLEMENTATION_ID
        );
        assert_ne!(exact_square.receipt_sha256, [0; 32]);

        // Rounding 3.75 to an integer before taking its root would select 2.
        // The complete-expression floor is floor(sqrt(3.75)) = 1.
        let no_double_round = certify_positive_sqrt_at_scale("15 / 4", &[], 0).unwrap();
        assert_eq!(no_double_round.producer_bits, 1);
        assert_eq!(no_double_round.producer_bits, no_double_round.watchdog_bits);
    }

    #[test]
    fn positive_sqrt_refuses_a_nonpositive_or_boundary_crossing_radicand() {
        for formula in ["0", "-1"] {
            assert!(certify_positive_sqrt_at_scale(formula, &[], 32).is_err());
        }

        let crossing = [ProjectionInput::binary_interval("x", 15, 17, -2)];
        let error = certify_positive_sqrt_at_scale("x", &crossing, 0).unwrap_err();
        assert!(error.contains("root cell"));

        assert!(certify_positive_sqrt_at_scale("4", &[], 4097)
            .unwrap_err()
            .contains("resource cap"));
    }

    #[test]
    fn positive_sqrt_watchdog_convicts_certificate_mutations() {
        let inputs = [ProjectionInput::new("x", 4, 0)];
        let original = producer::produce_positive_sqrt("x", &inputs, 32).unwrap();
        let request = ProjectionRequest::FixedScale { scale_bits: 32 };

        let mut endpoint = original.clone();
        endpoint.lower = BigRat::from_i64(5);
        assert!(watchdog::verify_positive_sqrt(&endpoint, "x", &inputs, request).is_err());

        let mut scale = original.clone();
        scale.target_binary_exponent2 += 1;
        assert!(watchdog::verify_positive_sqrt(&scale, "x", &inputs, request).is_err());

        let mut result = original.clone();
        result.producer_bits += 1;
        assert!(watchdog::verify_positive_sqrt(&result, "x", &inputs, request).is_err());

        let changed_inputs = [ProjectionInput::new("x", 9, 0)];
        assert!(watchdog::verify_positive_sqrt(&original, "x", &changed_inputs, request).is_err());
        assert!(watchdog::verify_positive_sqrt(&original, "x + 1", &inputs, request).is_err());
        assert!(watchdog::verify(&original, "x", &inputs, request).is_err());
    }

    #[test]
    fn zero_crossing_denominator_refuses() {
        let error = certify_at_scale("1 / (pi - pi)", &[], 32).unwrap_err();
        assert!(error.contains("zero"));
    }

    #[test]
    fn halfway_rounding_is_exact_and_even() {
        assert_eq!(certify_at_scale("1 / 2", &[], 0).unwrap().producer_bits, 0);
        assert_eq!(certify_at_scale("3 / 2", &[], 0).unwrap().producer_bits, 2);
    }

    #[test]
    fn watchdog_rejects_a_false_exact_midpoint_selection() {
        let request = ProjectionRequest::FixedScale { scale_bits: 0 };
        let correct = producer::produce("1 / 2", &[], 0).unwrap();
        assert_eq!(watchdog::verify(&correct, "1 / 2", &[], request), Ok(0));

        let mut false_midpoint = correct;
        false_midpoint.producer_bits = 1;
        let error = watchdog::verify(&false_midpoint, "1 / 2", &[], request).unwrap_err();
        assert!(error.contains("published integer"));
    }

    #[test]
    fn watchdog_rounding_covers_negative_midpoints_and_even_parity() {
        for (numerator, expected) in [
            (-1_i64, 0_i128),
            (-3_i64, -2_i128),
            (-5_i64, -2_i128),
            (-7_i64, -4_i128),
        ] {
            let value = BigRat::new(
                true,
                crate::bignum::BigUint::from_u64(numerator.unsigned_abs()),
                crate::bignum::BigUint::from_u64(2),
            );
            assert_eq!(
                watchdog::watchdog_round_to_binary_exponent(&value, 0),
                Some(expected)
            );
        }

        let min = BigRat::from_binary_i128(i128::MIN, 0);
        assert_eq!(
            watchdog::watchdog_round_to_binary_exponent(&min, 0),
            Some(i128::MIN)
        );
        let below_min = BigRat::new(
            true,
            crate::bignum::BigUint::from_u64(1)
                .shl_bits(127)
                .add(&crate::bignum::BigUint::from_u64(1)),
            crate::bignum::BigUint::from_u64(1),
        );
        assert_eq!(
            watchdog::watchdog_round_to_binary_exponent(&below_min, 0),
            None
        );
    }

    #[test]
    fn watchdog_rounding_covers_positive_exponent_midpoint_sides_and_overflow() {
        for (numerator, denominator, expected) in [
            (5_u64, 2_u64, 1_i128),
            (3_u64, 1_u64, 2_i128),
            (7_u64, 2_u64, 2_i128),
            (5_u64, 1_u64, 2_i128),
        ] {
            let value = BigRat::new(
                false,
                crate::bignum::BigUint::from_u64(numerator),
                crate::bignum::BigUint::from_u64(denominator),
            );
            assert_eq!(
                watchdog::watchdog_round_to_binary_exponent(&value, 1),
                Some(expected)
            );
        }

        let overflow = BigRat::new(
            false,
            crate::bignum::BigUint::from_u64(1).shl_bits(128),
            crate::bignum::BigUint::from_u64(1),
        );
        assert_eq!(
            watchdog::watchdog_round_to_binary_exponent(&overflow, 1),
            None
        );
    }

    #[test]
    fn watchdog_dyadic_floor_and_ceil_are_exact_outward_enclosures() {
        let exact_positive = BigRat::new(
            false,
            crate::bignum::BigUint::from_u64(3),
            crate::bignum::BigUint::from_u64(2),
        );
        let exact_negative = exact_positive.negate();
        assert_eq!(watchdog::watchdog_dyadic_floor(&exact_positive, 1), Some(3));
        assert_eq!(watchdog::watchdog_dyadic_ceil(&exact_positive, 1), Some(3));
        assert_eq!(
            watchdog::watchdog_dyadic_floor(&exact_negative, 1),
            Some(-3)
        );
        assert_eq!(watchdog::watchdog_dyadic_ceil(&exact_negative, 1), Some(-3));

        let non_dyadic_positive = BigRat::new(
            false,
            crate::bignum::BigUint::from_u64(5),
            crate::bignum::BigUint::from_u64(4),
        );
        let non_dyadic_negative = non_dyadic_positive.negate();
        assert_eq!(
            watchdog::watchdog_dyadic_floor(&non_dyadic_positive, 1),
            Some(2)
        );
        assert_eq!(
            watchdog::watchdog_dyadic_ceil(&non_dyadic_positive, 1),
            Some(3)
        );
        assert_eq!(
            watchdog::watchdog_dyadic_floor(&non_dyadic_negative, 1),
            Some(-3)
        );
        assert_eq!(
            watchdog::watchdog_dyadic_ceil(&non_dyadic_negative, 1),
            Some(-2)
        );
    }

    #[test]
    fn watchdog_proves_power_of_two_cells_below_at_and_above_the_boundary() {
        let target_log2 = 7_i64;
        let denominator_bits = 11_u32;
        let exact_numerator =
            crate::bignum::BigUint::from_u64(1).shl_bits(target_log2 as u32 + denominator_bits);
        let denominator = crate::bignum::BigUint::from_u64(1).shl_bits(denominator_bits);
        let one = crate::bignum::BigUint::from_u64(1);
        let below = BigRat::new(false, exact_numerator.sub(&one), denominator.clone());
        let exact = BigRat::new(false, exact_numerator.clone(), denominator.clone());
        let above = BigRat::new(false, exact_numerator.add(&one), denominator);

        assert_eq!(watchdog::watchdog_floor_log2(&below), Ok(target_log2 - 1));
        assert_eq!(watchdog::watchdog_floor_log2(&exact), Ok(target_log2));
        assert_eq!(watchdog::watchdog_floor_log2(&above), Ok(target_log2));
    }

    #[test]
    fn watchdog_rejects_a_forged_power_of_two_magnitude_bracket() {
        let request = ProjectionRequest::Significant { bits: 8 };
        let correct = producer::produce_significant("1024", &[], 8).unwrap();
        assert_eq!(watchdog::verify(&correct, "1024", &[], request), Ok(128));

        let mut forged = correct;
        forged.magnitude_log2 += 1;
        forged.target_binary_exponent2 += 1;
        let error = watchdog::verify(&forged, "1024", &[], request).unwrap_err();
        assert!(error.contains("magnitude bracket"));
    }

    #[test]
    fn independent_decimal_paths_agree() {
        for value in ["299792458", "1.380649e-23", "+2.50E3", "-0.125"] {
            assert_eq!(
                certified_decimal(value)
                    .unwrap()
                    .cmp_rat(&BigRat::from_decimal_str(value).unwrap()),
                Ordering::Equal
            );
            assert_eq!(
                certified_decimal_ulp(value)
                    .unwrap()
                    .cmp_rat(&BigRat::decimal_ulp(value).unwrap()),
                Ordering::Equal
            );
        }
    }

    #[test]
    fn certificate_mutations_are_convicted_by_the_watchdog() {
        let inputs = [ProjectionInput::new("x", 3, 1)];
        let original = producer::produce("x / pi", &inputs, 32).unwrap();

        let mut endpoint = original.clone();
        endpoint.lower = BigRat::from_i64(0);
        assert!(watchdog::verify(&endpoint, "x / pi", &inputs, original.request).is_err());

        let mut scale = original.clone();
        scale.target_binary_exponent2 += 1;
        assert!(watchdog::verify(&scale, "x / pi", &inputs, original.request).is_err());

        let mut result = original.clone();
        result.producer_bits += 1;
        assert!(watchdog::verify(&result, "x / pi", &inputs, original.request).is_err());

        let changed_inputs = [ProjectionInput::new("x", 4, 1)];
        assert!(watchdog::verify(&original, "x / pi", &changed_inputs, original.request).is_err());
        assert!(watchdog::verify(&original, "x * pi", &inputs, original.request).is_err());
    }

    #[test]
    fn formula_resource_cap_refuses_instead_of_selecting_a_value() {
        let oversized = std::iter::repeat_n("1", 130)
            .collect::<Vec<_>>()
            .join(" + ");
        let error = certify_at_scale(&oversized, &[], 32).unwrap_err();
        assert!(error.contains("resource cap"));
    }

    #[test]
    fn coordinate_and_power_resource_caps_refuse_before_allocation() {
        let tiny = [ProjectionInput::binary("x", 1, -4096)];
        assert_eq!(certify_at_scale("x", &tiny, 4096).unwrap().producer_bits, 1);
        assert!(certify_at_scale("10^27", &[], 0).is_ok());

        let too_wide = [ProjectionInput::new("x", 1, 4097)];
        assert!(certify_at_scale("x", &too_wide, 32)
            .unwrap_err()
            .contains("resource cap"));
        let too_large = [ProjectionInput::binary("x", 1, 4097)];
        assert!(certify_at_scale("x", &too_large, 32)
            .unwrap_err()
            .contains("resource cap"));
        let decimal = [ProjectionInput::decimal("x", "1e257")];
        assert!(certify_at_scale("x", &decimal, 32)
            .unwrap_err()
            .contains("resource cap"));
        assert!(certify_at_scale("1e257", &[], 32)
            .unwrap_err()
            .contains("resource cap"));
        assert!(certify_at_scale("10^65", &[], 32)
            .unwrap_err()
            .contains("resource cap"));
        let nested = format!("{}1{}", "(".repeat(65), ")".repeat(65));
        assert!(certify_at_scale(&nested, &[], 32)
            .unwrap_err()
            .contains("resource cap"));
        let signed = format!("{}1", "-".repeat(65));
        assert!(certify_at_scale(&signed, &[], 32)
            .unwrap_err()
            .contains("resource cap"));

        let huge_intermediate = [ProjectionInput::binary("x", i128::MAX, 4096)];
        assert!(certify_at_scale("x^64", &huge_intermediate, 0)
            .unwrap_err()
            .contains("intermediate resource cap"));
    }

    #[test]
    fn watchdog_independently_refuses_an_over_cap_power() {
        let mut certificate = producer::produce("10^2", &[], 32).unwrap();
        certificate.formula_sha256 = formula_digest("10^65");
        let error = watchdog::verify(
            &certificate,
            "10^65",
            &[],
            ProjectionRequest::FixedScale { scale_bits: 32 },
        )
        .unwrap_err();
        assert!(error.contains("power resource cap"));
    }

    #[test]
    fn watchdog_refuses_a_fixed_scale_certificate_for_another_request() {
        let certificate = producer::produce("2 * pi^5 / 15", &[], 31).unwrap();
        let error = watchdog::verify(
            &certificate,
            "2 * pi^5 / 15",
            &[],
            ProjectionRequest::FixedScale { scale_bits: 32 },
        )
        .unwrap_err();
        assert!(error.contains("request"));
    }

    #[test]
    fn watchdog_refuses_an_independently_wrong_significance_request() {
        let certificate = producer::produce_significant("2 * pi^5 / 15", &[], 32).unwrap();
        let error = watchdog::verify(
            &certificate,
            "2 * pi^5 / 15",
            &[],
            ProjectionRequest::Significant { bits: 31 },
        )
        .unwrap_err();
        assert!(error.contains("request"));
    }

    #[test]
    fn watchdog_recomputes_the_significance_target_exponent() {
        let mut certificate = producer::produce_significant("2 * pi^5 / 15", &[], 32).unwrap();
        certificate.target_binary_exponent2 += 1;
        let error = watchdog::verify(
            &certificate,
            "2 * pi^5 / 15",
            &[],
            ProjectionRequest::Significant { bits: 32 },
        )
        .unwrap_err();
        assert!(error.contains("target exponent"));
    }

    #[test]
    fn equal_projection_bits_keep_fixed_and_significant_receipts_distinct() {
        let fixed = certify_at_scale("1", &[], 32).unwrap();
        let significant = certify_at_significance("1", &[], 33).unwrap();
        assert_eq!(
            fixed.target_binary_exponent2,
            significant.target_binary_exponent2
        );
        assert_eq!(fixed.producer_bits, significant.producer_bits);
        assert_ne!(fixed.request, significant.request);
        assert_ne!(fixed.receipt_sha256, significant.receipt_sha256);
    }

    #[test]
    fn factored_receipt_pair_binds_every_outer_field() {
        let baseline = factored_receipt_digest([1; 32], 7, -4, "x + 1", 32, [2; 32], 9).unwrap();
        let mutations = [
            factored_receipt_digest([3; 32], 7, -4, "x + 1", 32, [2; 32], 9).unwrap(),
            factored_receipt_digest([1; 32], 8, -4, "x + 1", 32, [2; 32], 9).unwrap(),
            factored_receipt_digest([1; 32], 7, -3, "x + 1", 32, [2; 32], 9).unwrap(),
            factored_receipt_digest([1; 32], 7, -4, "x + 2", 32, [2; 32], 9).unwrap(),
            factored_receipt_digest([1; 32], 7, -4, "x + 1", 33, [2; 32], 9).unwrap(),
            factored_receipt_digest([1; 32], 7, -4, "x + 1", 32, [4; 32], 9).unwrap(),
            factored_receipt_digest([1; 32], 7, -4, "x + 1", 32, [2; 32], 10).unwrap(),
        ];
        assert!(mutations.into_iter().all(|receipt| receipt != baseline));
    }
}
