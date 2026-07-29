#!/usr/bin/env python3
"""Independently derive and inspect the deterministic Q32.32 constant table."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import sys
from fractions import Fraction
from typing import Iterable


ROOT = pathlib.Path(__file__).resolve().parent.parent
CORE_PATH = ROOT / "crates" / "core" / "src" / "fixed.rs"
GPU_PATH = ROOT / "crates" / "gpu" / "src" / "transcendental.rs"
RECEIPT_PATH = ROOT / "crates" / "units" / "data" / "fixed_math_authority_receipt.json"

SCHEMA = "civsim.core.fixed-math-authority-pair.v3"
CLAIM_ID = "core.deterministic-math-table"
PRODUCER_IMPLEMENTATION = (
    "civsim.fixed-math.machin-atanh-interval-producer.v1"
)
WATCHDOG_IMPLEMENTATION = (
    "civsim.fixed-math.split-identity-square-proof-watchdog.v1"
)
FRAC_BITS = 32
CORDIC_ITERATIONS = 32
SOURCE_TEXT_CONTRACT = "ascii-canonical-git-lf-with-crlf-checkout-equivalence"

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
    "checkout-line-ending-canonicalization",
    "bare-carriage-return-rejection",
    "unbound-core-cordic-copy",
    "unbound-gpu-cordic-copy",
    "unbound-gpu-cordic-expression-copy",
    "rust-token-whitespace-copy",
    "alternate-base-literal-copy",
    "string-comment-marker-copy",
    "commented-cfg-decoy-copy",
    "non-code-literal-exclusion",
    "checked-receipt-line-ending-canonicalization",
    "raw-identifier-copy",
    "unsuffixed-gpu-copy",
    "gpu-exemption-cardinality",
    "macro-token-tree-cfg-decoy",
    "unicode-whitespace-copy",
    "commented-role-substitution",
    "macro-role-substitution",
    "conditional-compilation-role-substitution",
    "macro-exemption-substitution",
    "attribute-token-tree-role-substitution",
    "raw-conditional-attribute-rejection",
    "cfg-macro-rejection",
    "gpu-exemption-function-binding",
    "gpu-exemption-role-inventory",
    "ascii-control-whitespace-copy",
    "excluded-test-conditional-eligibility",
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


def _repository_lf_bytes(raw: bytes, label: str) -> bytes:
    """Independently reconstruct Git's canonical LF text representation."""

    canonical = bytearray()
    cursor = 0
    while cursor < len(raw):
        value = raw[cursor]
        if value != 13:
            canonical.append(value)
            cursor += 1
            continue
        if cursor + 1 >= len(raw) or raw[cursor + 1] != 10:
            raise FixedMathWatchdogError(
                f"{label} contains a non-CRLF carriage return"
            )
        canonical.append(10)
        cursor += 2
    return bytes(canonical)


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
    lines = _ascii_lines(_watchdog_role_code(raw), "core source")
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
    lines = _ascii_lines(_watchdog_role_code(raw), "GPU source")
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

        if stripped.startswith("let mut x = ") and stripped.endswith("i64;"):
            source_form = stripped.rstrip()
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


def _watchdog_character_end(raw: bytes, quote: int) -> int | None:
    cursor = quote + 1
    if cursor >= len(raw) or raw[cursor] in b"\r\n":
        return None
    if raw[cursor] == ord("\\"):
        cursor += 1
        if raw[cursor : cursor + 2] == b"u{":
            closing = raw.find(b"}", cursor + 2)
            if closing < 0:
                return None
            cursor = closing + 1
        elif raw[cursor : cursor + 1] == b"x":
            cursor += 3
        else:
            cursor += 1
    else:
        cursor += 1
        while cursor < len(raw) and raw[cursor] & 0xC0 == 0x80:
            cursor += 1
    return cursor + 1 if raw[cursor : cursor + 1] == b"'" else None


def _watchdog_raw_string_end(raw: bytes, start: int) -> int | None:
    prefix = 0
    if raw[start : start + 1] == b"r":
        prefix = 1
    elif raw[start : start + 2] in (b"br", b"cr"):
        prefix = 2
    if not prefix:
        return None
    quote = start + prefix
    while raw[quote : quote + 1] == b"#":
        quote += 1
    if raw[quote : quote + 1] != b'"':
        return None
    hashes = quote - start - prefix
    closing = raw.find(b'"' + b"#" * hashes, quote + 1)
    if closing < 0:
        raise FixedMathWatchdogError("authority source has an unterminated raw string")
    return closing + 1 + hashes


