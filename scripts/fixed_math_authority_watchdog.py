#!/usr/bin/env python3
"""Independently derive and inspect the deterministic Q32.32 constant table."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys
from fractions import Fraction
from typing import Iterable


ROOT = pathlib.Path(__file__).resolve().parent.parent
CORE_PATH = ROOT / "crates" / "core" / "src" / "fixed.rs"
GPU_PATH = ROOT / "crates" / "gpu" / "src" / "transcendental.rs"
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


class FixedMathWatchdogError(ValueError):
    """The independent fixed-math proof or source scan failed closed."""


Bound = tuple[Fraction, Fraction]
Occurrence = dict[str, int | str]


def _sum_bounds(left: Bound, right: Bound) -> Bound:
    return left[0] + right[0], left[1] + right[1]


def _multiply_bound(value: Bound, multiplier: Fraction | int) -> Bound:
    multiplier = Fraction(multiplier)
    products = (
        value[0] * multiplier,
        value[1] * multiplier,
    )
    return min(products), max(products)


def _atan_series_bracket(argument: Fraction, term_count: int) -> Bound:
    if argument <= 0 or argument > 1:
        raise FixedMathWatchdogError("invalid independent arctan argument")
    powers: list[Fraction] = []
    current = argument
    for _ in range(term_count + 1):
        powers.append(current)
        current *= argument * argument
    partial = sum(
        (
            powers[index] / (2 * index + 1)
            if index % 2 == 0
            else -powers[index] / (2 * index + 1)
        )
        for index in range(term_count)
    )
    extension = partial + (
        powers[term_count] / (2 * term_count + 1)
        if term_count % 2 == 0
        else -powers[term_count] / (2 * term_count + 1)
    )
    return min(partial, extension), max(partial, extension)


def _atanh_tail_bracket(argument: Fraction, term_count: int) -> Bound:
    if argument <= 0 or argument >= 1:
        raise FixedMathWatchdogError("invalid independent atanh argument")
    terms = [
        argument ** (2 * index + 1) / (2 * index + 1)
        for index in range(term_count)
    ]
    lower = sum(terms, Fraction(0))
    first_missing = argument ** (2 * term_count + 1) / (
        2 * term_count + 1
    )
    upper = lower + first_missing / (1 - argument * argument)
    return lower, upper


def _nearest_even_from_bound(bound: Bound, label: str) -> int:
    scaled = (
        bound[0] * (1 << FRAC_BITS),
        bound[1] * (1 << FRAC_BITS),
    )

    def nearest(value: Fraction) -> int:
        floor = value.numerator // value.denominator
        doubled_residue = 2 * (
            value.numerator - floor * value.denominator
        )
        if doubled_residue < value.denominator:
            return floor
        if doubled_residue > value.denominator:
            return floor + 1
        return floor if floor % 2 == 0 else floor + 1

    first = nearest(scaled[0])
    second = nearest(scaled[1])
    if first != second:
        raise FixedMathWatchdogError(
            f"{label} independent bracket does not select one Q32.32 integer"
        )
    return first


def _cordic_squared_rounding(iterations: int) -> int:
    numerator = 1
    denominator = 1
    for index in range(iterations):
        factor_denominator = 1 << (2 * index)
        numerator *= factor_denominator
        denominator *= factor_denominator + 1

    scaled_square_numerator = numerator << (2 * FRAC_BITS)
    low = 0
    high = 1 << FRAC_BITS
    while low < high:
        candidate = (low + high + 1) // 2
        if (
            candidate * candidate * denominator
            <= scaled_square_numerator
        ):
            low = candidate
        else:
            high = candidate - 1
    floor = low
    if not (
        floor * floor * denominator <= scaled_square_numerator
        < (floor + 1) * (floor + 1) * denominator
    ):
        raise FixedMathWatchdogError(
            "CORDIC squared-threshold floor proof failed"
        )

    midpoint_left = (2 * floor + 1) ** 2 * denominator
    midpoint_right = numerator << (2 * FRAC_BITS + 2)
    if midpoint_right < midpoint_left:
        rounded = floor
    elif midpoint_right > midpoint_left:
        rounded = floor + 1
    else:
        rounded = floor if floor % 2 == 0 else floor + 1
    return rounded


def derive_ordered_bits(
    pi_second_denominator: int = 3,
) -> list[dict[str, int | str]]:
    pi = _multiply_bound(
        _sum_bounds(
            _atan_series_bracket(Fraction(1, 2), 112),
            _atan_series_bracket(
                Fraction(1, pi_second_denominator), 112
            ),
        ),
        4,
    )
    ln2 = _multiply_bound(
        _sum_bounds(
            _atanh_tail_bracket(Fraction(1, 5), 64),
            _atanh_tail_bracket(Fraction(1, 7), 64),
        ),
        2,
    )
    if ln2[0] <= 0:
        raise FixedMathWatchdogError("ln2 bracket is not positive")
    inverse_ln2 = Fraction(1, ln2[1]), Fraction(1, ln2[0])

    claims: list[dict[str, int | str]] = [
        {"id": "pi", "bits": _nearest_even_from_bound(pi, "pi")},
        {
            "id": "half_pi",
            "bits": _nearest_even_from_bound(
                _multiply_bound(pi, Fraction(1, 2)), "half pi"
            ),
        },
        {"id": "ln2", "bits": _nearest_even_from_bound(ln2, "ln2")},
        {
            "id": "inverse_ln2",
            "bits": _nearest_even_from_bound(
                inverse_ln2, "inverse ln2"
            ),
        },
        {
            "id": "cordic_inverse_gain",
            "bits": _cordic_squared_rounding(CORDIC_ITERATIONS),
        },
    ]

    for index in range(CORDIC_ITERATIONS):
        x = Fraction(1, 1 << index)
        first = x / 2
        second = x / (2 + x * x)
        # tan(atan(first) + atan(second)) = x. Both angles are positive
        # and their sum is below pi/2, so the principal angle is unique.
        transformed = _sum_bounds(
            _atan_series_bracket(first, 112),
            _atan_series_bracket(second, 112),
        )
        claims.append(
            {
                "id": f"cordic_atan[{index}]",
                "bits": _nearest_even_from_bound(
                    transformed, f"CORDIC atan {index}"
                ),
            }
        )
    return claims


def _ascii_lines(raw: bytes, label: str) -> list[str]:
    try:
        return raw.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        raise FixedMathWatchdogError(
            f"{label} is not valid UTF-8"
        ) from error


def _decimal_between(text: str, prefix: str, suffix: str, label: str) -> int:
    if not text.startswith(prefix) or not text.endswith(suffix):
        raise FixedMathWatchdogError(f"{label} does not match its source form")
    body = text[len(prefix) : len(text) - len(suffix)]
    if not body or body in {"+", "-"}:
        raise FixedMathWatchdogError(f"{label} has no decimal integer")
    unsigned = body[1:] if body[0] in "+-" else body
    if not unsigned.isdigit():
        raise FixedMathWatchdogError(f"{label} is not a decimal integer")
    return int(body)


def _line_occurrence(
    path: str, line_number: int, role: str, bits: int
) -> Occurrence:
    return {
        "path": path,
        "line": line_number,
        "role": role,
        "bits": bits,
    }


def _scan_core(raw: bytes) -> tuple[int, list[Occurrence]]:
    lines = _ascii_lines(raw, "core source")
    path = "crates/core/src/fixed.rs"
    scalar_specs = {
        "const LN2: Fixed = Fixed::from_bits(": ("core.ln2", ");"),
        "const INV_LN2: Fixed = Fixed::from_bits(": (
            "core.inverse_ln2",
            ");",
        ),
        "const PI_BITS: Fixed = Fixed::from_bits(": ("core.pi", ");"),
        "const HALF_PI_BITS: Fixed = Fixed::from_bits(": (
            "core.half_pi",
            ");",
        ),
        "const CORDIC_INV_GAIN: Fixed = Fixed::from_bits(": (
            "core.cordic_inverse_gain",
            ");",
        ),
    }
    occurrences: list[Occurrence] = []
    seen_scalars: set[str] = set()
    iterations: int | None = None
    in_table = False
    table_values: list[tuple[int, int]] = []

    for line_number, line in enumerate(lines, 1):
        stripped = line.strip()
        for prefix, (role, suffix) in scalar_specs.items():
            if stripped.startswith(prefix):
                value_text = stripped.split("//", 1)[0].rstrip()
                value = _decimal_between(value_text, prefix, suffix, role)
                if role in seen_scalars:
                    raise FixedMathWatchdogError(
                        f"{role} appears more than once"
                    )
                seen_scalars.add(role)
                occurrences.append(
                    _line_occurrence(path, line_number, role, value)
                )

        if stripped.startswith("const CORDIC_N: usize = "):
            if iterations is not None:
                raise FixedMathWatchdogError("CORDIC_N appears more than once")
            iterations = _decimal_between(
                stripped,
                "const CORDIC_N: usize = ",
                ";",
                "CORDIC_N",
            )
        if stripped == "const CORDIC_ATAN: [Fixed; CORDIC_N] = [":
            if in_table or table_values:
                raise FixedMathWatchdogError(
                    "core CORDIC table appears more than once"
                )
            in_table = True
            continue
        if in_table and stripped == "];":
            in_table = False
            continue
        if in_table:
            value = _decimal_between(
                stripped,
                "Fixed::from_bits(",
                "),",
                "core CORDIC table entry",
            )
            table_values.append((line_number, value))

    if in_table:
        raise FixedMathWatchdogError("core CORDIC table is unterminated")
    if set(scalar_specs.values()) and len(seen_scalars) != len(scalar_specs):
        raise FixedMathWatchdogError(
            "core scalar authority declaration set is incomplete"
        )
    if iterations is None:
        raise FixedMathWatchdogError("core CORDIC_N is absent")
    if len(table_values) != iterations:
        raise FixedMathWatchdogError(
            "core table length differs from CORDIC_N"
        )
    for index, (line_number, value) in enumerate(table_values):
        occurrences.append(
            _line_occurrence(
                path, line_number, f"core.cordic_atan[{index}]", value
            )
        )
    return iterations, occurrences


def _scan_gpu(raw: bytes) -> tuple[int, list[Occurrence]]:
    lines = _ascii_lines(raw, "GPU source")
    path = "crates/gpu/src/transcendental.rs"
    scalar_counts = {
        "gpu.ln2": 0,
        "gpu.inverse_ln2": 0,
        "gpu.half_pi": 0,
    }
    occurrences: list[Occurrence] = []
    in_function = False
    table_values: dict[int, tuple[int, int]] = {}
    gain_count = 0

    for line_number, line in enumerate(lines, 1):
        stripped = line.strip()
        scalar_lines = (
            ("let ln2 = ", "i64;", "gpu.ln2"),
            ("let inv_ln2 = ", "i64;", "gpu.inverse_ln2"),
            ("let half_pi = ", "i64;", "gpu.half_pi"),
        )
        for prefix, suffix, role in scalar_lines:
            if stripped.startswith(prefix):
                source_form = stripped.split("//", 1)[0].rstrip()
                value = _decimal_between(
                    source_form, prefix, suffix, role
                )
                index = scalar_counts[role]
                scalar_counts[role] += 1
                occurrences.append(
                    _line_occurrence(
                        path, line_number, f"{role}[{index}]", value
                    )
                )

        if (
            stripped.startswith("let mut x = ")
            and "// CORDIC_INV_GAIN" in stripped
        ):
            source_form = stripped.split("//", 1)[0].rstrip()
            value = _decimal_between(
                source_form,
                "let mut x = ",
                "i64;",
                "GPU CORDIC inverse gain",
            )
            occurrences.append(
                _line_occurrence(
                    path,
                    line_number,
                    f"gpu.cordic_inverse_gain[{gain_count}]",
                    value,
                )
            )
            gain_count += 1

        if stripped == "fn cordic_atan_table() -> Array<i64> {":
            if in_function or table_values:
                raise FixedMathWatchdogError(
                    "GPU CORDIC table function appears more than once"
                )
            in_function = True
            continue
        if in_function and stripped == "}":
            in_function = False
            continue
        if in_function and stripped.startswith("a["):
            index_end = stripped.find("usize]")
            if index_end < 2:
                raise FixedMathWatchdogError(
                    "GPU CORDIC table index is malformed"
                )
            index_text = stripped[2:index_end]
            if not index_text.isdigit():
                raise FixedMathWatchdogError(
                    "GPU CORDIC table index is not decimal"
                )
            index = int(index_text)
            prefix = f"a[{index}usize] = "
            value = _decimal_between(
                stripped,
                prefix,
                "i64;",
                "GPU CORDIC table value",
            )
            if index in table_values:
                raise FixedMathWatchdogError(
                    "GPU CORDIC table index is duplicated"
                )
            table_values[index] = (line_number, value)

    if in_function:
        raise FixedMathWatchdogError(
            "GPU CORDIC table function is unterminated"
        )
    required_counts = {
        "gpu.ln2": 2,
        "gpu.inverse_ln2": 1,
        "gpu.half_pi": 4,
    }
    if scalar_counts != required_counts:
        raise FixedMathWatchdogError(
            "GPU scalar mirror occurrence counts changed"
        )
    if gain_count != 1:
        raise FixedMathWatchdogError(
            "GPU CORDIC inverse gain occurrence count changed"
        )
    if sorted(table_values) != list(range(CORDIC_ITERATIONS)):
        raise FixedMathWatchdogError(
            "GPU CORDIC table indices must be exactly 0 through 31"
        )
    for index in range(CORDIC_ITERATIONS):
        line_number, value = table_values[index]
        occurrences.append(
            _line_occurrence(
                path, line_number, f"gpu.cordic_atan[{index}]", value
            )
        )
    return len(table_values), occurrences


def _scan_decimal_tokens(raw: bytes) -> list[int]:
    text = raw.decode("utf-8")
    values: list[int] = []
    index = 0
    while index < len(text):
        negative = text[index] == "-"
        start = index + 1 if negative else index
        if start < len(text) and text[start].isdigit():
            before = text[index - 1] if index else ""
            if before.isalnum() or before == "_":
                index += 1
                continue
            end = start
            while end < len(text) and text[end].isdigit():
                end += 1
            suffix_end = end + 3 if text[end : end + 3] == "i64" else end
            after = text[suffix_end] if suffix_end < len(text) else ""
            if not (after.isalnum() or after == "_"):
                number = int(text[start:end])
                values.append(-number if negative else number)
            index = suffix_end
        else:
            index += 1
    return values


def _check_scalar_coverage(
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
    parsed = {number: 0 for number in selected}
    for occurrence in occurrences:
        number = int(occurrence["bits"])
        if number in parsed:
            parsed[number] += 1
    scanned = {number: 0 for number in selected}
    for number in _scan_decimal_tokens(core_raw) + _scan_decimal_tokens(gpu_raw):
        if number in scanned:
            scanned[number] += 1
    if scanned != parsed:
        raise FixedMathWatchdogError(
            "independent scan found an unbound scalar authority copy"
        )


def inspect_sources(
    core_raw: bytes,
    gpu_raw: bytes,
    ordered_bits: list[dict[str, int | str]],
) -> list[Occurrence]:
    core_iterations, core = _scan_core(core_raw)
    gpu_iterations, gpu = _scan_gpu(gpu_raw)
    if core_iterations != CORDIC_ITERATIONS:
        raise FixedMathWatchdogError("core CORDIC_N must equal 32")
    if gpu_iterations != CORDIC_ITERATIONS:
        raise FixedMathWatchdogError("GPU CORDIC table must have 32 entries")

    derived = {str(row["id"]): int(row["bits"]) for row in ordered_bits}
    required: dict[str, int] = {
        "core.pi": derived["pi"],
        "core.half_pi": derived["half_pi"],
        "core.ln2": derived["ln2"],
        "core.inverse_ln2": derived["inverse_ln2"],
        "core.cordic_inverse_gain": derived["cordic_inverse_gain"],
        "gpu.ln2[0]": derived["ln2"],
        "gpu.ln2[1]": derived["ln2"],
        "gpu.inverse_ln2[0]": derived["inverse_ln2"],
        "gpu.half_pi[0]": derived["half_pi"],
        "gpu.half_pi[1]": derived["half_pi"],
        "gpu.half_pi[2]": derived["half_pi"],
        "gpu.half_pi[3]": derived["half_pi"],
        "gpu.cordic_inverse_gain[0]": derived["cordic_inverse_gain"],
    }
    for index in range(CORDIC_ITERATIONS):
        required[f"core.cordic_atan[{index}]"] = derived[
            f"cordic_atan[{index}]"
        ]
        required[f"gpu.cordic_atan[{index}]"] = derived[
            f"cordic_atan[{index}]"
        ]

    occurrences = sorted(
        core + gpu,
        key=lambda row: (str(row["path"]), int(row["line"]), str(row["role"])),
    )
    observed: dict[str, int] = {}
    for row in occurrences:
        role = str(row["role"])
        if role in observed:
            raise FixedMathWatchdogError(
                "independent scan found a duplicate source role"
            )
        observed[role] = int(row["bits"])
    if observed != required:
        missing = sorted(set(required) - set(observed))
        surplus = sorted(set(observed) - set(required))
        differing = sorted(
            role
            for role in set(required) & set(observed)
            if required[role] != observed[role]
        )
        evidence: list[str] = []
        if missing:
            evidence.append("missing " + ", ".join(missing))
        if surplus:
            evidence.append("surplus " + ", ".join(surplus))
        if differing:
            evidence.append("different bits " + ", ".join(differing))
        raise FixedMathWatchdogError(
            "independent source scan differs from derivation: "
            + "; ".join(evidence)
        )
    _check_scalar_coverage(core_raw, gpu_raw, occurrences, derived)
    return occurrences


def _encode_json(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def build_receipt(core_raw: bytes, gpu_raw: bytes) -> bytes:
    derived = derive_ordered_bits()
    occurrences = inspect_sources(core_raw, gpu_raw, derived)
    return _encode_json(
        {
            "canaries": CANARIES,
            "claim_id": CLAIM_ID,
            "cordic_iterations": CORDIC_ITERATIONS,
            "derivations": DERIVATIONS,
            "frac_bits": FRAC_BITS,
            "ordered_derived_bits": derived,
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
    )


def _must_fail(
    label: str,
    core_raw: bytes,
    gpu_raw: bytes,
    derived: list[dict[str, int | str]],
) -> None:
    try:
        inspect_sources(core_raw, gpu_raw, derived)
    except FixedMathWatchdogError:
        return
    raise AssertionError(f"watchdog mutation passed: {label}")


def _single_substitution(
    raw: bytes, old: bytes, new: bytes, label: str
) -> bytes:
    if raw.count(old) != 1:
        raise AssertionError(f"{label} watchdog anchor is not unique")
    return raw.replace(old, new, 1)


def _require_receipt_match(expected: bytes, observed: bytes) -> None:
    if expected != observed:
        raise FixedMathWatchdogError(
            "independent canonical receipt comparison failed"
        )


def _must_reject_receipt(
    baseline: bytes, changed: bytes, label: str
) -> None:
    try:
        _require_receipt_match(baseline, changed)
    except FixedMathWatchdogError:
        return
    raise AssertionError(f"{label} receipt mutation passed")


def self_test() -> None:
    core_raw = CORE_PATH.read_bytes()
    gpu_raw = GPU_PATH.read_bytes()
    derived = derive_ordered_bits()
    baseline = build_receipt(core_raw, gpu_raw)
    value_by_id = {str(row["id"]): int(row["bits"]) for row in derived}

    pi = value_by_id["pi"]
    _must_fail(
        "one-bit constant",
        _single_substitution(
            core_raw,
            f"Fixed::from_bits({pi}); // pi".encode(),
            f"Fixed::from_bits({pi ^ 1}); // pi".encode(),
            "one-bit constant",
        ),
        gpu_raw,
        derived,
    )

    first = value_by_id["cordic_atan[0]"]
    second = value_by_id["cordic_atan[1]"]
    _must_fail(
        "atan omission",
        _single_substitution(
            core_raw,
            f"    Fixed::from_bits({first}),\n".encode(),
            b"",
            "atan omission",
        ),
        gpu_raw,
        derived,
    )
    _must_fail(
        "atan reorder",
        _single_substitution(
            core_raw,
            (
                f"    Fixed::from_bits({first}),\n"
                f"    Fixed::from_bits({second}),\n"
            ).encode(),
            (
                f"    Fixed::from_bits({second}),\n"
                f"    Fixed::from_bits({first}),\n"
            ).encode(),
            "atan reorder",
        ),
        gpu_raw,
        derived,
    )

    ln2 = value_by_id["ln2"]
    _must_fail(
        "GPU mirror drift",
        core_raw,
        _single_substitution(
            gpu_raw,
            f"let ln2 = {ln2}i64; // ln 2".encode(),
            f"let ln2 = {ln2 ^ 1}i64; // ln 2".encode(),
            "GPU mirror drift",
        ),
        derived,
    )

    changed_digest = build_receipt(
        core_raw, gpu_raw + b"\n// watchdog source digest canary\n"
    )
    _must_reject_receipt(
        baseline, changed_digest, "source digest"
    )

    decoded = json.loads(baseline)
    for field, value, label in (
        ("claim_id", CLAIM_ID + ".canary", "claim"),
        ("schema", SCHEMA + ".canary", "schema"),
    ):
        mutation = dict(decoded)
        mutation[field] = value
        _must_reject_receipt(baseline, _encode_json(mutation), label)
    receipt_mutation = b" " + baseline
    _must_reject_receipt(baseline, receipt_mutation, "receipt bytes")

    changed_derivation = derive_ordered_bits(pi_second_denominator=4)
    _must_fail(
        "derivation formula",
        core_raw,
        gpu_raw,
        changed_derivation,
    )
    print("fixed-math authority watchdog self-test: PASS")


def validate_checked_receipt(receipt: bytes) -> None:
    try:
        checked = RECEIPT_PATH.read_bytes()
    except FileNotFoundError as error:
        raise FixedMathWatchdogError(
            "checked-in fixed-math authority receipt is missing"
        ) from error
    try:
        _require_receipt_match(receipt + b"\n", checked)
    except FixedMathWatchdogError as error:
        raise FixedMathWatchdogError(
            "checked-in fixed-math authority receipt is stale"
        ) from error


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.receipt and args.self_test:
        parser.error("--receipt and --self-test are exclusive")
    try:
        if args.self_test:
            self_test()
            return 0
        receipt = build_receipt(CORE_PATH.read_bytes(), GPU_PATH.read_bytes())
        if args.receipt:
            sys.stdout.buffer.write(receipt + b"\n")
        else:
            validate_checked_receipt(receipt)
            print("fixed-math authority watchdog: PASS")
    except (
        AssertionError,
        FixedMathWatchdogError,
        json.JSONDecodeError,
        OSError,
    ) as error:
        print(f"fixed-math authority watchdog: FAIL: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
