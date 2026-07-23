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

pub(crate) const CERTIFICATE_SCHEMA_ID: &str = "civsim.units.certified-formula-projection.v3";
pub(crate) const PRODUCER_IMPLEMENTATION_ID: &str =
    "civsim.units.recursive-descent-alternating-interval.v2";
pub(crate) const WATCHDOG_IMPLEMENTATION_ID: &str =
    "civsim.units.shunting-yard-consecutive-interval.v2";
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

#[derive(Debug, Clone)]
pub(crate) struct ProjectionCertificate {
    pub(crate) schema_id: &'static str,
    pub(crate) producer_implementation_id: &'static str,
    pub(crate) watchdog_implementation_id: &'static str,
    pub(crate) formula_sha256: [u8; 32],
    pub(crate) inputs_sha256: [u8; 32],
    pub(crate) target_binary_exponent2: i32,
    pub(crate) pi_terms: u32,
    pub(crate) magnitude_log2: i64,
    pub(crate) lower: BigRat,
    pub(crate) upper: BigRat,
    pub(crate) producer_bits: i128,
    pub(crate) watchdog_bits: i128,
    pub(crate) receipt_sha256: [u8; 32],
}

pub(crate) fn certify_at_scale(
    formula: &str,
    inputs: &[ProjectionInput],
    target_scale_bits: u32,
) -> Result<ProjectionCertificate, String> {
    let mut certificate = producer::produce(formula, inputs, target_scale_bits)?;
    let watchdog_bits = watchdog::verify(&certificate, formula, inputs)?;
    if watchdog_bits != certificate.producer_bits {
        return Err(format!(
            "certified projection disagrees: producer {} watchdog {}",
            certificate.producer_bits, watchdog_bits
        ));
    }
    certificate.watchdog_bits = watchdog_bits;
    certificate.receipt_sha256 = receipt_digest(&certificate);
    Ok(certificate)
}

pub(crate) fn certify_at_significance(
    formula: &str,
    inputs: &[ProjectionInput],
    significant_bits: u32,
) -> Result<ProjectionCertificate, String> {
    let mut certificate = producer::produce_significant(formula, inputs, significant_bits)?;
    let watchdog_bits = watchdog::verify(&certificate, formula, inputs)?;
    if watchdog_bits != certificate.producer_bits {
        return Err(format!(
            "certified coefficient disagrees: producer {} watchdog {}",
            certificate.producer_bits, watchdog_bits
        ));
    }
    certificate.watchdog_bits = watchdog_bits;
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
    hash.update(CERTIFICATE_SCHEMA_ID.as_bytes());
    hash.update(PRODUCER_IMPLEMENTATION_ID.as_bytes());
    hash.update(WATCHDOG_IMPLEMENTATION_ID.as_bytes());
    hash.update(certificate.formula_sha256);
    hash.update(certificate.inputs_sha256);
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
    fn reciprocal_and_cancellation_are_certified() {
        for formula in ["1 / pi^2", "pi - 3"] {
            let certificate = certify_at_scale(formula, &[], 64).unwrap();
            assert_eq!(certificate.producer_bits, certificate.watchdog_bits);
        }
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
        assert!(watchdog::verify(&endpoint, "x / pi", &inputs).is_err());

        let mut scale = original.clone();
        scale.target_binary_exponent2 += 1;
        assert!(watchdog::verify(&scale, "x / pi", &inputs).is_err());

        let mut result = original.clone();
        result.producer_bits += 1;
        assert!(watchdog::verify(&result, "x / pi", &inputs).is_err());

        let changed_inputs = [ProjectionInput::new("x", 4, 1)];
        assert!(watchdog::verify(&original, "x / pi", &changed_inputs).is_err());
        assert!(watchdog::verify(&original, "x * pi", &inputs).is_err());
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
        let error = watchdog::verify(&certificate, "10^65", &[]).unwrap_err();
        assert!(error.contains("power resource cap"));
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