def _watchdog_string_end(raw: bytes, quote: int) -> int:
    cursor = quote + 1
    while cursor < len(raw):
        if raw[cursor] == ord("\\"):
            cursor += 2
        elif raw[cursor] == ord('"'):
            return cursor + 1
        else:
            cursor += 1
    raise FixedMathWatchdogError("authority source has an unterminated string")


def _mask_noncode_rust(raw: bytes) -> bytes:
    """Replace comments and literals while preserving structural offsets."""

    try:
        raw.decode("ascii")
    except UnicodeDecodeError as error:
        raise FixedMathWatchdogError(
            "bound authority source is not ASCII"
        ) from error
    masked = bytearray(raw)

    def erase(start: int, end: int) -> None:
        for index in range(start, end):
            if masked[index] != 10:
                masked[index] = 32

    cursor = 0
    while cursor < len(raw):
        if raw[cursor : cursor + 2] == b"//":
            end = raw.find(b"\n", cursor + 2)
            end = len(raw) if end < 0 else end
            erase(cursor, end)
            cursor = end
            continue
        if raw[cursor : cursor + 2] == b"/*":
            start = cursor
            depth = 1
            cursor += 2
            while cursor < len(raw) and depth:
                if raw[cursor : cursor + 2] == b"/*":
                    depth += 1
                    cursor += 2
                elif raw[cursor : cursor + 2] == b"*/":
                    depth -= 1
                    cursor += 2
                else:
                    cursor += 1
            if depth:
                raise FixedMathWatchdogError(
                    "authority source has an unterminated block comment"
                )
            erase(start, cursor)
            continue
        raw_end = _watchdog_raw_string_end(raw, cursor)
        if raw_end is not None:
            erase(cursor, raw_end)
            cursor = raw_end
            continue
        if raw[cursor : cursor + 2] in (b'b"', b'c"'):
            end = _watchdog_string_end(raw, cursor + 1)
            erase(cursor, end)
            cursor = end
            continue
        if raw[cursor : cursor + 1] == b'"':
            end = _watchdog_string_end(raw, cursor)
            erase(cursor, end)
            cursor = end
            continue
        if raw[cursor : cursor + 2] in (b"b'", b"c'"):
            end = _watchdog_character_end(raw, cursor + 1)
            if end is not None:
                erase(cursor, end)
                cursor = end
                continue
        if raw[cursor : cursor + 1] == b"'":
            end = _watchdog_character_end(raw, cursor)
            if end is not None:
                erase(cursor, end)
                cursor = end
                continue
        cursor += 1
    return bytes(masked)


def _watchdog_production_code(raw: bytes) -> bytes:
    code = _mask_noncode_rust(raw)
    result = bytearray(code)
    attribute = re.compile(
        rb"#\s*\[\s*(?:r#)?cfg\s*\(\s*test\s*\)\s*\]"
    )
    conditional = re.compile(
        rb"#\s*!?\s*\[\s*(?:r#)?(?:cfg|cfg_attr)\b"
    )
    module = re.compile(
        rb"#\s*\[\s*(?:r#)?cfg\s*\(\s*test\s*\)\s*\]\s*"
        rb"mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{"
    )
    covered_until = 0
    structural_cursor = 0
    delimiter_stack: list[int] = []
    openers = {ord("("): ord(")"), ord("["): ord("]"), ord("{"): ord("}")}
    closers = {ord(")"), ord("]"), ord("}")}
    for found in attribute.finditer(code):
        if found.start() < covered_until:
            continue
        for value in code[structural_cursor : found.start()]:
            if value in openers:
                delimiter_stack.append(openers[value])
            elif value in closers:
                if not delimiter_stack or delimiter_stack.pop() != value:
                    raise FixedMathWatchdogError(
                        "authority source has mismatched delimiters"
                    )
        if delimiter_stack:
            raise FixedMathWatchdogError(
                "cfg(test) exclusion is not a root module item"
            )
        header = module.match(code, found.start())
        if header is None:
            raise FixedMathWatchdogError(
                "cfg(test) is permitted only on a plainly declared module"
            )
        opening = code.rfind(b"{", found.start(), header.end())
        depth = 1
        cursor = opening + 1
        while cursor < len(code) and depth:
            if code[cursor] == ord("{"):
                depth += 1
            elif code[cursor] == ord("}"):
                depth -= 1
            cursor += 1
        if depth:
            raise FixedMathWatchdogError("cfg(test) module is unterminated")
        for index in range(found.start(), cursor):
            if result[index] != 10:
                result[index] = 32
        covered_until = cursor
        structural_cursor = cursor
    for value in code[structural_cursor:]:
        if value in openers:
            delimiter_stack.append(openers[value])
        elif value in closers:
            if not delimiter_stack or delimiter_stack.pop() != value:
                raise FixedMathWatchdogError(
                    "authority source has mismatched delimiters"
                )
    if delimiter_stack:
        raise FixedMathWatchdogError("authority source has unclosed delimiters")
    if conditional.search(bytes(result)) is not None:
        raise FixedMathWatchdogError(
            "conditional compilation is forbidden in authority sources"
        )
    if re.search(rb"\b(?:r#)?cfg\s*!\s*[\(\[\{]", result):
        raise FixedMathWatchdogError(
            "the cfg! macro is forbidden in authority sources"
        )
    return bytes(result)


