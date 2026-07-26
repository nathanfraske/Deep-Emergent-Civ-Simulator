use super::*;

const CANARY_TRANSCRIPT_SCHEMA_ID: &str = "civsim.units.formula-canary-transcript.producer.v2";
const CANARY_SUITE_ID: &str = "civsim.units.formula-canary-suite.recursive-descent-producer.v2";

pub(super) fn production_canary_attestation() -> Result<CanaryAttestation, String> {
    static ATTESTATION: OnceLock<Result<CanaryAttestation, String>> = OnceLock::new();
    ATTESTATION.get_or_init(run_production_canary_suite).clone()
}

fn run_production_canary_suite() -> Result<CanaryAttestation, String> {
    let mut transcript = ProducerCanaryTranscript::new();
    let fixed_zero = ProjectionRequest::FixedScale { scale_bits: 0 };

    for (case_id, formula, expected) in [
        ("power-over-unary", "5 + -2^2", 1),
        ("parenthesized-negative-power", "(-2)^2", 4),
        ("explicit-negated-power", "5 + -(2^2)", 1),
    ] {
        producer_canary_expect_projection(
            &mut transcript,
            case_id,
            formula,
            &[],
            fixed_zero,
            expected,
        )?;
    }
    producer_canary_expect_projection(
        &mut transcript,
        "frozen-pi-scale-96",
        "pi",
        &[],
        ProjectionRequest::FixedScale { scale_bits: 96 },
        248902613312231085230521944622,
    )?;
    let producer_multilimb_inputs = [
        ProjectionInput::binary("two96", 1, 96),
        ProjectionInput::binary("two192", 1, 192),
    ];
    producer_canary_expect_projection(
        &mut transcript,
        "frozen-multilimb-rational",
        "((two96 + 1) * (two96 - 1)) / (two192 - 1)",
        &producer_multilimb_inputs,
        fixed_zero,
        1,
    )?;
    for (case_id, formula) in [
        ("refuse-plus-exponent", "2^+3"),
        ("refuse-minus-exponent", "2^-3"),
        ("refuse-parenthesized-exponent", "2^(3)"),
        ("refuse-decimal-exponent", "2^3.0"),
        ("refuse-chained-exponent", "2^3^2"),
    ] {
        producer_canary_expect_projection_refusal(
            &mut transcript,
            case_id,
            formula,
            &[],
            fixed_zero,
        )?;
    }

    let binding_inputs = [ProjectionInput::new("x", 2, 0)];
    let binding_certificate = produce("x + 1", &binding_inputs, 0)?;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-formula",
        &binding_certificate,
        "x + 2",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    let changed_inputs = [ProjectionInput::new("x", 3, 0)];
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-input",
        &binding_certificate,
        "x + 1",
        &changed_inputs,
        fixed_zero,
        false,
    )?;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-request",
        &binding_certificate,
        "x + 1",
        &binding_inputs,
        ProjectionRequest::FixedScale { scale_bits: 1 },
        false,
    )?;
    let mut mutation = binding_certificate.clone();
    mutation.formula_sha256[0] ^= 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-formula-digest",
        &mutation,
        "x + 1",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    mutation = binding_certificate.clone();
    mutation.inputs_sha256[0] ^= 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-input-digest",
        &mutation,
        "x + 1",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    mutation = binding_certificate.clone();
    mutation.request = ProjectionRequest::FixedScale { scale_bits: 1 };
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-certificate-request",
        &mutation,
        "x + 1",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    mutation = binding_certificate.clone();
    mutation.target_binary_exponent2 += 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-target-exponent",
        &mutation,
        "x + 1",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    mutation = binding_certificate.clone();
    mutation.magnitude_log2 += 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-magnitude",
        &mutation,
        "x + 1",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    mutation = binding_certificate.clone();
    mutation.lower = BigRat::from_i64(4);
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-endpoint",
        &mutation,
        "x + 1",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    mutation = binding_certificate.clone();
    mutation.producer_bits += 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "mutate-result",
        &mutation,
        "x + 1",
        &binding_inputs,
        fixed_zero,
        false,
    )?;
    for (case_id, mutate_identity) in [
        ("mutate-schema", 0_u8),
        ("mutate-producer-id", 1_u8),
        ("mutate-watchdog-id", 2_u8),
    ] {
        mutation = binding_certificate.clone();
        match mutate_identity {
            0 => mutation.schema_id = "civsim.units.canary.invalid-schema",
            1 => mutation.producer_implementation_id = "civsim.units.canary.invalid-producer",
            _ => mutation.watchdog_implementation_id = "civsim.units.canary.invalid-watchdog",
        }
        producer_canary_expect_recheck_refusal(
            &mut transcript,
            case_id,
            &mutation,
            "x + 1",
            &binding_inputs,
            fixed_zero,
            false,
        )?;
    }

    producer_canary_expect_sqrt(&mut transcript, "sqrt-exact-success", "4", &[], 0, 2)?;
    producer_canary_expect_sqrt_refusal(&mut transcript, "sqrt-refuse-zero", "0", &[], 0)?;
    producer_canary_expect_sqrt_refusal(&mut transcript, "sqrt-refuse-negative", "-1", &[], 0)?;
    let crossing = [ProjectionInput::binary_interval("x", 15, 17, -2)];
    producer_canary_expect_sqrt_refusal(
        &mut transcript,
        "sqrt-refuse-crossing-cell",
        "x",
        &crossing,
        0,
    )?;

    let sqrt_inputs = [ProjectionInput::new("x", 4, 0)];
    let sqrt_certificate = produce_positive_sqrt("x", &sqrt_inputs, 0)?;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-formula",
        &sqrt_certificate,
        "x + 5",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;
    let changed_sqrt_inputs = [ProjectionInput::new("x", 9, 0)];
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-input",
        &sqrt_certificate,
        "x",
        &changed_sqrt_inputs,
        fixed_zero,
        true,
    )?;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-request",
        &sqrt_certificate,
        "x",
        &sqrt_inputs,
        ProjectionRequest::FixedScale { scale_bits: 1 },
        true,
    )?;
    mutation = sqrt_certificate.clone();
    mutation.target_binary_exponent2 += 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-target",
        &mutation,
        "x",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;
    mutation = sqrt_certificate.clone();
    mutation.magnitude_log2 += 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-magnitude",
        &mutation,
        "x",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;
    mutation = sqrt_certificate.clone();
    mutation.upper = BigRat::from_i64(5);
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-endpoint",
        &mutation,
        "x",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;
    mutation = sqrt_certificate.clone();
    mutation.producer_bits += 1;
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-result",
        &mutation,
        "x",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;
    mutation = sqrt_certificate.clone();
    mutation.schema_id = "civsim.units.canary.invalid-sqrt-schema";
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-schema",
        &mutation,
        "x",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;
    mutation = sqrt_certificate.clone();
    mutation.producer_implementation_id = "civsim.units.canary.invalid-sqrt-producer";
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-producer-id",
        &mutation,
        "x",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;
    mutation = sqrt_certificate.clone();
    mutation.watchdog_implementation_id = "civsim.units.canary.invalid-sqrt-watchdog";
    producer_canary_expect_recheck_refusal(
        &mut transcript,
        "sqrt-mutate-watchdog-id",
        &mutation,
        "x",
        &sqrt_inputs,
        fixed_zero,
        true,
    )?;

    let oversized_formula = "1".repeat(MAX_FORMULA_BYTES + 1);
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-formula-bytes",
        &oversized_formula,
        &[],
        fixed_zero,
    )?;
    let excessive_inputs = vec![ProjectionInput::new("x", 1, 0); MAX_INPUTS + 1];
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-input-count",
        "1",
        &excessive_inputs,
        fixed_zero,
    )?;
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-target-scale",
        "1",
        &[],
        ProjectionRequest::FixedScale {
            scale_bits: MAX_NORMALIZATION_SHIFT_BITS + 1,
        },
    )?;
    let wide_coordinate = [ProjectionInput::binary(
        "x",
        1,
        (MAX_NORMALIZATION_SHIFT_BITS + 1) as i32,
    )];
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-coordinate",
        "x",
        &wide_coordinate,
        fixed_zero,
    )?;
    let decimal_resource = [ProjectionInput::decimal("x", "1e257")];
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-decimal",
        "x",
        &decimal_resource,
        fixed_zero,
    )?;
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-power",
        "10^65",
        &[],
        fixed_zero,
    )?;
    let nested = format!(
        "{}1{}",
        "(".repeat(MAX_NESTING_DEPTH + 1),
        ")".repeat(MAX_NESTING_DEPTH + 1)
    );
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-nesting",
        &nested,
        &[],
        fixed_zero,
    )?;
    let signed = format!("{}1", "-".repeat(MAX_SIGN_RUN + 1));
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-sign-run",
        &signed,
        &[],
        fixed_zero,
    )?;
    let huge_intermediate = [ProjectionInput::binary("x", i128::MAX, 4096)];
    producer_canary_expect_projection_refusal(
        &mut transcript,
        "resource-intermediate",
        "x^64",
        &huge_intermediate,
        fixed_zero,
    )?;

    let outer_preimage = producer_canary_factored_preimage(
        [0x11; 32], 7, -4, "x + 1", 32, [0x22; 32], 9, [0x33; 32], 7, -4, "x + 1", 32, [0x22; 32],
        9,
    );
    let baseline = factored_receipt_digest([0x11; 32], 7, -4, "x + 1", 32, [0x22; 32], 9);
    let substituted = factored_receipt_digest([0x33; 32], 7, -4, "x + 1", 32, [0x22; 32], 9);
    let outer_observation = if baseline != substituted {
        Ok(1)
    } else {
        Err("producer factored binding accepted a substituted outer receipt".to_owned())
    };
    transcript.record(
        "factored-outer-binding-substitution",
        &outer_preimage,
        &outer_observation,
    );
    outer_observation?;

    Ok(transcript.finish())
}

