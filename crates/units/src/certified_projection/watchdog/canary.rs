use super::*;

const CANARY_TRANSCRIPT_SCHEMA_ID: &str = "civsim.units.formula-canary-transcript.watchdog.v2";
const CANARY_SUITE_ID: &str = "civsim.units.formula-canary-suite.shunting-yard-watchdog.v2";

pub(super) fn production_canary_attestation() -> Result<CanaryAttestation, String> {
    static ATTESTATION: OnceLock<Result<CanaryAttestation, String>> = OnceLock::new();
    ATTESTATION.get_or_init(run_production_canary_suite).clone()
}

fn run_production_canary_suite() -> Result<CanaryAttestation, String> {
    let mut transcript = WatchdogCanaryTranscript::new();
    let fixed_zero = ProjectionRequest::FixedScale { scale_bits: 0 };

    for (case_id, formula, expected) in [
        ("unary-below-power", "5 + -2^2", 1),
        ("negative-base-in-parentheses", "(-2)^2", 4),
        ("negated-parenthesized-power", "5 + -(2^2)", 1),
    ] {
        watchdog_canary_expect_projection(
            &mut transcript,
            case_id,
            formula,
            &[],
            fixed_zero,
            expected,
        )?;
    }
    watchdog_canary_expect_projection_at_terms(
        &mut transcript,
        "frozen-pi-scale-80",
        "pi",
        &[],
        ProjectionRequest::FixedScale { scale_bits: 80 },
        32,
        3797952473636338580787994,
    )?;
    let watchdog_multilimb_inputs = [
        ProjectionInput::binary("two240", 1, 240),
        ProjectionInput::binary("two120", 1, 120),
    ];
    watchdog_canary_expect_projection(
        &mut transcript,
        "frozen-multilimb-rational",
        "(two240 + two120) / two120",
        &watchdog_multilimb_inputs,
        fixed_zero,
        1329227995784915872903807060280344577,
    )?;
    let power_baseline = watchdog_canary_build_projection("2^3", &[], fixed_zero)?;
    for (case_id, formula) in [
        ("reject-signed-plus-exponent", "2^+3"),
        ("reject-signed-minus-exponent", "2^-3"),
        ("reject-grouped-exponent", "2^(3)"),
        ("reject-fractional-token-exponent", "2^3.0"),
        ("reject-second-power-operator", "2^3^2"),
    ] {
        let mut certificate = power_baseline.clone();
        certificate.formula_sha256 = formula_digest(formula);
        watchdog_canary_expect_verify_refusal(
            &mut transcript,
            case_id,
            &certificate,
            formula,
            &[],
            fixed_zero,
            false,
        )?;
    }

    let input_set = [ProjectionInput::new("x", 2, 0)];
    let baseline = watchdog_canary_build_projection("x + 1", &input_set, fixed_zero)?;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "substitute-formula",
        &baseline,
        "x + 2",
        &input_set,
        fixed_zero,
        false,
    )?;
    let substituted_input = [ProjectionInput::new("x", 3, 0)];
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "substitute-input",
        &baseline,
        "x + 1",
        &substituted_input,
        fixed_zero,
        false,
    )?;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "substitute-request",
        &baseline,
        "x + 1",
        &input_set,
        ProjectionRequest::FixedScale { scale_bits: 1 },
        false,
    )?;
    let mut mutation = baseline.clone();
    mutation.formula_sha256[0] ^= 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "alter-formula-digest",
        &mutation,
        "x + 1",
        &input_set,
        fixed_zero,
        false,
    )?;
    mutation = baseline.clone();
    mutation.inputs_sha256[0] ^= 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "alter-input-digest",
        &mutation,
        "x + 1",
        &input_set,
        fixed_zero,
        false,
    )?;
    mutation = baseline.clone();
    mutation.request = ProjectionRequest::FixedScale { scale_bits: 1 };
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "alter-certificate-request",
        &mutation,
        "x + 1",
        &input_set,
        fixed_zero,
        false,
    )?;
    mutation = baseline.clone();
    mutation.target_binary_exponent2 += 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "alter-target",
        &mutation,
        "x + 1",
        &input_set,
        fixed_zero,
        false,
    )?;
    mutation = baseline.clone();
    mutation.magnitude_log2 += 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "alter-magnitude-cell",
        &mutation,
        "x + 1",
        &input_set,
        fixed_zero,
        false,
    )?;
    mutation = baseline.clone();
    mutation.lower = BigRat::from_i64(4);
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "alter-lower-endpoint",
        &mutation,
        "x + 1",
        &input_set,
        fixed_zero,
        false,
    )?;
    mutation = baseline.clone();
    mutation.producer_bits += 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "alter-published-result",
        &mutation,
        "x + 1",
        &input_set,
        fixed_zero,
        false,
    )?;
    for (case_id, identity) in [
        ("alter-schema-identity", 0_u8),
        ("alter-producer-identity", 1_u8),
        ("alter-watchdog-identity", 2_u8),
    ] {
        mutation = baseline.clone();
        match identity {
            0 => mutation.schema_id = "civsim.units.watchdog-canary.bad-schema",
            1 => mutation.producer_implementation_id = "civsim.units.watchdog-canary.bad-producer",
            _ => mutation.watchdog_implementation_id = "civsim.units.watchdog-canary.bad-watchdog",
        }
        watchdog_canary_expect_verify_refusal(
            &mut transcript,
            case_id,
            &mutation,
            "x + 1",
            &input_set,
            fixed_zero,
            false,
        )?;
    }

    watchdog_canary_expect_sqrt(&mut transcript, "root-cell-exact-success", "4", &[], 0, 2)?;
    let root_baseline = watchdog_canary_build_sqrt("4", &[], 0)?;
    for (case_id, formula) in [("root-refuse-zero", "0"), ("root-refuse-negative", "-1")] {
        let mut certificate = root_baseline.clone();
        certificate.formula_sha256 = formula_digest(formula);
        watchdog_canary_expect_verify_refusal(
            &mut transcript,
            case_id,
            &certificate,
            formula,
            &[],
            fixed_zero,
            true,
        )?;
    }
    let root_inputs = [ProjectionInput::new("r", 4, 0)];
    let root_input_baseline = watchdog_canary_build_sqrt("r", &root_inputs, 0)?;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-substitute-formula",
        &root_input_baseline,
        "r + 5",
        &root_inputs,
        fixed_zero,
        true,
    )?;
    let changed_root_inputs = [ProjectionInput::new("r", 9, 0)];
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-substitute-input",
        &root_input_baseline,
        "r",
        &changed_root_inputs,
        fixed_zero,
        true,
    )?;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-substitute-request",
        &root_input_baseline,
        "r",
        &root_inputs,
        ProjectionRequest::FixedScale { scale_bits: 1 },
        true,
    )?;
    mutation = root_input_baseline.clone();
    mutation.target_binary_exponent2 += 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-alter-target",
        &mutation,
        "r",
        &root_inputs,
        fixed_zero,
        true,
    )?;
    mutation = root_input_baseline.clone();
    mutation.magnitude_log2 += 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-alter-magnitude",
        &mutation,
        "r",
        &root_inputs,
        fixed_zero,
        true,
    )?;
    mutation = root_input_baseline.clone();
    mutation.upper = BigRat::from_i64(9);
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-alter-endpoint",
        &mutation,
        "r",
        &root_inputs,
        fixed_zero,
        true,
    )?;
    mutation = root_input_baseline.clone();
    mutation.producer_bits += 1;
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-alter-result",
        &mutation,
        "r",
        &root_inputs,
        fixed_zero,
        true,
    )?;
    mutation = root_input_baseline.clone();
    mutation.schema_id = "civsim.units.watchdog-canary.bad-root-schema";
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-alter-schema",
        &mutation,
        "r",
        &root_inputs,
        fixed_zero,
        true,
    )?;
    mutation = root_input_baseline.clone();
    mutation.producer_implementation_id = "civsim.units.watchdog-canary.bad-root-producer";
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-alter-producer-id",
        &mutation,
        "r",
        &root_inputs,
        fixed_zero,
        true,
    )?;
    mutation = root_input_baseline.clone();
    mutation.watchdog_implementation_id = "civsim.units.watchdog-canary.bad-root-watchdog";
    watchdog_canary_expect_verify_refusal(
        &mut transcript,
        "root-alter-watchdog-id",
        &mutation,
        "r",
        &root_inputs,
        fixed_zero,
        true,
    )?;

    let long_formula = "7".repeat(MAX_FORMULA_BYTES + 1);
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-formula-bytes",
        &long_formula,
        &[],
        fixed_zero,
    )?;
    let many_inputs = vec![ProjectionInput::new("z", 1, 0); MAX_INPUTS + 1];
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-input-count",
        "1",
        &many_inputs,
        fixed_zero,
    )?;
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-fixed-scale",
        "1",
        &[],
        ProjectionRequest::FixedScale {
            scale_bits: MAX_COORDINATE_SHIFT_BITS + 1,
        },
    )?;
    let coordinate_overrun = [ProjectionInput::binary(
        "z",
        1,
        (MAX_COORDINATE_SHIFT_BITS + 1) as i32,
    )];
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-coordinate",
        "z",
        &coordinate_overrun,
        fixed_zero,
    )?;
    let decimal_overrun = [ProjectionInput::decimal("z", "1e257")];
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-decimal",
        "z",
        &decimal_overrun,
        fixed_zero,
    )?;
    watchdog_canary_expect_build_refusal(&mut transcript, "limit-power", "10^65", &[], fixed_zero)?;
    let deep_formula = format!(
        "{}1{}",
        "(".repeat(MAX_NESTING_DEPTH + 1),
        ")".repeat(MAX_NESTING_DEPTH + 1)
    );
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-nesting",
        &deep_formula,
        &[],
        fixed_zero,
    )?;
    let sign_formula = format!("{}1", "-".repeat(MAX_SIGN_RUN + 1));
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-sign-run",
        &sign_formula,
        &[],
        fixed_zero,
    )?;
    let large_component = [ProjectionInput::binary("z", i128::MAX, 4096)];
    watchdog_canary_expect_build_refusal(
        &mut transcript,
        "limit-intermediate",
        "z^64",
        &large_component,
        fixed_zero,
    )?;

    let outer_preimage = watchdog_canary_factored_preimage(
        [0x41; 32], 13, -7, "y / 3", 40, [0x52; 32], -11, [0x63; 32], 13, -7, "y / 3", 40,
        [0x52; 32], -11,
    );
    let first = factored_receipt_digest([0x41; 32], 13, -7, "y / 3", 40, [0x52; 32], -11);
    let second = factored_receipt_digest([0x63; 32], 13, -7, "y / 3", 40, [0x52; 32], -11);
    let factored_observation = if first != second {
        Ok(1)
    } else {
        Err("watchdog factored frame missed an outer receipt substitution".to_owned())
    };
    transcript.record(
        "outer-factored-receipt-substitution",
        &outer_preimage,
        &factored_observation,
    );
    factored_observation?;

    Ok(transcript.finish())
}