WATCHDOG_RUST_KEYWORDS = {
    b"Self", b"abstract", b"as", b"async", b"await", b"become", b"box",
    b"break", b"const", b"continue", b"crate", b"do", b"dyn", b"else",
    b"enum", b"extern", b"false", b"final", b"fn", b"for", b"if",
    b"impl", b"in", b"let", b"loop", b"macro", b"match", b"mod",
    b"move", b"mut", b"override", b"priv", b"pub", b"ref", b"return",
    b"self", b"static", b"struct", b"super", b"trait", b"true", b"try",
    b"type", b"typeof", b"union", b"unsafe", b"unsized", b"use",
    b"virtual", b"where", b"while", b"yield",
}


def _watchdog_delimited_end(code: bytes, opening: int) -> int:
    pairs = {ord("("): ord(")"), ord("["): ord("]"), ord("{"): ord("}")}
    first = code[opening]
    if first not in pairs:
        raise FixedMathWatchdogError("macro tree lacks an opening delimiter")
    stack = [pairs[first]]
    cursor = opening + 1
    while cursor < len(code) and stack:
        value = code[cursor]
        if value in pairs:
            stack.append(pairs[value])
        elif value in {ord(")"), ord("]"), ord("}")}:
            if stack.pop() != value:
                raise FixedMathWatchdogError(
                    "macro tree has mismatched delimiters"
                )
        cursor += 1
    if stack:
        raise FixedMathWatchdogError("macro tree is unterminated")
    return cursor


def _watchdog_role_code(raw: bytes) -> bytes:
    """Mask macro token trees independently from the production-code scan."""

    code = _watchdog_production_code(raw)
    attribute = re.compile(rb"#\s*!?\s*(?P<opening>\[)")
    attribute_free = bytearray(code)
    covered_until = 0
    for match in attribute.finditer(code):
        if match.start() < covered_until:
            continue
        end = _watchdog_delimited_end(code, match.start("opening"))
        for index in range(match.start(), end):
            if attribute_free[index] != 10:
                attribute_free[index] = 32
        covered_until = end
    code = bytes(attribute_free)
    definition = re.compile(
        rb"\bmacro_rules\s*!\s*(?:r#)?[A-Za-z_][A-Za-z0-9_]*\s*"
        rb"(?P<opening>[\(\[\{])"
    )
    invocation = re.compile(
        rb"(?P<name>(?:r#)?[A-Za-z_][A-Za-z0-9_]*)\s*!\s*"
        rb"(?P<opening>[\(\[\{])"
    )
    candidates: list[tuple[int, re.Match[bytes]]] = [
        (match.start(), match) for match in definition.finditer(code)
    ]
    for match in invocation.finditer(code):
        name = match.group("name")
        normalized = name[2:] if name.startswith(b"r#") else name
        if normalized in WATCHDOG_RUST_KEYWORDS and not name.startswith(b"r#"):
            continue
        candidates.append((match.start(), match))
    result = bytearray(code)
    covered_until = 0
    for _start, match in sorted(candidates, key=lambda row: row[0]):
        if match.start() < covered_until:
            continue
        opening = match.start("opening")
        end = _watchdog_delimited_end(code, opening)
        for index in range(match.start(), end):
            if result[index] != 10:
                result[index] = 32
        covered_until = end
    return bytes(result)