struct ProducerCanaryTranscript {
    hash: Sha256,
    case_count: u32,
}

impl ProducerCanaryTranscript {
    fn new() -> Self {
        let mut hash = Sha256::new();
        producer_canary_field(&mut hash, 1, CANARY_TRANSCRIPT_SCHEMA_ID.as_bytes());
        producer_canary_field(&mut hash, 2, CANARY_SUITE_ID.as_bytes());
        Self {
            hash,
            case_count: 0,
        }
    }

    fn record(&mut self, case_id: &str, complete_preimage: &[u8], outcome: &Result<i128, String>) {
        self.case_count += 1;
        producer_canary_field(&mut self.hash, 3, &self.case_count.to_le_bytes());
        producer_canary_field(&mut self.hash, 4, case_id.as_bytes());
        producer_canary_field(&mut self.hash, 5, complete_preimage);
        match outcome {
            Ok(bits) => {
                producer_canary_field(&mut self.hash, 6, &[1]);
                producer_canary_field(&mut self.hash, 7, &bits.to_le_bytes());
            }
            Err(error) => {
                producer_canary_field(&mut self.hash, 6, &[0]);
                producer_canary_field(&mut self.hash, 7, error.as_bytes());
            }
        }
    }

    fn finish(self) -> CanaryAttestation {
        CanaryAttestation {
            suite_schema_id: CANARY_TRANSCRIPT_SCHEMA_ID,
            suite_id: CANARY_SUITE_ID,
            case_count: self.case_count,
            transcript_sha256: self.hash.finalize().into(),
        }
    }
}