struct WatchdogCanaryTranscript {
    frame: Vec<u8>,
    case_count: u32,
}

impl WatchdogCanaryTranscript {
    fn new() -> Self {
        let mut frame = Vec::new();
        watchdog_canary_field(&mut frame, 0x81, CANARY_TRANSCRIPT_SCHEMA_ID.as_bytes());
        watchdog_canary_field(&mut frame, 0x82, CANARY_SUITE_ID.as_bytes());
        Self {
            frame,
            case_count: 0,
        }
    }

    fn record(&mut self, case_id: &str, complete_preimage: &[u8], outcome: &Result<i128, String>) {
        self.case_count += 1;
        watchdog_canary_field(&mut self.frame, 0x83, &self.case_count.to_be_bytes());
        watchdog_canary_field(&mut self.frame, 0x84, case_id.as_bytes());
        watchdog_canary_field(&mut self.frame, 0x85, complete_preimage);
        match outcome {
            Ok(bits) => {
                watchdog_canary_field(&mut self.frame, 0x86, &[0xa5]);
                watchdog_canary_field(&mut self.frame, 0x87, &bits.to_be_bytes());
            }
            Err(error) => {
                watchdog_canary_field(&mut self.frame, 0x86, &[0x5a]);
                watchdog_canary_field(&mut self.frame, 0x87, error.as_bytes());
            }
        }
    }