def _watchdog_named_function_spans(
    code: bytes,
) -> dict[str, tuple[int, int]]:
    spans: dict[str, tuple[int, int]] = {}
    for name in ("fixed_ln", "fixed_sin", "fixed_cos"):
        pattern = re.compile(
            rb"\bfn\s+"
            + name.encode("ascii")
            + rb"\s*\([^{};]*\)\s*->\s*i64\s*(?P<opening>\{)"
        )
        matches = list(pattern.finditer(code))
        if len(matches) != 1:
            raise FixedMathWatchdogError(
                f"GPU exemption function {name} must appear exactly once"
            )
        opening = matches[0].start("opening")
        spans[name] = (opening, _watchdog_delimited_end(code, opening))
    return spans


WATCHDOG_INTEGER_BODY = (
    rb"(?:0[xX][0-9A-Fa-f_]+|0[oO][0-7_]+|0[bB][01_]+|[0-9][0-9_]*)"
)


def _watchdog_integer_value(literal: bytes, require_i64: bool) -> int | None:
    text = literal.decode("ascii")
    if text.endswith("i64"):
        text = text[:-3]
    elif require_i64:
        return None
    compact = text.replace("_", "")
    base = 10
    digits = compact
    if compact[:2].lower() == "0x":
        base, digits = 16, compact[2:]
    elif compact[:2].lower() == "0o":
        base, digits = 8, compact[2:]
    elif compact[:2].lower() == "0b":
        base, digits = 2, compact[2:]
    value = 0
    for character in digits.lower():
        digit = "0123456789abcdef".find(character)
        if digit < 0 or digit >= base:
            return None
        value = value * base + digit
    return value if digits else None


def _watchdog_gpu_exemption_roles(
    code: bytes, function_spans: dict[str, tuple[int, int]]
) -> dict[tuple[int, int], str]:
    roles: dict[tuple[int, int], str] = {}

    def body(name: str) -> tuple[bytes, int]:
        start, end = function_spans[name]
        return code[start:end], start

    fixed_ln, ln_offset = body("fixed_ln")
    e_declarations = list(re.finditer(rb"\blet\s+e\s*=", fixed_ln))
    word_role = re.compile(
        rb"\blet\s+e\s*=\s*msb\s*-\s*(?P<literal>"
        + WATCHDOG_INTEGER_BODY
        + rb"(?:_?i64)?)\s*;"
    )
    word_matches = list(word_role.finditer(fixed_ln))
    if len(e_declarations) != 1 or len(word_matches) != 1:
        raise FixedMathWatchdogError("fixed_ln word-width role changed")
    word = word_matches[0]
    if _watchdog_integer_value(word.group("literal"), False) != 32:
        raise FixedMathWatchdogError("fixed_ln word-width literal changed")
    roles[
        (
            ln_offset + word.start("literal"),
            ln_offset + word.end("literal"),
        )
    ] = "word-width"

    def bind_quadrant(
        function_name: str,
        initial: bytes,
        second_branch: bytes,
        third_branch: bytes,
        role: str,
    ) -> None:
        source, offset = body(function_name)
        declarations = list(re.finditer(rb"\blet\s+out\s*=", source))
        if len(declarations) != 3:
            raise FixedMathWatchdogError(
                f"{function_name} out-role inventory changed"
            )
        statements: list[tuple[bytes, int]] = []
        for declaration in declarations:
            end = source.find(b";", declaration.end())
            if end < 0:
                raise FixedMathWatchdogError(
                    f"{function_name} out role is unterminated"
                )
            statements.append(
                (source[declaration.start() : end + 1], declaration.start())
            )
        if re.sub(rb"\s+", b"", statements[0][0]) != (
            b"letout=" + initial + b";"
        ):
            raise FixedMathWatchdogError(
                f"{function_name} initial out role changed"
            )

        def select_literal(
            statement: tuple[bytes, int], expected: int, branch: bytes
        ) -> tuple[int, int]:
            pattern = re.compile(
                rb"\blet\s+out\s*=\s*select\s*\(\s*q\s*==\s*"
                rb"(?P<literal>"
                + WATCHDOG_INTEGER_BODY
                + rb"(?:_?i64)?)\s*,\s*"
                + branch
                + rb"\s*,\s*out\s*\)\s*;\s*"
            )
            match = pattern.fullmatch(statement[0])
            if match is None or _watchdog_integer_value(
                match.group("literal"), False
            ) != expected:
                raise FixedMathWatchdogError(
                    f"{function_name} quadrant role changed"
                )
            start = offset + statement[1] + match.start("literal")
            return start, offset + statement[1] + match.end("literal")

        second = select_literal(statements[1], 2, second_branch)
        select_literal(statements[2], 1, third_branch)
        roles[second] = role

    bind_quadrant(
        "fixed_sin", b"neg_c", b"neg_s", b"c", "quadrant-sine"
    )
    bind_quadrant(
        "fixed_cos", b"s", b"neg_c", b"neg_s", "quadrant-cosine"
    )
    return roles


