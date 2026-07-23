#!/usr/bin/env python3
"""Prove and cross-check the live deterministic Q32.32 constant table."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import pathlib
import re
import subprocess
import sys
import tempfile
from fractions import Fraction
from typing import Iterable


ROOT = pathlib.Path(__file__).resolve().parent.parent
CORE_PATH = ROOT / "crates" / "core" / "src" / "fixed.rs"
GPU_PATH = ROOT / "crates" / "gpu" / "src" / "transcendental.rs"
WATCHDOG_PATH = ROOT / "scripts" / "fixed_math_authority_watchdog.py"
RECEIPT_PATH = ROOT / "crates" / "units" / "data" / "fixed_math_authority_receipt.json"

SCHEMA = "civsim.core.fixed-math-authority-pair.v1"
CLAIM_ID = "core.deterministic-math-table"
PRODUCER_IMPLEMENTATION = (
    "civsim.fixed-math.machin-atanh-interval-producer.v1"
)
WATCHDOG_IMPLEMENTATION = (
    "civsim.fixed-math.split-identity-square-proof-watchdog.v1"
)
FRAC_BITS = 32
CORDIC_ITERATIONS = 32

CANARIES = [
    "one-bit-constant-mutation",
    "atan-member-omission",
    "atan-member-reordering",
    "gpu-mirror-drift",
    "source-digest-mutation",
    "claim-id-mutation",
    "schema-mutation",
    "receipt-byte-mutation",
    "derivation-formula-mutation",
]

DERIVATIONS = [
    {
        "claim": "pi",
        "producer": "machin:16*atan(1/5)-4*atan(1/239)",
        "watchdog": "split-angle:4*(atan(1/2)+atan(1/3))",
    },
    {
        "claim": "ln2",
        "producer": "atanh:2*atanh(1/3)",
        "watchdog": "split-atanh:2*(atanh(1/5)+atanh(1/7))",
    },
    {
        "claim": "inverse-ln2",
        "producer": "positive-rational-interval-reciprocal",
        "watchdog": "cross-multiplied-reciprocal-bracket",
    },
    {
        "claim": "cordic-atan-table",
        "producer": "direct-alternating-arctan-with-pi-quarter-at-index-zero",
        "watchdog": "rational-tangent-split-arctan",
    },
    {
        "claim": "cordic-inverse-gain",
        "producer": "exact-product-rational-sqrt-interval",
        "watchdog": "squared-threshold-nearest-integer-search",
    },
]


class FixedMathAuthorityError(ValueError):
    """The fixed-math authority claim or receipt failed closed."""


Interval = tuple[Fraction, Fraction]
Occurrence = dict[str, int | str]


def _line_number(raw: bytes, offset: int) -> int:
    return raw.count(b"\n", 0, offset) + 1


def _interval_add(left: Interval, right: Interval) -> Interval:
    return left[0] + right[0], left[1] + right[1]


def _interval_scale(coefficient: Fraction | int, value: Interval) -> Interval:
    coefficient = Fraction(coefficient)
    if coefficient >= 0:
        return coefficient * value[0], coefficient * value[1]
    return coefficient * value[1], coefficient * value[0]


def _interval_reciprocal(value: Interval) -> Interval:
    if value[0] <= 0:
        raise FixedMathAuthorityError(
            "reciprocal interval must be strictly positive"
        )
    return Fraction(1, value[1]), Fraction(1, value[0])


def _alternating_arctan_bounds(x: Fraction, terms: int) -> Interval:
    if not (0 < x <= 1):
        raise FixedMathAuthorityError("arctan series requires 0 < x <= 1")
    if terms < 1:
        raise FixedMathAuthorityError("arctan series requires at least one term")
    partial = Fraction(0)
    power = x
    square = x * x
    for index in range(terms):
        term = power / (2 * index + 1)
        partial = partial + term if index % 2 == 0 else partial - term
        power *= square
    next_term = power / (2 * terms + 1)
    adjacent = partial + next_term if terms % 2 == 0 else partial - next_term
    return min(partial, adjacent), max(partial, adjacent)


def _positive_atanh_bounds(x: Fraction, terms: int) -> Interval:
    if not (0 < x < 1):
        raise FixedMathAuthorityError("atanh series requires 0 < x < 1")
    if terms < 1:
        raise FixedMathAuthorityError("atanh series requires at least one term")
    partial = Fraction(0)
    power = x
    square = x * x
    for index in range(terms):
        partial += power / (2 * index + 1)
        power *= square
    first_omitted = power / (2 * terms + 1)
    tail_bound = first_omitted / (1 - square)
    return partial, partial + tail_bound


def _round_half_even(value: Fraction) -> int:
    quotient, remainder = divmod(value.numerator, value.denominator)
    comparison = 2 * remainder - value.denominator
    if comparison > 0 or (comparison == 0 and quotient % 2 != 0):
        return quotient + 1
    return quotient


def _certified_q32(interval: Interval, label: str) -> int:
    if interval[0] > interval[1]:
        raise FixedMathAuthorityError(f"{label} has an inverted interval")
    scale = 1 << FRAC_BITS
    low = _round_half_even(interval[0] * scale)
    high = _round_half_even(interval[1] * scale)
    if low != high:
        raise FixedMathAuthorityError(
            f"{label} interval does not prove one Q32.32 rounding"
        )
    if not -(1 << 63) <= low < (1 << 63):
        raise FixedMathAuthorityError(f"{label} does not fit i64")
    return low


def _cordic_gain_interval(iterations: int, precision_bits: int = 112) -> Interval:
    gain_squared = Fraction(1)
    for index in range(iterations):
        gain_squared /= 1 + Fraction(1, 1 << (2 * index))
    scaled_floor = (
        gain_squared.numerator << (2 * precision_bits)
    ) // gain_squared.denominator
    root_floor = math.isqrt(scaled_floor)
    denominator = 1 << precision_bits
    low = Fraction(root_floor, denominator)
    high = Fraction(root_floor + 1, denominator)
    if low * low > gain_squared or high * high <= gain_squared:
        raise FixedMathAuthorityError(
            "CORDIC gain square-root bracket did not contain the exact product"
        )
    return low, high


def derive_ordered_bits(pi_tail_denominator: int = 239) -> list[dict[str, int | str]]:
    pi = _interval_add(
        _interval_scale(
            16, _alternating_arctan_bounds(Fraction(1, 5), 48)
        ),
        _interval_scale(
            -4,
            _alternating_arctan_bounds(
                Fraction(1, pi_tail_denominator), 24
            ),
        ),
    )
    half_pi = _interval_scale(Fraction(1, 2), pi)
    ln2 = _interval_scale(
        2, _positive_atanh_bounds(Fraction(1, 3), 56)
    )
    inverse_ln2 = _interval_reciprocal(ln2)

    values: list[dict[str, int | str]] = [
        {"id": "pi", "bits": _certified_q32(pi, "pi")},
        {"id": "half_pi", "bits": _certified_q32(half_pi, "half pi")},
        {"id": "ln2", "bits": _certified_q32(ln2, "ln2")},
        {
            "id": "inverse_ln2",
            "bits": _certified_q32(inverse_ln2, "inverse ln2"),
        },
        {
            "id": "cordic_inverse_gain",
            "bits": _certified_q32(
                _cordic_gain_interval(CORDIC_ITERATIONS),
                "CORDIC inverse gain",
            ),
        },
    ]
    for index in range(CORDIC_ITERATIONS):
        if index == 0:
            interval = _interval_scale(Fraction(1, 4), pi)
        else:
            interval = _alternating_arctan_bounds(
                Fraction(1, 1 << index), 104
            )
        values.append(
            {
                "id": f"cordic_atan[{index}]",
                "bits": _certified_q32(interval, f"CORDIC atan {index}"),
            }
        )
    return values


def _unique_match(pattern: bytes, raw: bytes, label: str) -> re.Match[bytes]:
    matches = list(re.finditer(pattern, raw, flags=re.MULTILINE))
    if len(matches) != 1:
        raise FixedMathAuthorityError(
            f"{label} must have exactly one canonical source declaration"
        )
    return matches[0]


def _parse_core(raw: bytes) -> tuple[int, list[Occurrence]]:
    declarations = [
        ("core.ln2", rb"^const LN2: Fixed = Fixed::from_bits\((-?\d+)\);"),
        (
            "core.inverse_ln2",
            rb"^const INV_LN2: Fixed = Fixed::from_bits\((-?\d+)\);",
        ),
        (
            "core.pi",
            rb"^const PI_BITS: Fixed = Fixed::from_bits\((-?\d+)\);",
        ),
        (
            "core.half_pi",
            rb"^const HALF_PI_BITS: Fixed = Fixed::from_bits\((-?\d+)\);",
        ),
        (
            "core.cordic_inverse_gain",
            rb"^const CORDIC_INV_GAIN: Fixed = Fixed::from_bits\((-?\d+)\);",
        ),
    ]
    occurrences: list[Occurrence] = []
    for role, pattern in declarations:
        match = _unique_match(pattern, raw, role)
        occurrences.append(
            {
                "path": "crates/core/src/fixed.rs",
                "line": _line_number(raw, match.start(1)),
                "role": role,
                "bits": int(match.group(1)),
            }
        )

    count_match = _unique_match(
        rb"^const CORDIC_N: usize = (\d+);", raw, "core.CORDIC_N"
    )
    iterations = int(count_match.group(1))
    table_match = _unique_match(
        rb"^const CORDIC_ATAN: \[Fixed; CORDIC_N\] = \[\r?\n"
        rb"(?P<body>[\s\S]*?)^\];",
        raw,
        "core.CORDIC_ATAN",
    )
    body = table_match.group("body")
    entries = list(
        re.finditer(rb"^\s*Fixed::from_bits\((-?\d+)\),\r?$", body, re.MULTILINE)
    )
    if len(entries) != iterations:
        raise FixedMathAuthorityError(
            "core CORDIC table length differs from CORDIC_N"
        )
    body_offset = table_match.start("body")
    for index, match in enumerate(entries):
        absolute = body_offset + match.start(1)
        occurrences.append(
            {
                "path": "crates/core/src/fixed.rs",
                "line": _line_number(raw, absolute),
                "role": f"core.cordic_atan[{index}]",
                "bits": int(match.group(1)),
            }
        )
    return iterations, occurrences


def _parse_gpu(raw: bytes) -> tuple[int, list[Occurrence]]:
    occurrences: list[Occurrence] = []
    scalar_patterns = [
        ("gpu.ln2", rb"^\s*let ln2 = (-?\d+)i64;"),
        ("gpu.inverse_ln2", rb"^\s*let inv_ln2 = (-?\d+)i64;"),
        ("gpu.half_pi", rb"^\s*let half_pi = (-?\d+)i64;"),
    ]
    expected_counts = {
        "gpu.ln2": 2,
        "gpu.inverse_ln2": 1,
        "gpu.half_pi": 4,
    }
    for role, pattern in scalar_patterns:
        matches = list(re.finditer(pattern, raw, flags=re.MULTILINE))
        if len(matches) != expected_counts[role]:
            raise FixedMathAuthorityError(
                f"{role} source occurrence count changed"
            )
        for index, match in enumerate(matches):
            occurrences.append(
                {
                    "path": "crates/gpu/src/transcendental.rs",
                    "line": _line_number(raw, match.start(1)),
                    "role": f"{role}[{index}]",
                    "bits": int(match.group(1)),
                }
            )

    gain = _unique_match(
        rb"^\s*let mut x = (-?\d+)i64; // CORDIC_INV_GAIN",
        raw,
        "gpu.cordic_inverse_gain",
    )
    occurrences.append(
        {
            "path": "crates/gpu/src/transcendental.rs",
            "line": _line_number(raw, gain.start(1)),
            "role": "gpu.cordic_inverse_gain[0]",
            "bits": int(gain.group(1)),
        }
    )

    table_match = _unique_match(
        rb"^fn cordic_atan_table\(\) -> Array<i64> \{\r?\n"
        rb"(?P<body>[\s\S]*?)^\}",
        raw,
        "gpu.cordic_atan_table",
    )
    body = table_match.group("body")
    entries = list(
        re.finditer(
            rb"^\s*a\[(\d+)usize\] = (-?\d+)i64;\r?$",
            body,
            flags=re.MULTILINE,
        )
    )
    indices = [int(match.group(1)) for match in entries]
    if indices != list(range(CORDIC_ITERATIONS)):
        raise FixedMathAuthorityError(
            "GPU CORDIC table must contain ordered indices 0 through 31"
        )
    body_offset = table_match.start("body")
    for match in entries:
        index = int(match.group(1))
        absolute = body_offset + match.start(2)
        occurrences.append(
            {
                "path": "crates/gpu/src/transcendental.rs",
                "line": _line_number(raw, absolute),
                "role": f"gpu.cordic_atan[{index}]",
                "bits": int(match.group(2)),
            }
        )
    return len(entries), occurrences


def _numeric_literal_counts(raw: bytes, selected: set[int]) -> dict[int, int]:
    counts = {value: 0 for value in selected}
    for match in re.finditer(
        rb"(?<![A-Za-z0-9_])(-?\d+)(?:i64)?(?![A-Za-z0-9_])", raw
    ):
        value = int(match.group(1))
        if value in counts:
            counts[value] += 1
    return counts


def _validate_unbound_scalar_copies(
    core_raw: bytes,
    gpu_raw: bytes,
    occurrences: list[Occurrence],
    derived: dict[str, int],
) -> None:
    selected = {
        derived["pi"],
        derived["half_pi"],
        derived["ln2"],
        derived["inverse_ln2"],
        derived["cordic_inverse_gain"],
    }
    parsed_counts = {value: 0 for value in selected}
    for occurrence in occurrences:
        value = int(occurrence["bits"])
        if value in parsed_counts:
            parsed_counts[value] += 1
    source_counts = _numeric_literal_counts(core_raw + b"\n" + gpu_raw, selected)
    if source_counts != parsed_counts:
        raise FixedMathAuthorityError(
            "a scalar authority value has an unbound raw source copy"
        )


def parse_and_validate_sources(
    core_raw: bytes,
    gpu_raw: bytes,
    ordered_bits: list[dict[str, int | str]],
) -> list[Occurrence]:
    core_iterations, core_occurrences = _parse_core(core_raw)
    gpu_iterations, gpu_occurrences = _parse_gpu(gpu_raw)
    if core_iterations != CORDIC_ITERATIONS:
        raise FixedMathAuthorityError("core CORDIC_N must remain 32")
    if gpu_iterations != CORDIC_ITERATIONS:
        raise FixedMathAuthorityError("GPU CORDIC table must remain 32 entries")

    derived = {str(row["id"]): int(row["bits"]) for row in ordered_bits}
    expected_by_role = {
        "core.pi": derived["pi"],
        "core.half_pi": derived["half_pi"],
        "core.ln2": derived["ln2"],
        "core.inverse_ln2": derived["inverse_ln2"],
        "core.cordic_inverse_gain": derived["cordic_inverse_gain"],
    }
    expected_by_role.update(
        {
            f"core.cordic_atan[{index}]": derived[f"cordic_atan[{index}]"]
            for index in range(CORDIC_ITERATIONS)
        }
    )
    expected_by_role.update(
        {
            "gpu.ln2[0]": derived["ln2"],
            "gpu.ln2[1]": derived["ln2"],
            "gpu.inverse_ln2[0]": derived["inverse_ln2"],
            "gpu.half_pi[0]": derived["half_pi"],
            "gpu.half_pi[1]": derived["half_pi"],
            "gpu.half_pi[2]": derived["half_pi"],
            "gpu.half_pi[3]": derived["half_pi"],
            "gpu.cordic_inverse_gain[0]": derived["cordic_inverse_gain"],
        }
    )
    expected_by_role.update(
        {
            f"gpu.cordic_atan[{index}]": derived[f"cordic_atan[{index}]"]
            for index in range(CORDIC_ITERATIONS)
        }
    )

    occurrences = sorted(
        core_occurrences + gpu_occurrences,
        key=lambda row: (str(row["path"]), int(row["line"]), str(row["role"])),
    )
    found_by_role = {str(row["role"]): int(row["bits"]) for row in occurrences}
    if len(found_by_role) != len(occurrences):
        raise FixedMathAuthorityError("source occurrence roles are not unique")
    if found_by_role != expected_by_role:
        missing = sorted(set(expected_by_role) - set(found_by_role))
        extra = sorted(set(found_by_role) - set(expected_by_role))
        wrong = sorted(
            role
            for role in set(found_by_role) & set(expected_by_role)
            if found_by_role[role] != expected_by_role[role]
        )
        details: list[str] = []
        if missing:
            details.append("missing " + ", ".join(missing))
        if extra:
            details.append("extra " + ", ".join(extra))
        if wrong:
            details.append("wrong bits " + ", ".join(wrong))
        raise FixedMathAuthorityError(
            "source constants differ from exact derivation: " + "; ".join(details)
        )
    _validate_unbound_scalar_copies(
        core_raw, gpu_raw, occurrences, derived
    )
    return occurrences


def _canonical_json(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def build_receipt(core_raw: bytes, gpu_raw: bytes) -> bytes:
    ordered_bits = derive_ordered_bits()
    occurrences = parse_and_validate_sources(core_raw, gpu_raw, ordered_bits)
    receipt = {
        "canaries": CANARIES,
        "claim_id": CLAIM_ID,
        "cordic_iterations": CORDIC_ITERATIONS,
        "derivations": DERIVATIONS,
        "frac_bits": FRAC_BITS,
        "ordered_derived_bits": ordered_bits,
        "producer_implementation": PRODUCER_IMPLEMENTATION,
        "proof_contract": "exact-rational-interval-round-half-even-q32.32",
        "schema": SCHEMA,
        "source_digests": [
            {
                "path": "crates/core/src/fixed.rs",
                "sha256": hashlib.sha256(core_raw).hexdigest(),
            },
            {
                "path": "crates/gpu/src/transcendental.rs",
                "sha256": hashlib.sha256(gpu_raw).hexdigest(),
            },
        ],
        "source_occurrences": occurrences,
        "watchdog_implementation": WATCHDOG_IMPLEMENTATION,
    }
    return _canonical_json(receipt)


def require_exact_agreement(expected: bytes, candidate: bytes) -> None:
    if expected != candidate:
        raise FixedMathAuthorityError(
            "producer and watchdog canonical receipts differ"
        )


def cross_check(local_receipt: bytes) -> None:
    completed = subprocess.run(
        [sys.executable, str(WATCHDOG_PATH), "--receipt"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=90,
    )
    if completed.returncode != 0:
        detail = (completed.stdout + completed.stderr).decode(
            "utf-8", errors="replace"
        ).strip()
        raise FixedMathAuthorityError(
            f"independent fixed-math watchdog failed: {detail}"
        )
    if completed.stderr:
        raise FixedMathAuthorityError(
            "independent fixed-math watchdog emitted unexpected stderr"
        )
    require_exact_agreement(local_receipt + b"\n", completed.stdout)


def _require_rejected(
    label: str,
    core_raw: bytes,
    gpu_raw: bytes,
    ordered_bits: list[dict[str, int | str]],
) -> None:
    try:
        parse_and_validate_sources(core_raw, gpu_raw, ordered_bits)
    except FixedMathAuthorityError:
        return
    raise AssertionError(f"self-test mutation passed: {label}")


def _replace_once(raw: bytes, old: bytes, new: bytes, label: str) -> bytes:
    if raw.count(old) != 1:
        raise AssertionError(f"{label} self-test anchor is not unique")
    return raw.replace(old, new, 1)


def _exercise_receipt_mutation(
    receipt: bytes, field: str, value: object, label: str
) -> None:
    decoded = json.loads(receipt)
    decoded[field] = value
    mutated = _canonical_json(decoded)
    try:
        require_exact_agreement(receipt, mutated)
    except FixedMathAuthorityError:
        return
    raise AssertionError(f"{label} mutation passed")


def _require_receipt_rejected(
    baseline: bytes, mutation: bytes, label: str
) -> None:
    try:
        require_exact_agreement(baseline, mutation)
    except FixedMathAuthorityError:
        return
    raise AssertionError(f"{label} mutation passed")


def _run_watchdog_self_test() -> None:
    completed = subprocess.run(
        [sys.executable, str(WATCHDOG_PATH), "--self-test"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=120,
    )
    if completed.returncode != 0:
        detail = (completed.stdout + completed.stderr).decode(
            "utf-8", errors="replace"
        ).strip()
        raise FixedMathAuthorityError(
            f"independent fixed-math watchdog self-test failed: {detail}"
        )
    if completed.stderr:
        raise FixedMathAuthorityError(
            "independent fixed-math watchdog self-test emitted unexpected stderr"
        )
    if completed.stdout.decode("utf-8", errors="strict").splitlines() != [
        "fixed-math authority watchdog self-test: PASS"
    ]:
        raise FixedMathAuthorityError(
            "independent fixed-math watchdog self-test emitted unexpected output"
        )


def self_test() -> None:
    core_raw = CORE_PATH.read_bytes()
    gpu_raw = GPU_PATH.read_bytes()
    ordered_bits = derive_ordered_bits()
    baseline = build_receipt(core_raw, gpu_raw)

    pi_bits = next(
        int(row["bits"]) for row in ordered_bits if row["id"] == "pi"
    )
    _require_rejected(
        "one-bit constant mutation",
        _replace_once(
            core_raw,
            f"Fixed::from_bits({pi_bits}); // pi".encode(),
            f"Fixed::from_bits({pi_bits ^ 1}); // pi".encode(),
            "one-bit constant",
        ),
        gpu_raw,
        ordered_bits,
    )

    atan_zero = next(
        int(row["bits"])
        for row in ordered_bits
        if row["id"] == "cordic_atan[0]"
    )
    atan_one = next(
        int(row["bits"])
        for row in ordered_bits
        if row["id"] == "cordic_atan[1]"
    )
    _require_rejected(
        "atan member omission",
        _replace_once(
            core_raw,
            f"    Fixed::from_bits({atan_zero}),\n".encode(),
            b"",
            "atan omission",
        ),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "atan member reordering",
        _replace_once(
            core_raw,
            (
                f"    Fixed::from_bits({atan_zero}),\n"
                f"    Fixed::from_bits({atan_one}),\n"
            ).encode(),
            (
                f"    Fixed::from_bits({atan_one}),\n"
                f"    Fixed::from_bits({atan_zero}),\n"
            ).encode(),
            "atan reorder",
        ),
        gpu_raw,
        ordered_bits,
    )

    ln2_bits = next(
        int(row["bits"]) for row in ordered_bits if row["id"] == "ln2"
    )
    _require_rejected(
        "GPU mirror drift",
        core_raw,
        _replace_once(
            gpu_raw,
            f"let ln2 = {ln2_bits}i64; // ln 2".encode(),
            f"let ln2 = {ln2_bits ^ 1}i64; // ln 2".encode(),
            "GPU mirror drift",
        ),
        ordered_bits,
    )

    digest_mutation = build_receipt(
        core_raw + b"\n// source digest mutation canary\n", gpu_raw
    )
    _require_receipt_rejected(
        baseline, digest_mutation, "source digest"
    )

    _exercise_receipt_mutation(
        baseline, "claim_id", CLAIM_ID + ".mutated", "claim-id"
    )
    _exercise_receipt_mutation(
        baseline, "schema", SCHEMA + ".mutated", "schema"
    )
    receipt_byte_mutation = baseline[:-1] + (
        b"0" if baseline[-1:] != b"0" else b"1"
    )
    _require_receipt_rejected(
        baseline, receipt_byte_mutation, "receipt-byte"
    )

    mutated_derivation = derive_ordered_bits(pi_tail_denominator=238)
    _require_rejected(
        "derivation formula mutation",
        core_raw,
        gpu_raw,
        mutated_derivation,
    )

    cross_check(baseline)
    _run_watchdog_self_test()
    print("fixed-math authority producer self-test: PASS")


def _write_generated(receipt: bytes) -> None:
    RECEIPT_PATH.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=RECEIPT_PATH.name + ".",
        suffix=".tmp",
        dir=RECEIPT_PATH.parent,
    )
    temporary = pathlib.Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(receipt)
            handle.write(b"\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, RECEIPT_PATH)
    finally:
        if temporary.exists():
            temporary.unlink()


def validate_checked_receipt(receipt: bytes) -> None:
    try:
        checked = RECEIPT_PATH.read_bytes()
    except FileNotFoundError as error:
        raise FixedMathAuthorityError(
            "checked-in fixed-math authority receipt is missing; "
            "run with --generate after reviewing both proofs"
        ) from error
    require_exact_agreement(receipt + b"\n", checked)


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--receipt", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if sum((args.generate, args.receipt, args.self_test)) > 1:
        parser.error("--generate, --receipt, and --self-test are exclusive")
    try:
        if args.self_test:
            self_test()
            return 0
        receipt = build_receipt(CORE_PATH.read_bytes(), GPU_PATH.read_bytes())
        cross_check(receipt)
        if args.receipt:
            sys.stdout.buffer.write(receipt + b"\n")
            return 0
        if args.generate:
            _write_generated(receipt)
            print(
                "fixed-math authority receipt generated: "
                + RECEIPT_PATH.relative_to(ROOT).as_posix()
            )
            return 0
        validate_checked_receipt(receipt)
        print("fixed-math authority producer and watchdog: PASS")
    except (
        AssertionError,
        FixedMathAuthorityError,
        json.JSONDecodeError,
        OSError,
        subprocess.SubprocessError,
    ) as error:
        print(f"fixed-math authority producer and watchdog: FAIL: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