    fn finish(self) -> CanaryAttestation {
        CanaryAttestation {
            suite_schema_id: CANARY_TRANSCRIPT_SCHEMA_ID,
            suite_id: CANARY_SUITE_ID,
            case_count: self.case_count,
            transcript_sha256: Sha256::digest(self.frame).into(),
        }
    }
}

fn watchdog_canary_expect_projection(
    transcript: &mut WatchdogCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    expected: i128,
) -> Result<(), String> {
    watchdog_canary_expect_projection_at_terms(
        transcript, case_id, formula, inputs, request, 0, expected,
    )
}

fn watchdog_canary_expect_projection_at_terms(
    transcript: &mut WatchdogCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    pi_terms: u32,
    expected: i128,
) -> Result<(), String> {
    let certificate =
        watchdog_canary_build_projection_at_terms(formula, inputs, request, pi_terms)?;
    let preimage =
        watchdog_canary_projection_preimage(0, formula, inputs, request, Some(&certificate));
    let observed = verify(&certificate, formula, inputs, request);
    transcript.record(case_id, &preimage, &observed);
    match observed {
        Ok(bits) if bits == expected => Ok(()),
        Ok(bits) => Err(format!(
            "watchdog canary '{case_id}' selected {bits}, expected {expected}"
        )),
        Err(error) => Err(format!("watchdog canary '{case_id}' refused: {error}")),
    }
}