fn producer_canary_field(hash: &mut Sha256, tag: u8, bytes: &[u8]) {
    hash.update([tag]);
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

fn producer_canary_expect_projection(
    transcript: &mut ProducerCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    expected: i128,
) -> Result<(), String> {
    let preimage = producer_canary_projection_preimage(0, formula, inputs, request, None);
    let observed = producer_canary_produce(formula, inputs, request)
        .map(|certificate| certificate.producer_bits);
    transcript.record(case_id, &preimage, &observed);
    match observed {
        Ok(bits) if bits == expected => Ok(()),
        Ok(bits) => Err(format!(
            "producer canary '{case_id}' selected {bits}, expected {expected}"
        )),
        Err(error) => Err(format!("producer canary '{case_id}' refused: {error}")),
    }
}

fn producer_canary_expect_projection_refusal(
    transcript: &mut ProducerCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
) -> Result<(), String> {
    let preimage = producer_canary_projection_preimage(0, formula, inputs, request, None);
    let observed = producer_canary_produce(formula, inputs, request)
        .map(|certificate| certificate.producer_bits);
    transcript.record(case_id, &preimage, &observed);
    if observed.is_err() {
        Ok(())
    } else {
        Err(format!("producer canary '{case_id}' did not refuse"))
    }
}

fn producer_canary_produce(
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
) -> Result<ProjectionCertificate, String> {
    match request {
        ProjectionRequest::FixedScale { scale_bits } => produce(formula, inputs, scale_bits),
        ProjectionRequest::Significant { bits } => produce_significant(formula, inputs, bits),
    }
}

fn producer_canary_expect_sqrt(
    transcript: &mut ProducerCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    scale_bits: u32,
    expected: i128,
) -> Result<(), String> {
    let request = ProjectionRequest::FixedScale { scale_bits };
    let preimage = producer_canary_projection_preimage(1, formula, inputs, request, None);
    let observed = produce_positive_sqrt(formula, inputs, scale_bits)
        .map(|certificate| certificate.producer_bits);
    transcript.record(case_id, &preimage, &observed);
    match observed {
        Ok(bits) if bits == expected => Ok(()),
        Ok(bits) => Err(format!(
            "producer canary '{case_id}' selected root {bits}, expected {expected}"
        )),
        Err(error) => Err(format!("producer canary '{case_id}' refused: {error}")),
    }
}

fn producer_canary_expect_sqrt_refusal(
    transcript: &mut ProducerCanaryTranscript,
    case_id: &str,
    formula: &str,
    inputs: &[ProjectionInput],
    scale_bits: u32,
) -> Result<(), String> {
    let request = ProjectionRequest::FixedScale { scale_bits };
    let preimage = producer_canary_projection_preimage(1, formula, inputs, request, None);
    let observed = produce_positive_sqrt(formula, inputs, scale_bits)
        .map(|certificate| certificate.producer_bits);
    transcript.record(case_id, &preimage, &observed);
    if observed.is_err() {
        Ok(())
    } else {
        Err(format!("producer canary '{case_id}' did not refuse"))
    }
}

fn producer_canary_expect_recheck_refusal(
    transcript: &mut ProducerCanaryTranscript,
    case_id: &str,
    certificate: &ProjectionCertificate,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    positive_sqrt: bool,
) -> Result<(), String> {
    let operation = if positive_sqrt { 3 } else { 2 };
    let preimage =
        producer_canary_projection_preimage(operation, formula, inputs, request, Some(certificate));
    let observed = producer_canary_recheck(certificate, formula, inputs, request, positive_sqrt);
    transcript.record(case_id, &preimage, &observed);
    if observed.is_err() {
        Ok(())
    } else {
        Err(format!("producer canary '{case_id}' accepted a mutation"))
    }
}

fn producer_canary_recheck(
    certificate: &ProjectionCertificate,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    positive_sqrt: bool,
) -> Result<i128, String> {
    let reproduced = if positive_sqrt {
        let ProjectionRequest::FixedScale { scale_bits } = request else {
            return Err("producer square-root recheck requires fixed scale".to_owned());
        };
        produce_positive_sqrt(formula, inputs, scale_bits)?
    } else {
        producer_canary_produce(formula, inputs, request)?
    };
    if !producer_canary_same_certificate(certificate, &reproduced) {
        return Err("producer recheck differs from the supplied certificate".to_owned());
    }
    Ok(reproduced.producer_bits)
}

fn producer_canary_same_certificate(
    supplied: &ProjectionCertificate,
    reproduced: &ProjectionCertificate,
) -> bool {
    supplied.schema_id == reproduced.schema_id
        && supplied.producer_implementation_id == reproduced.producer_implementation_id
        && supplied.watchdog_implementation_id == reproduced.watchdog_implementation_id
        && supplied.formula_sha256 == reproduced.formula_sha256
        && supplied.inputs_sha256 == reproduced.inputs_sha256
        && supplied.request == reproduced.request
        && supplied.target_binary_exponent2 == reproduced.target_binary_exponent2
        && supplied.pi_terms == reproduced.pi_terms
        && supplied.magnitude_log2 == reproduced.magnitude_log2
        && supplied.lower.cmp_rat(&reproduced.lower) == Ordering::Equal
        && supplied.upper.cmp_rat(&reproduced.upper) == Ordering::Equal
        && supplied.producer_bits == reproduced.producer_bits
        && supplied.watchdog_bits == reproduced.watchdog_bits
        && supplied.producer_canary_attestation == reproduced.producer_canary_attestation
        && supplied.watchdog_canary_attestation == reproduced.watchdog_canary_attestation
        && supplied.receipt_sha256 == reproduced.receipt_sha256
}

fn producer_canary_projection_preimage(
    operation: u8,
    formula: &str,
    inputs: &[ProjectionInput],
    request: ProjectionRequest,
    certificate: Option<&ProjectionCertificate>,
) -> Vec<u8> {
    let mut frame = Vec::new();
    producer_canary_frame_field(&mut frame, 0, &[operation]);
    producer_canary_frame_field(&mut frame, 1, formula.as_bytes());
    producer_canary_encode_inputs(&mut frame, inputs);
    producer_canary_encode_request(&mut frame, request);
    match certificate {
        Some(certificate) => {
            producer_canary_frame_field(&mut frame, 4, &[1]);
            producer_canary_encode_certificate(&mut frame, certificate);
        }
        None => producer_canary_frame_field(&mut frame, 4, &[0]),
    }
    frame
}

fn producer_canary_encode_inputs(frame: &mut Vec<u8>, inputs: &[ProjectionInput]) {
    producer_canary_frame_field(frame, 2, &(inputs.len() as u64).to_le_bytes());
    for input in inputs {
        producer_canary_frame_field(frame, 20, input.symbol.as_bytes());
        match input.coordinate {
            ProjectionCoordinate::Scaled { bits, scale_bits } => {
                producer_canary_frame_field(frame, 21, &[0]);
                producer_canary_frame_field(frame, 22, &bits.to_le_bytes());
                producer_canary_frame_field(frame, 23, &scale_bits.to_le_bytes());
            }
            ProjectionCoordinate::Decimal(value) => {
                producer_canary_frame_field(frame, 21, &[1]);
                producer_canary_frame_field(frame, 22, value.as_bytes());
            }
            ProjectionCoordinate::Binary { bits, exponent2 } => {
                producer_canary_frame_field(frame, 21, &[2]);
                producer_canary_frame_field(frame, 22, &bits.to_le_bytes());
                producer_canary_frame_field(frame, 23, &exponent2.to_le_bytes());
            }
            ProjectionCoordinate::BinaryInterval {
                lower_bits,
                upper_bits,
                exponent2,
            } => {
                producer_canary_frame_field(frame, 21, &[3]);
                producer_canary_frame_field(frame, 22, &lower_bits.to_le_bytes());
                producer_canary_frame_field(frame, 23, &upper_bits.to_le_bytes());
                producer_canary_frame_field(frame, 24, &exponent2.to_le_bytes());
            }
        }
    }
}

fn producer_canary_encode_request(frame: &mut Vec<u8>, request: ProjectionRequest) {
    match request {
        ProjectionRequest::FixedScale { scale_bits } => {
            producer_canary_frame_field(frame, 3, &[0]);
            producer_canary_frame_field(frame, 30, &scale_bits.to_le_bytes());
        }
        ProjectionRequest::Significant { bits } => {
            producer_canary_frame_field(frame, 3, &[1]);
            producer_canary_frame_field(frame, 30, &bits.to_le_bytes());
        }
    }
}

fn producer_canary_encode_certificate(frame: &mut Vec<u8>, certificate: &ProjectionCertificate) {
    producer_canary_frame_field(frame, 40, certificate.schema_id.as_bytes());
    producer_canary_frame_field(frame, 41, certificate.producer_implementation_id.as_bytes());
    producer_canary_frame_field(frame, 42, certificate.watchdog_implementation_id.as_bytes());
    producer_canary_frame_field(frame, 43, &certificate.formula_sha256);
    producer_canary_frame_field(frame, 44, &certificate.inputs_sha256);
    producer_canary_encode_request(frame, certificate.request);
    producer_canary_frame_field(
        frame,
        45,
        &certificate.target_binary_exponent2.to_le_bytes(),
    );
    producer_canary_frame_field(frame, 46, &certificate.pi_terms.to_le_bytes());
    producer_canary_frame_field(frame, 47, &certificate.magnitude_log2.to_le_bytes());
    producer_canary_frame_field(frame, 48, &certificate.lower.canonical_bytes());
    producer_canary_frame_field(frame, 49, &certificate.upper.canonical_bytes());
    producer_canary_frame_field(frame, 50, &certificate.producer_bits.to_le_bytes());
    producer_canary_frame_field(frame, 51, &certificate.watchdog_bits.to_le_bytes());
    producer_canary_encode_attestation(frame, 52, certificate.producer_canary_attestation);
    producer_canary_encode_attestation(frame, 53, certificate.watchdog_canary_attestation);
    producer_canary_frame_field(frame, 54, &certificate.receipt_sha256);
}

fn producer_canary_encode_attestation(
    frame: &mut Vec<u8>,
    tag: u8,
    attestation: CanaryAttestation,
) {
    let mut nested = Vec::new();
    producer_canary_frame_field(&mut nested, 1, attestation.suite_schema_id.as_bytes());
    producer_canary_frame_field(&mut nested, 2, attestation.suite_id.as_bytes());
    producer_canary_frame_field(&mut nested, 3, &attestation.case_count.to_le_bytes());
    producer_canary_frame_field(&mut nested, 4, &attestation.transcript_sha256);
    producer_canary_frame_field(frame, tag, &nested);
}

#[allow(clippy::too_many_arguments)]
fn producer_canary_factored_preimage(
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
    producer_canary_frame_field(&mut frame, 1, &first_coefficient_receipt);
    producer_canary_frame_field(&mut frame, 2, &first_coefficient_bits.to_le_bytes());
    producer_canary_frame_field(&mut frame, 3, &first_coefficient_exponent.to_le_bytes());
    producer_canary_frame_field(&mut frame, 4, first_formula.as_bytes());
    producer_canary_frame_field(&mut frame, 5, &first_scale.to_le_bytes());
    producer_canary_frame_field(&mut frame, 6, &first_terminal_receipt);
    producer_canary_frame_field(&mut frame, 7, &first_terminal_bits.to_le_bytes());
    producer_canary_frame_field(&mut frame, 8, &second_coefficient_receipt);
    producer_canary_frame_field(&mut frame, 9, &second_coefficient_bits.to_le_bytes());
    producer_canary_frame_field(&mut frame, 10, &second_coefficient_exponent.to_le_bytes());
    producer_canary_frame_field(&mut frame, 11, second_formula.as_bytes());
    producer_canary_frame_field(&mut frame, 12, &second_scale.to_le_bytes());
    producer_canary_frame_field(&mut frame, 13, &second_terminal_receipt);
    producer_canary_frame_field(&mut frame, 14, &second_terminal_bits.to_le_bytes());
    frame
}

fn producer_canary_frame_field(frame: &mut Vec<u8>, tag: u8, bytes: &[u8]) {
    frame.push(tag);
    frame.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    frame.extend_from_slice(bytes);
}