def _scan_core_authority_literals(raw: bytes) -> list[tuple[int, int]]:
    code = _watchdog_production_code(raw)
    call = re.compile(
        rb"\bFixed\s*::\s*(?:r#)?from_bits\s*\(\s*(?P<sign>-?)\s*"
        rb"(?P<literal>" + WATCHDOG_INTEGER_BODY + rb"(?:_?i64)?)\s*\)"
    )
    values: list[tuple[int, int]] = []
    for match in call.finditer(code):
        value = _watchdog_integer_value(match.group("literal"), False)
        if value is not None:
            values.append(
                (
                    code.count(b"\n", 0, match.start("literal")) + 1,
                    -value if match.group("sign") else value,
                )
            )
    return values


def _scan_gpu_authority_literals(raw: bytes) -> list[tuple[int, int]]:
    code = _watchdog_production_code(raw)
    exemption_code = _watchdog_role_code(raw)
    function_spans = _watchdog_named_function_spans(exemption_code)
    exemption_roles = _watchdog_gpu_exemption_roles(
        exemption_code, function_spans
    )
    literal = re.compile(
        rb"(?<![A-Za-z0-9_])(?P<literal>"
        + WATCHDOG_INTEGER_BODY
        + rb"(?:_?i64)?)(?![A-Za-z0-9_])"
    )
    values: list[tuple[int, int]] = []
    exemption_counts = {
        "word-width": 0,
        "quadrant-sine": 0,
        "quadrant-cosine": 0,
    }
    for match in literal.finditer(code):
        value = _watchdog_integer_value(match.group("literal"), False)
        if value is None:
            continue
        role = exemption_roles.get(
            (match.start("literal"), match.end("literal"))
        )
        if role is None:
            values.append(
                (code.count(b"\n", 0, match.start("literal")) + 1, value)
            )
        else:
            exemption_counts[role] += 1
    if exemption_counts != {
        "word-width": 1,
        "quadrant-sine": 1,
        "quadrant-cosine": 1,
    }:
        raise FixedMathWatchdogError(
            "GPU non-Q32 literal exemption inventory changed"
        )
    return values