fn watchdog_canary_expect_sqrt(
    transcript: &mut WatchdogCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    scale_bits: u32,
    expected: i128,
) -> Result<(), String> {
    let request = ProjectionRequest::FixedScale { scale_bits };
    let certificate = watchdog_canary_build_sqrt(formula, inputs, scale_bits)?;
    let preimage =
        watchdog_canary_projection_preimage(1, formula, inputs, request, Some(&certificate));
    let observed = verify_positive_sqrt(&certificate, formula, inputs, request);
    transcript.record(case_id, &preimage, &observed);
    match observed {
        Ok(bits) if bits == expected => Ok(()),
        Ok(bits) => Err(format!(
            "watchdog canary '{case_id}' selected root {bits}, expected {expected}"
        )),
        Err(error) => Err(format!("watchdog canary '{case_id}' refused: {error}")),
    }
}

fn watchdog_canary_expect_verify_refusal(
    transcript: &mut WatchdogCanaryTranscript,
    case_id: &str,
    certificate: &ProjectionCertificate,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    positive_sqrt: bool,
) -> Result<(), String> {
    let operation = if positive_sqrt { 3 } else { 2 };
    let preimage =
        watchdog_canary_projection_preimage(operation, formula, inputs, request, Some(certificate));
    let observed = if positive_sqrt {
        verify_positive_sqrt(certificate, formula, inputs, request)
    } else {
        verify(certificate, formula, inputs, request)
    };
    transcript.record(case_id, &preimage, &observed);
    if observed.is_err() {
        Ok(())
    } else {
        Err(format!("watchdog canary '{case_id}' accepted a mutation"))
    }
}

fn watchdog_canary_expect_build_refusal(
    transcript: &mut WatchdogCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
) -> Result<(), String> {
    let preimage = watchdog_canary_projection_preimage(4, formula, inputs, request, None);
    let observed = watchdog_canary_build_projection(formula, inputs, request)
        .map(|certificate| certificate.producer_bits);
    transcript.record(case_id, &preimage, &observed);
    if observed.is_err() {
        Ok(())
    } else {
        Err(format!("watchdog canary '{case_id}' did not refuse"))
    }
}

fn watchdog_canary_build_projection(
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
) -> Result<ProjectionCertificate, String> {
    watchdog_canary_build_projection_at_terms(formula, inputs, request, 0)
}

fn watchdog_canary_build_projection_at_terms(
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    pi_terms: u32,
) -> Result<ProjectionCertificate, String> {
    validate_watchdog_resources(formula, inputs, request)?;
    let lexemes = scan(formula)?;
    if lexemes.len() > MAX_TOKENS {
        return Err("watchdog canary formula exceeds token resource cap".to_owned());
    }
    let rpn = shunting_yard(&lexemes)?;
    let found = evaluate_rpn(&rpn, inputs, pi_terms)?;
    let zero = BigRat::from_i64(0);
    if found.lower.cmp_rat(&zero) != Ordering::Greater
        || found.upper.cmp_rat(&zero) != Ordering::Greater
    {
        return Err("watchdog canary formula lacks a positive magnitude cell".to_owned());
    }
    let lower_log2 = watchdog_floor_log2(&found.lower)?;
    if lower_log2 != watchdog_floor_log2(&found.upper)? {
        return Err("watchdog canary formula spans magnitude cells".to_owned());
    }
    let target_binary_exponent2 = match request {
        ProjectionRequest::FixedScale { scale_bits } => i32::try_from(scale_bits)
            .ok()
            .and_then(i32::checked_neg)
            .ok_or_else(|| "watchdog canary fixed scale exceeds i32".to_owned())?,
        ProjectionRequest::Significant { bits } => {
            let exponent = lower_log2
                .checked_sub(i64::from(bits) - 1)
                .ok_or_else(|| "watchdog canary significance exponent overflows".to_owned())?;
            i32::try_from(exponent)
                .map_err(|_| "watchdog canary significance exponent exceeds i32".to_owned())?
        }
    };
    let lower_bits = watchdog_round_to_binary_exponent(&found.lower, target_binary_exponent2)
        .ok_or_else(|| "watchdog canary lower endpoint exceeds i128".to_owned())?;
    let upper_bits = watchdog_round_to_binary_exponent(&found.upper, target_binary_exponent2)
        .ok_or_else(|| "watchdog canary upper endpoint exceeds i128".to_owned())?;
    if lower_bits != upper_bits {
        return Err("watchdog canary interval spans a rounding cell".to_owned());
    }
    Ok(ProjectionCertificate {
        schema_id: CERTIFICATE_SCHEMA_ID,
        producer_implementation_id: PRODUCER_IMPLEMENTATION_ID,
        watchdog_implementation_id: WATCHDOG_IMPLEMENTATION_ID,
        formula_sha256: formula_digest(formula),
        inputs_sha256: input_digest(inputs),
        request,
        target_binary_exponent2,
        pi_terms,
        magnitude_log2: lower_log2,
        lower: found.lower,
        upper: found.upper,
        producer_bits: lower_bits,
        watchdog_bits: 0,
        producer_canary_attestation: CanaryAttestation::UNATTESTED,
        watchdog_canary_attestation: CanaryAttestation::UNATTESTED,
        receipt_sha256: [0; 32],
    })
}

fn watchdog_canary_build_sqrt(
    formula: &str,
    inputs: &[ProjectionInput],
    scale_bits: u32,
) -> Result<ProjectionCertificate, String> {
    let request = ProjectionRequest::FixedScale { scale_bits };
    validate_watchdog_resources(formula, inputs, request)?;
    let lexemes = scan(formula)?;
    let rpn = shunting_yard(&lexemes)?;
    let found = evaluate_rpn(&rpn, inputs, 0)?;
    let zero = BigRat::from_i64(0);
    if found.lower.cmp_rat(&zero) != Ordering::Greater
        || found.upper.cmp_rat(&zero) != Ordering::Greater
    {
        return Err("watchdog root canary requires a positive radicand".to_owned());
    }
    let mut candidate = 0_u128;
    loop {
        if watchdog_root_cell_contains(&found.lower, candidate, scale_bits)?
            && watchdog_root_cell_contains(&found.upper, candidate, scale_bits)?
        {
            break;
        }
        candidate = candidate
            .checked_add(1)
            .ok_or_else(|| "watchdog root canary candidate overflows".to_owned())?;
        if candidate > 1024 {
            return Err("watchdog root canary fixture exceeds its local search bound".to_owned());
        }
    }
    let producer_bits = i128::try_from(candidate)
        .map_err(|_| "watchdog root canary candidate exceeds i128".to_owned())?;
    let target_binary_exponent2 = i32::try_from(scale_bits)
        .ok()
        .and_then(i32::checked_neg)
        .ok_or_else(|| "watchdog root canary fixed scale exceeds i32".to_owned())?;
    let magnitude_log2 = watchdog_floor_log2(&found.lower)?.div_euclid(2);
    if magnitude_log2 != watchdog_floor_log2(&found.upper)?.div_euclid(2) {
        return Err("watchdog root canary spans magnitude cells".to_owned());
    }
    Ok(ProjectionCertificate {
        schema_id: POSITIVE_SQRT_CERTIFICATE_SCHEMA_ID,
        producer_implementation_id: POSITIVE_SQRT_PRODUCER_IMPLEMENTATION_ID,
        watchdog_implementation_id: POSITIVE_SQRT_WATCHDOG_IMPLEMENTATION_ID,
        formula_sha256: formula_digest(formula),
        inputs_sha256: input_digest(inputs),
        request,
        target_binary_exponent2,
        pi_terms: 0,
        magnitude_log2,
        lower: found.lower,
        upper: found.upper,
        producer_bits,
        watchdog_bits: 0,
        producer_canary_attestation: CanaryAttestation::UNATTESTED,
        watchdog_canary_attestation: CanaryAttestation::UNATTESTED,
        receipt_sha256: [0; 32],
    })
}