def _check_authority_coverage(
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
        *(derived[f"cordic_atan[{index}]"] for index in range(CORDIC_ITERATIONS)),
    }
    expected_core = sorted(
        (int(row["line"]), int(row["bits"]))
        for row in occurrences
        if row["path"] == "crates/core/src/fixed.rs"
        and int(row["bits"]) in selected
    )
    expected_gpu = sorted(
        (int(row["line"]), int(row["bits"]))
        for row in occurrences
        if row["path"] == "crates/gpu/src/transcendental.rs"
        and int(row["bits"]) in selected
    )
    scanned_core = sorted(
        row
        for row in _scan_core_authority_literals(core_raw)
        if row[1] in selected
    )
    scanned_gpu = sorted(
        row
        for row in _scan_gpu_authority_literals(gpu_raw)
        if row[1] in selected
    )
    if scanned_core != expected_core or scanned_gpu != expected_gpu:
        raise FixedMathWatchdogError(
            "independent role locations and direct authority scan differ"
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
    _check_authority_coverage(core_raw, gpu_raw, occurrences, derived)
    return occurrences


def _encode_json(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def build_receipt(core_raw: bytes, gpu_raw: bytes) -> bytes:
    core_raw = _repository_lf_bytes(core_raw, "core source")
    gpu_raw = _repository_lf_bytes(gpu_raw, "GPU source")
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
            "source_text_contract": SOURCE_TEXT_CONTRACT,
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
    core_raw = _repository_lf_bytes(CORE_PATH.read_bytes(), "core source")
    gpu_raw = _repository_lf_bytes(GPU_PATH.read_bytes(), "GPU source")
    derived = derive_ordered_bits()
    baseline = build_receipt(core_raw, gpu_raw)
    alternate_checkout = build_receipt(
        core_raw.replace(b"\n", b"\r\n"),
        gpu_raw.replace(b"\n", b"\r\n"),
    )
    if alternate_checkout != baseline:
        raise AssertionError("watchdog CRLF checkout changed the receipt")
    try:
        build_receipt(core_raw, gpu_raw + b"\r")
    except FixedMathWatchdogError:
        pass
    else:
        raise AssertionError("watchdog accepted a bare carriage return")
    _validate_checked_receipt_bytes(baseline, baseline + b"\r\n")
    try:
        _validate_checked_receipt_bytes(baseline, baseline + b"\r")
    except FixedMathWatchdogError:
        pass
    else:
        raise AssertionError("watchdog accepted a bare-CR checked receipt")
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
    last = value_by_id[f"cordic_atan[{CORDIC_ITERATIONS - 1}]"]
    _must_fail(
        "unbound core CORDIC copy",
        core_raw
        + (
            "\nconst UNBOUND_CORDIC_COPY: Fixed = "
            f"Fixed::from_bits({last});\n"
        ).encode(),
        gpu_raw,
        derived,
    )
    _must_fail(
        "unbound GPU CORDIC copy",
        core_raw,
        gpu_raw
        + (
            "\nfn unbound_cordic_copy() -> i64 {\n"
            f"    let unbound = {last}i64;\n"
            "    unbound\n"
            "}\n"
        ).encode(),
        derived,
    )
    second = value_by_id["cordic_atan[1]"]
    _must_fail(
        "unbound GPU CORDIC expression copy",
        core_raw,
        gpu_raw
        + (
            "\nfn unbound_cordic_expression() -> i64 {\n"
            f"    consume_bits({second}i64)\n"
            "}\n"
        ).encode(),
        derived,
    )
    _must_fail(
        "Rust-token whitespace copy",
        core_raw
        + f"\nconst TOKEN_SPACE_COPY: Fixed = Fixed::from_bits ({last});\n".encode(),
        gpu_raw,
        derived,
    )
    _must_fail(
        "alternate-base core copy",
        core_raw
        + "\nconst BINARY_COPY: Fixed = Fixed::from_bits (\n0b10_i64\n);\n".encode(),
        gpu_raw,
        derived,
    )
    for label, spelling in (
        ("binary", "0b10i64"),
        ("octal", "0o2_i64"),
        ("hexadecimal", "0x2i64"),
        ("underscored decimal", "2_i64"),
    ):
        _must_fail(
            f"alternate-base GPU {label} copy",
            core_raw,
            gpu_raw + f"\nfn alternate_copy() -> i64 {{ {spelling} }}\n".encode(),
            derived,
        )
    _must_fail(
        "string comment-marker GPU copy",
        core_raw,
        gpu_raw
        + (
            "\nfn marker_copy() -> i64 {\n"
            '    let marker = "//";\n'
            f"    let copied = {last}i64;\n"
            "    copied\n"
            "}\n"
        ).encode(),
        derived,
    )
    _must_fail(
        "commented cfg(test) decoy copy",
        core_raw
        + (
            "\n/*\n#[cfg(test)]\nmod decoy {\n*/\n"
            "const CFG_DECOY_COPY: Fixed = "
            f"Fixed::from_bits({last});\n"
            "/*\n}\n*/\n"
        ).encode(),
        gpu_raw,
        derived,
    )
    _must_fail(
        "raw identifier copy",
        core_raw
        + f"\nconst RAW_COPY: Fixed = Fixed::r#from_bits({last});\n".encode(),
        gpu_raw,
        derived,
    )
    for label, addition in (
        ("unsuffixed GPU return copy", "\nfn bare_copy() -> i64 { 2 }\n"),
        (
            "unsuffixed GPU typed-binding copy",
            "\nfn typed_copy() -> i64 { let copied: i64 = 2; copied }\n",
        ),
    ):
        _must_fail(
            label,
            core_raw,
            gpu_raw + addition.encode(),
            derived,
        )
    word_width_line = b"    let e = msb - 32i64;\n"
    sine_quadrant_line = b"    let out = select(q == 2i64, neg_s, out);\n"
    cosine_quadrant_line = b"    let out = select(q == 2i64, neg_c, out);\n"
    for label, mutated_gpu in (
        (
            "duplicate word-width exemption",
            _single_substitution(
                gpu_raw,
                word_width_line,
                word_width_line + word_width_line,
                "word-width exemption duplication",
            ),
        ),
        (
            "removed word-width exemption",
            _single_substitution(
                gpu_raw,
                word_width_line,
                b"",
                "word-width exemption removal",
            ),
        ),
        (
            "duplicate sine-quadrant exemption",
            _single_substitution(
                gpu_raw,
                sine_quadrant_line,
                sine_quadrant_line + sine_quadrant_line,
                "sine-quadrant exemption duplication",
            ),
        ),
        (
            "removed cosine-quadrant exemption",
            _single_substitution(
                gpu_raw,
                cosine_quadrant_line,
                b"",
                "cosine-quadrant exemption removal",
            ),
        ),
    ):
        _must_fail(
            label,
            core_raw,
            mutated_gpu,
            derived,
        )
    _must_fail(
        "cfg(test) macro token-tree decoy",
        core_raw
        + (
            "\nmacro_rules! cfg_token_passthrough {\n"
            "    ($hash:tt $attr:tt $module:tt $name:ident "
            "{ $value:expr }) => {\n"
            "        const MACRO_CFG_COPY: Fixed = $value;\n"
            "    };\n"
            "}\n"
            "cfg_token_passthrough! {\n"
            "    #[cfg(test)] mod decoy { "
            f"Fixed::from_bits({last}) }}\n"
            "}\n"
        ).encode(),
        gpu_raw,
        derived,
    )
    _must_fail(
        "Unicode whitespace copy",
        core_raw
        + (
            f"\nconst UNICODE_WS_COPY: Fixed = "
            f"Fixed::from_bits\u0085({last});\n"
        ).encode("utf-8"),
        gpu_raw,
        derived,
    )
    _must_fail(
        "ASCII control-whitespace copy",
        core_raw
        + f"\nconst CONTROL_WS_COPY: Fixed = Fixed::from_bits\v(\f{last});\n".encode(),
        gpu_raw,
        derived,
    )
    _must_fail(
        "commented core role substitution",
        _single_substitution(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi}); // pi\n".encode(),
            (
                "/*\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi}); // pi\n"
                "*/\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi ^ 1}) }};\n"
                "const PI_AUTHORITY_DECOY_COPY: Fixed = "
                f"Fixed::from_bits({pi});\n"
            ).encode(),
            "commented core role substitution",
        ),
        gpu_raw,
        derived,
    )
    ln2_role = value_by_id["ln2"]
    _must_fail(
        "commented GPU role substitution",
        core_raw,
        _single_substitution(
            gpu_raw,
            f"    let ln2 = {ln2_role}i64; // ln 2\n".encode(),
            (
                "    /*\n"
                f"    let ln2 = {ln2_role}i64; // ln 2\n"
                "    */\n"
                f"    let ln2 = {{ {ln2_role ^ 1}i64 }};\n"
                "    let ln2_authority_decoy = "
                f"{ln2_role}i64;\n"
            ).encode(),
            "commented GPU role substitution",
        ),
        derived,
    )
    _must_fail(
        "macro token-tree GPU role substitution",
        core_raw,
        _single_substitution(
            gpu_raw,
            f"    let ln2 = {ln2_role}i64; // ln 2\n".encode(),
            (
                "    macro_rules! hide_authority_role {\n"
                "        ($($tokens:tt)*) => {};\n"
                "    }\n"
                "    hide_authority_role! {\n"
                f"        let ln2 = {ln2_role}i64;\n"
                "    }\n"
                f"    let  ln2 = {ln2_role ^ 1}i64;\n"
            ).encode(),
            "macro token-tree GPU role substitution",
        ),
        derived,
    )
    _must_fail(
        "conditional-compilation core role substitution",
        _single_substitution(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi}); // pi\n".encode(),
            (
                "#[cfg(any())]\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi}); // pi\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi ^ 1}) }};\n"
            ).encode(),
            "conditional-compilation core role substitution",
        ),
        gpu_raw,
        derived,
    )
    _must_fail(
        "macro token-tree GPU exemption substitution",
        core_raw,
        _single_substitution(
            gpu_raw,
            word_width_line,
            b"",
            "macro token-tree GPU exemption removal",
        )
        + (
            "\nmacro_rules! discard_exemption_context {\n"
            "    ($($tokens:tt)*) => {};\n"
            "}\n"
            "discard_exemption_context! {\n"
            "    let e = msb - 32i64;\n"
            "}\n"
        ).encode(),
        derived,
    )
    _must_fail(
        "tool-attribute core role substitution",
        _single_substitution(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi}); // pi\n".encode(),
            (
                "#[rustfmt::authority_decoy(\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi});\n"
                ")]\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi ^ 1}) }};\n"
            ).encode(),
            "tool-attribute core role substitution",
        ),
        gpu_raw,
        derived,
    )
    _must_fail(
        "raw conditional-attribute core role substitution",
        _single_substitution(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi}); // pi\n".encode(),
            (
                "#[r#cfg(any())]\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi}); // pi\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi ^ 1}) }};\n"
            ).encode(),
            "raw conditional-attribute core role substitution",
        ),
        gpu_raw,
        derived,
    )
    _must_fail(
        "cfg! macro",
        core_raw + b"\nconst CFG_DEPENDENT_COPY: bool = cfg!(test);\n",
        gpu_raw,
        derived,
    )
    _must_fail(
        "moved GPU word-width exemption",
        core_raw,
        _single_substitution(
            gpu_raw,
            word_width_line,
            b"    let e = msb - 31i64;\n",
            "moved GPU word-width exemption source",
        )
        + (
            "\nfn unrelated_exemption_context(msb: i64) {\n"
            "    let e = msb - 32i64;\n"
            "    let _ = e;\n"
            "}\n"
        ).encode(),
        derived,
    )
    _must_fail(
        "same-function GPU word-width decoy",
        core_raw,
        _single_substitution(
            _single_substitution(
                gpu_raw,
                word_width_line,
                b"    let e = msb - 31i64;\n",
                "same-function GPU word-width source",
            ),
            b"    let main = q32_mul(e64, ln2) + ln_m;\n",
            b"    let main = q32_mul(e64, ln2) + ln_m;\n"
            b"    let e = msb - 32i64;\n",
            "same-function GPU word-width decoy",
        ),
        derived,
    )
    expected_occurrences = inspect_sources(core_raw, gpu_raw, derived)
    non_code_occurrences = inspect_sources(
        core_raw
        + (
            f"\n// Fixed::from_bits ({last})\n"
            f'const NON_CODE_NOTE: &str = "Fixed::from_bits({last})";\n'
            "#[cfg(test)]\n"
            "mod nested_conditional_note {\n"
            "    #[cfg(any())]\n"
            "    fn disabled_note() {}\n"
            "}\n"
        ).encode(),
        gpu_raw
        + (
            f"\n// {last:#x}i64\n"
            f'fn non_code_note() -> &\'static str {{ "// {last}i64" }}\n'
        ).encode(),
        derived,
    )
    if non_code_occurrences != expected_occurrences:
        raise AssertionError("non-code authority text changed occurrence coverage")

    first = value_by_id["cordic_atan[0]"]
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


def _validate_checked_receipt_bytes(receipt: bytes, checked: bytes) -> None:
    canonical_checked = _repository_lf_bytes(checked, "checked receipt")
    _require_receipt_match(receipt + b"\n", canonical_checked)


def validate_checked_receipt(receipt: bytes) -> None:
    try:
        checked = RECEIPT_PATH.read_bytes()
    except FileNotFoundError as error:
        raise FixedMathWatchdogError(
            "checked-in fixed-math authority receipt is missing"
        ) from error
    try:
        _validate_checked_receipt_bytes(receipt, checked)
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