fn watchdog_canary_projection_preimage(
    operation: u8,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    certificate: Option<&ProjectionCertificate>,
) -> Vec<u8> {
    let mut frame = Vec::new();
    watchdog_canary_field(&mut frame, 0x10, &[operation]);
    watchdog_canary_field(&mut frame, 0x11, formula.as_bytes());
    watchdog_canary_field(&mut frame, 0x12, &(inputs.len() as u64).to_be_bytes());
    for input in inputs {
        watchdog_canary_field(&mut frame, 0x13, input.symbol.as_bytes());
        match input.coordinate {
            ProjectionCoordinate::Scaled { bits, scale_bits } => {
                watchdog_canary_field(&mut frame, 0x14, &[0x10]);
                watchdog_canary_field(&mut frame, 0x15, &bits.to_be_bytes());
                watchdog_canary_field(&mut frame, 0x16, &scale_bits.to_be_bytes());
            }
            ProjectionCoordinate::Decimal(value) => {
                watchdog_canary_field(&mut frame, 0x14, &[0x20]);
                watchdog_canary_field(&mut frame, 0x15, value.as_bytes());
            }
            ProjectionCoordinate::Binary { bits, exponent2 } => {
                watchdog_canary_field(&mut frame, 0x14, &[0x30]);
                watchdog_canary_field(&mut frame, 0x15, &bits.to_be_bytes());
                watchdog_canary_field(&mut frame, 0x16, &exponent2.to_be_bytes());
            }
            ProjectionCoordinate::BinaryInterval {
                lower_bits,
                upper_bits,
                exponent2,
            } => {
                watchdog_canary_field(&mut frame, 0x14, &[0x40]);
                watchdog_canary_field(&mut frame, 0x15, &lower_bits.to_be_bytes());
                watchdog_canary_field(&mut frame, 0x16, &upper_bits.to_be_bytes());
                watchdog_canary_field(&mut frame, 0x17, &exponent2.to_be_bytes());
            }
        }
    }
    watchdog_canary_encode_request(&mut frame, request);
    match certificate {
        Some(certificate) => {
            watchdog_canary_field(&mut frame, 0x19, &[0xff]);
            watchdog_canary_encode_certificate(&mut frame, certificate);
        }
        None => watchdog_canary_field(&mut frame, 0x19, &[0x00]),
    }
    frame
}

fn watchdog_canary_encode_request(frame: &mut Vec<u8>, request: ProjectionRequest) {
    match request {
        ProjectionRequest::FixedScale { scale_bits } => {
            watchdog_canary_field(frame, 0x18, &[0x71]);
            watchdog_canary_field(frame, 0x1a, &scale_bits.to_be_bytes());
        }
        ProjectionRequest::Significant { bits } => {
            watchdog_canary_field(frame, 0x18, &[0x72]);
            watchdog_canary_field(frame, 0x1a, &bits.to_be_bytes());
        }
    }
}

fn watchdog_canary_encode_certificate(frame: &mut Vec<u8>, certificate: &ProjectionCertificate) {
    watchdog_canary_field(frame, 0x21, certificate.schema_id.as_bytes());
    watchdog_canary_field(
        frame,
        0x22,
        certificate.producer_implementation_id.as_bytes(),
    );
    watchdog_canary_field(
        frame,
        0x23,
        certificate.watchdog_implementation_id.as_bytes(),
    );
    watchdog_canary_field(frame, 0x24, &certificate.formula_sha256);
    watchdog_canary_field(frame, 0x25, &certificate.inputs_sha256);
    watchdog_canary_encode_request(frame, certificate.request);
    watchdog_canary_field(
        frame,
        0x26,
        &certificate.target_binary_exponent2.to_be_bytes(),
    );
    watchdog_canary_field(frame, 0x27, &certificate.pi_terms.to_be_bytes());
    watchdog_canary_field(frame, 0x28, &certificate.magnitude_log2.to_be_bytes());
    watchdog_canary_field(frame, 0x29, &certificate.lower.canonical_bytes());
    watchdog_canary_field(frame, 0x2a, &certificate.upper.canonical_bytes());
    watchdog_canary_field(frame, 0x2b, &certificate.producer_bits.to_be_bytes());
    watchdog_canary_field(frame, 0x2c, &certificate.watchdog_bits.to_be_bytes());
    watchdog_canary_encode_attestation(frame, 0x2d, certificate.producer_canary_attestation);
    watchdog_canary_encode_attestation(frame, 0x2e, certificate.watchdog_canary_attestation);
    watchdog_canary_field(frame, 0x2f, &certificate.receipt_sha256);
}

fn watchdog_canary_encode_attestation(
    frame: &mut Vec<u8>,
    tag: u8,
    attestation: CanaryAttestation,
) {
    let mut record = Vec::new();
    watchdog_canary_field(&mut record, 0x31, attestation.suite_schema_id.as_bytes());
    watchdog_canary_field(&mut record, 0x32, attestation.suite_id.as_bytes());
    watchdog_canary_field(&mut record, 0x33, &attestation.case_count.to_be_bytes());
    watchdog_canary_field(&mut record, 0x34, &attestation.transcript_sha256);
    watchdog_canary_field(frame, tag, &record);
}

#[allow(clippy::too_many_arguments)]
fn watchdog_canary_factored_preimage(
    first_coefficient_receipt: [u8; 32],
    first_coefficient_bits: i128,
    first_coefficient_exponent: i32,
    first_formula: &str,
    first_scale: u32,
    first_terminal_receipt: [u8; 32],
    first_terminal_bits: i128,
    second_coefficient_receipt: [u8; 32],
    second_coefficient_bits: i128,
    second_coefficient_exponent: i32,
    second_formula: &str,
    second_scale: u32,
    second_terminal_receipt: [u8; 32],
    second_terminal_bits: i128,
) -> Vec<u8> {
    let mut frame = Vec::new();
    watchdog_canary_field(&mut frame, 0x41, &first_coefficient_receipt);
    watchdog_canary_field(&mut frame, 0x42, &first_coefficient_bits.to_be_bytes());
    watchdog_canary_field(&mut frame, 0x43, &first_coefficient_exponent.to_be_bytes());
    watchdog_canary_field(&mut frame, 0x44, first_formula.as_bytes());
    watchdog_canary_field(&mut frame, 0x45, &first_scale.to_be_bytes());
    watchdog_canary_field(&mut frame, 0x46, &first_terminal_receipt);
    watchdog_canary_field(&mut frame, 0x47, &first_terminal_bits.to_be_bytes());
    watchdog_canary_field(&mut frame, 0x48, &second_coefficient_receipt);
    watchdog_canary_field(&mut frame, 0x49, &second_coefficient_bits.to_be_bytes());
    watchdog_canary_field(&mut frame, 0x4a, &second_coefficient_exponent.to_be_bytes());
    watchdog_canary_field(&mut frame, 0x4b, second_formula.as_bytes());
    watchdog_canary_field(&mut frame, 0x4c, &second_scale.to_be_bytes());
    watchdog_canary_field(&mut frame, 0x4d, &second_terminal_receipt);
    watchdog_canary_field(&mut frame, 0x4e, &second_terminal_bits.to_be_bytes());
    frame
}

fn watchdog_canary_field(frame: &mut Vec<u8>, tag: u8, bytes: &[u8]) {
    frame.push(tag);
    frame.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    frame.extend_from_slice(bytes);
}
