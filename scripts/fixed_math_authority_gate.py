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


class FixedMathAuthorityError(ValueError):
    """The fixed-math authority claim or receipt failed closed."""


Interval = tuple[Fraction, Fraction]
Occurrence = dict[str, int | str]
RustToken = tuple[bytes, int, int]


def _canonical_repository_text(raw: bytes, label: str) -> bytes:
    """Return the canonical Git LF form while refusing ambiguous CR bytes."""

    without_pairs = raw.replace(b"\r\n", b"")
    if b"\r" in without_pairs:
        raise FixedMathAuthorityError(
            f"{label} contains a bare carriage return"
        )
    return raw.replace(b"\r\n", b"\n")


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
    code = _producer_production_code(raw)
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
        match = _unique_match(pattern, code, role)
        occurrences.append(
            {
                "path": "crates/core/src/fixed.rs",
                "line": _line_number(raw, match.start(1)),
                "role": role,
                "bits": int(match.group(1)),
            }
        )

    count_match = _unique_match(
        rb"^const CORDIC_N: usize = (\d+);", code, "core.CORDIC_N"
    )
    iterations = int(count_match.group(1))
    table_match = _unique_match(
        rb"^const CORDIC_ATAN: \[Fixed; CORDIC_N\] = \[\r?\n"
        rb"(?P<body>[\s\S]*?)^\];",
        code,
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
    code = _producer_production_code(raw)
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
        matches = list(re.finditer(pattern, code, flags=re.MULTILINE))
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
        rb"^\s*let mut x = (-?\d+)i64;",
        code,
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
        code,
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


def _producer_character_end(raw: bytes, quote: int) -> int | None:
    cursor = quote + 1
    if cursor >= len(raw) or raw[cursor] in b"\r\n":
        return None
    if raw[cursor] == ord("\\"):
        cursor += 1
        if cursor >= len(raw):
            return None
        if raw[cursor : cursor + 2] == b"u{":
            closing = raw.find(b"}", cursor + 2)
            if closing < 0:
                return None
            cursor = closing + 1
        elif raw[cursor] == ord("x"):
            cursor += 3
        else:
            cursor += 1
    else:
        cursor += 1
        while cursor < len(raw) and raw[cursor] & 0xC0 == 0x80:
            cursor += 1
    return cursor + 1 if raw[cursor : cursor + 1] == b"'" else None


def _producer_raw_string_end(raw: bytes, start: int) -> int | None:
    if raw[start : start + 2] in (b"br", b"cr"):
        quote = start + 2
    elif raw[start : start + 1] == b"r":
        quote = start + 1
    else:
        return None
    while raw[quote : quote + 1] == b"#":
        quote += 1
    if raw[quote : quote + 1] != b'"':
        return None
    terminator = b'"' + b"#" * (quote - start - (2 if start + 1 < quote and raw[start : start + 2] in (b"br", b"cr") else 1))
    closing = raw.find(terminator, quote + 1)
    if closing < 0:
        raise FixedMathAuthorityError("Rust authority source has an unterminated raw string")
    return closing + len(terminator)


def _producer_quoted_string_end(raw: bytes, quote: int) -> int:
    cursor = quote + 1
    while cursor < len(raw):
        if raw[cursor] == ord("\\"):
            cursor += 2
        elif raw[cursor] == ord('"'):
            return cursor + 1
        else:
            cursor += 1
    raise FixedMathAuthorityError("Rust authority source has an unterminated string")


def _producer_rust_tokens(raw: bytes) -> list[RustToken]:
    """Tokenize authority-relevant Rust while omitting non-code bytes."""

    if not raw.isascii():
        raise FixedMathAuthorityError(
            "bound Rust authority sources must contain ASCII bytes only"
        )
    tokens: list[RustToken] = []
    cursor = 0
    while cursor < len(raw):
        if raw[cursor : cursor + 2] == b"//":
            newline = raw.find(b"\n", cursor + 2)
            cursor = len(raw) if newline < 0 else newline + 1
            continue
        if raw[cursor : cursor + 2] == b"/*":
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
                raise FixedMathAuthorityError(
                    "Rust authority source has an unterminated block comment"
                )
            continue
        raw_string_end = _producer_raw_string_end(raw, cursor)
        if raw_string_end is not None:
            cursor = raw_string_end
            continue
        if raw[cursor : cursor + 2] in (b'b"', b'c"'):
            cursor = _producer_quoted_string_end(raw, cursor + 1)
            continue
        if raw[cursor : cursor + 1] == b'"':
            cursor = _producer_quoted_string_end(raw, cursor)
            continue
        if raw[cursor : cursor + 2] in (b"b'", b"c'"):
            character_end = _producer_character_end(raw, cursor + 1)
            if character_end is not None:
                cursor = character_end
                continue
        if raw[cursor : cursor + 1] == b"'":
            character_end = _producer_character_end(raw, cursor)
            if character_end is not None:
                cursor = character_end
                continue
        if (
            raw[cursor : cursor + 2] == b"r#"
            and raw[cursor + 2 : cursor + 3]
            in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_"
        ):
            end = cursor + 3
            while (
                end < len(raw)
                and raw[end]
                in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_0123456789"
            ):
                end += 1
            tokens.append((raw[cursor + 2 : end], cursor, end))
            cursor = end
            continue
        value = raw[cursor]
        if value in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_":
            end = cursor + 1
            while end < len(raw) and raw[end] in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_0123456789":
                end += 1
            tokens.append((raw[cursor:end], cursor, end))
            cursor = end
            continue
        if value in b"0123456789":
            end = cursor + 1
            while end < len(raw) and raw[end] in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_0123456789":
                end += 1
            tokens.append((raw[cursor:end], cursor, end))
            cursor = end
            continue
        pair = raw[cursor : cursor + 2]
        if pair in (b"::", b"==", b"!=", b"<=", b">=", b"->", b"&&", b"||"):
            tokens.append((pair, cursor, cursor + 2))
            cursor += 2
            continue
        if value not in b" \t\r\n\v\f":
            tokens.append((raw[cursor : cursor + 1], cursor, cursor + 1))
        cursor += 1
    return tokens


def _producer_production_tokens(raw: bytes) -> list[RustToken]:
    tokens = _producer_rust_tokens(raw)
    marker = (b"#", b"[", b"cfg", b"(", b"test", b")", b"]")
    openers = {b"(": b")", b"[": b"]", b"{": b"}"}
    closers = {b")", b"]", b"}"}
    delimiter_stack: list[bytes] = []
    production: list[RustToken] = []
    cursor = 0
    while cursor < len(tokens):
        token_text = tokens[cursor][0]
        is_test_marker = (
            tuple(token[0] for token in tokens[cursor : cursor + len(marker)])
            == marker
        )
        conditional_attribute = (
            tuple(token[0] for token in tokens[cursor : cursor + 3])
            in ((b"#", b"[", b"cfg"), (b"#", b"[", b"cfg_attr"))
            or tuple(token[0] for token in tokens[cursor : cursor + 4])
            in (
                (b"#", b"!", b"[", b"cfg"),
                (b"#", b"!", b"[", b"cfg_attr"),
            )
        )
        if conditional_attribute and not is_test_marker:
            raise FixedMathAuthorityError(
                "conditional compilation is forbidden in bound authority sources"
            )
        if (
            token_text == b"cfg"
            and tokens[cursor + 1 : cursor + 2]
            and tokens[cursor + 1][0] == b"!"
            and tokens[cursor + 2 : cursor + 3]
            and tokens[cursor + 2][0] in {b"(", b"[", b"{"}
        ):
            raise FixedMathAuthorityError(
                "the cfg! macro is forbidden in bound authority sources"
            )
        if not is_test_marker:
            if token_text in openers:
                delimiter_stack.append(openers[token_text])
            elif token_text in closers:
                if not delimiter_stack or delimiter_stack.pop() != token_text:
                    raise FixedMathAuthorityError(
                        "Rust authority source has mismatched delimiters"
                    )
            production.append(tokens[cursor])
            cursor += 1
            continue
        if delimiter_stack:
            raise FixedMathAuthorityError(
                "cfg(test) exclusion is permitted only on a root module item"
            )
        header = cursor + len(marker)
        if (
            header + 2 >= len(tokens)
            or tokens[header][0] != b"mod"
            or not re.fullmatch(rb"[A-Za-z_][A-Za-z0-9_]*", tokens[header + 1][0])
            or tokens[header + 2][0] != b"{"
        ):
            raise FixedMathAuthorityError(
                "cfg(test) is permitted only on a plainly declared module"
            )
        depth = 1
        cursor = header + 3
        while cursor < len(tokens) and depth:
            if tokens[cursor][0] == b"{":
                depth += 1
            elif tokens[cursor][0] == b"}":
                depth -= 1
            cursor += 1
        if depth:
            raise FixedMathAuthorityError("cfg(test) module is unterminated")
    if delimiter_stack:
        raise FixedMathAuthorityError("Rust authority source has unclosed delimiters")
    return production


RUST_KEYWORDS = {
    b"Self", b"abstract", b"as", b"async", b"await", b"become", b"box",
    b"break", b"const", b"continue", b"crate", b"do", b"dyn", b"else",
    b"enum", b"extern", b"false", b"final", b"fn", b"for", b"if",
    b"impl", b"in", b"let", b"loop", b"macro", b"match", b"mod",
    b"move", b"mut", b"override", b"priv", b"pub", b"ref", b"return",
    b"self", b"static", b"struct", b"super", b"trait", b"true", b"try",
    b"type", b"typeof", b"union", b"unsafe", b"unsized", b"use",
    b"virtual", b"where", b"while", b"yield",
}


def _producer_delimited_end(tokens: list[RustToken], opening: int) -> int:
    pairs = {b"(": b")", b"[": b"]", b"{": b"}"}
    first = tokens[opening][0]
    if first not in pairs:
        raise FixedMathAuthorityError("macro token tree lacks an opening delimiter")
    stack = [pairs[first]]
    cursor = opening + 1
    while cursor < len(tokens) and stack:
        text = tokens[cursor][0]
        if text in pairs:
            stack.append(pairs[text])
        elif text in {b")", b"]", b"}"}:
            if stack.pop() != text:
                raise FixedMathAuthorityError(
                    "macro token tree has mismatched delimiters"
                )
        cursor += 1
    if stack:
        raise FixedMathAuthorityError("macro token tree is unterminated")
    return cursor


def _producer_role_tokens(raw: bytes) -> list[RustToken]:
    """Exclude macro token trees from eligible authority-role syntax."""

    tokens = _producer_production_tokens(raw)
    result: list[RustToken] = []
    cursor = 0
    while cursor < len(tokens):
        if (
            cursor + 1 < len(tokens)
            and tokens[cursor][0] == b"#"
            and tokens[cursor + 1][0] == b"["
        ):
            cursor = _producer_delimited_end(tokens, cursor + 1)
            continue
        if (
            cursor + 2 < len(tokens)
            and tokens[cursor][0] == b"#"
            and tokens[cursor + 1][0] == b"!"
            and tokens[cursor + 2][0] == b"["
        ):
            cursor = _producer_delimited_end(tokens, cursor + 2)
            continue
        if (
            cursor + 3 < len(tokens)
            and tokens[cursor][0] == b"macro_rules"
            and tokens[cursor + 1][0] == b"!"
            and re.fullmatch(rb"[A-Za-z_][A-Za-z0-9_]*", tokens[cursor + 2][0])
            and tokens[cursor + 3][0] in {b"(", b"[", b"{"}
        ):
            cursor = _producer_delimited_end(tokens, cursor + 3)
            continue
        if (
            tokens[cursor][0] == b"!"
            and cursor > 0
            and cursor + 1 < len(tokens)
            and tokens[cursor + 1][0] in {b"(", b"[", b"{"}
            and re.fullmatch(rb"[A-Za-z_][A-Za-z0-9_]*", tokens[cursor - 1][0])
        ):
            previous = tokens[cursor - 1]
            raw_identifier = raw[previous[1] : previous[2]].startswith(b"r#")
            if previous[0] not in RUST_KEYWORDS or raw_identifier:
                cursor = _producer_delimited_end(tokens, cursor + 1)
                continue
        result.append(tokens[cursor])
        cursor += 1
    return result


def _producer_named_function_spans(
    tokens: list[RustToken], names: set[bytes]
) -> dict[bytes, tuple[int, int]]:
    spans: dict[bytes, tuple[int, int]] = {}
    cursor = 0
    while cursor + 1 < len(tokens):
        if tokens[cursor][0] != b"fn" or tokens[cursor + 1][0] not in names:
            cursor += 1
            continue
        name = tokens[cursor + 1][0]
        if name in spans:
            raise FixedMathAuthorityError(
                f"GPU exemption function {name.decode()} appears more than once"
            )
        opening = cursor + 2
        while opening < len(tokens) and tokens[opening][0] not in {b"{", b";"}:
            opening += 1
        if opening >= len(tokens) or tokens[opening][0] != b"{":
            raise FixedMathAuthorityError(
                f"GPU exemption function {name.decode()} has no body"
            )
        end = _producer_delimited_end(tokens, opening)
        spans[name] = (tokens[opening][1], tokens[end - 1][2])
        cursor = end
    if set(spans) != names:
        missing = sorted(name.decode() for name in names - set(spans))
        raise FixedMathAuthorityError(
            "GPU exemption function set changed: " + ", ".join(missing)
        )
    return spans


def _producer_production_code(raw: bytes) -> bytes:
    """Project eligible role tokens onto an offset-preserving source view."""

    result = bytearray(10 if value == 10 else 32 for value in raw)
    for _text, start, end in _producer_role_tokens(raw):
        result[start:end] = raw[start:end]
    return bytes(result)


def _parse_rust_integer_token(token: bytes, require_i64: bool) -> int | None:
    text = token.decode("ascii")
    if text.endswith("i64"):
        text = text[:-3]
    elif require_i64:
        return None
    valid = any(
        re.fullmatch(pattern, text)
        for pattern in (
            r"0[xX][0-9A-Fa-f_]+",
            r"0[oO][0-7_]+",
            r"0[bB][01_]+",
            r"[0-9][0-9_]*",
        )
    )
    if not valid:
        return None
    compact = text.replace("_", "")
    try:
        if compact.lower().startswith("0x"):
            return int(compact[2:], 16)
        if compact.lower().startswith("0o"):
            return int(compact[2:], 8)
        if compact.lower().startswith("0b"):
            return int(compact[2:], 2)
        return int(compact, 10)
    except ValueError:
        return None


def _core_authority_literals(
    raw: bytes, selected: set[int]
) -> list[tuple[int, int]]:
    literals: list[tuple[int, int]] = []
    tokens = _producer_production_tokens(raw)
    for index in range(len(tokens) - 5):
        if tuple(token[0] for token in tokens[index : index + 4]) != (
            b"Fixed",
            b"::",
            b"from_bits",
            b"(",
        ):
            continue
        literal = index + 4
        sign = 1
        if tokens[literal][0] == b"-":
            sign = -1
            literal += 1
        if literal + 1 >= len(tokens) or tokens[literal + 1][0] != b")":
            continue
        parsed = _parse_rust_integer_token(tokens[literal][0], False)
        if parsed is not None and sign * parsed in selected:
            literals.append(
                (_line_number(raw, tokens[literal][1]), sign * parsed)
            )
    return literals


def _producer_gpu_exemption_roles(
    tokens: list[RustToken],
    function_spans: dict[bytes, tuple[int, int]],
) -> dict[tuple[int, int], str]:
    def body(name: bytes) -> list[RustToken]:
        start, end = function_spans[name]
        return [token for token in tokens if start < token[1] < end]

    def statement(source: list[RustToken], start: int) -> list[RustToken]:
        end = start
        while end < len(source) and source[end][0] != b";":
            end += 1
        if end >= len(source):
            raise FixedMathAuthorityError("GPU exemption statement is unterminated")
        return source[start : end + 1]

    roles: dict[tuple[int, int], str] = {}
    fixed_ln = body(b"fixed_ln")
    e_starts = [
        index
        for index in range(len(fixed_ln) - 2)
        if [token[0] for token in fixed_ln[index : index + 3]]
        == [b"let", b"e", b"="]
    ]
    if len(e_starts) != 1:
        raise FixedMathAuthorityError(
            "fixed_ln must contain exactly one let-e declaration"
        )
    word_statement = statement(fixed_ln, e_starts[0])
    if [token[0] for token in word_statement[:5]] != [
        b"let", b"e", b"=", b"msb", b"-",
    ] or len(word_statement) != 7:
        raise FixedMathAuthorityError("fixed_ln word-width role changed")
    word_literal = word_statement[5]
    if _parse_rust_integer_token(word_literal[0], False) != 32:
        raise FixedMathAuthorityError("fixed_ln word-width literal changed")
    roles[(word_literal[1], word_literal[2])] = "word-width"

    def bind_quadrant(
        function_name: bytes,
        initial: bytes,
        second_branch: bytes,
        third_branch: bytes,
        role: str,
    ) -> None:
        source = body(function_name)
        starts = [
            index
            for index in range(len(source) - 2)
            if [token[0] for token in source[index : index + 3]]
            == [b"let", b"out", b"="]
        ]
        if len(starts) != 3:
            raise FixedMathAuthorityError(
                f"{function_name.decode()} out-role inventory changed"
            )
        statements = [statement(source, start) for start in starts]
        if [token[0] for token in statements[0]] != [
            b"let", b"out", b"=", initial, b";",
        ]:
            raise FixedMathAuthorityError(
                f"{function_name.decode()} initial out role changed"
            )

        def select_literal(
            selected: list[RustToken], expected: int, branch: bytes
        ) -> RustToken:
            if len(selected) != 14 or [token[0] for token in selected[:7]] != [
                b"let", b"out", b"=", b"select", b"(", b"q", b"==",
            ] or [token[0] for token in selected[8:]] != [
                b",", branch, b",", b"out", b")", b";",
            ]:
                raise FixedMathAuthorityError(
                    f"{function_name.decode()} quadrant role changed"
                )
            literal = selected[7]
            if _parse_rust_integer_token(literal[0], False) != expected:
                raise FixedMathAuthorityError(
                    f"{function_name.decode()} quadrant selector changed"
                )
            return literal

        second = select_literal(statements[1], 2, second_branch)
        select_literal(statements[2], 1, third_branch)
        roles[(second[1], second[2])] = role

    bind_quadrant(b"fixed_sin", b"neg_c", b"neg_s", b"c", "quadrant-sine")
    bind_quadrant(b"fixed_cos", b"s", b"neg_c", b"neg_s", "quadrant-cosine")
    return roles


def _gpu_authority_literals(
    raw: bytes, selected: set[int]
) -> list[tuple[int, int]]:
    literals: list[tuple[int, int]] = []
    exemption_counts = {
        "word-width": 0,
        "quadrant-sine": 0,
        "quadrant-cosine": 0,
    }
    tokens = _producer_production_tokens(raw)
    role_tokens = _producer_role_tokens(raw)
    function_spans = _producer_named_function_spans(
        role_tokens, {b"fixed_ln", b"fixed_sin", b"fixed_cos"}
    )
    exemption_roles = _producer_gpu_exemption_roles(
        role_tokens, function_spans
    )
    for token in tokens:
        value = _parse_rust_integer_token(token[0], False)
        if value is None:
            continue
        role = exemption_roles.get((token[1], token[2]))
        if role is not None:
            exemption_counts[role] += 1
        elif value in selected:
            literals.append((_line_number(raw, token[1]), value))
    if exemption_counts != {
        "word-width": 1,
        "quadrant-sine": 1,
        "quadrant-cosine": 1,
    }:
        raise FixedMathAuthorityError(
            "GPU non-Q32 literal exemption inventory changed"
        )
    return literals


def _validate_unbound_authority_copies(
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
    scanned_core = sorted(_core_authority_literals(core_raw, selected))
    scanned_gpu = sorted(_gpu_authority_literals(gpu_raw, selected))
    if scanned_core != expected_core or scanned_gpu != expected_gpu:
        raise FixedMathAuthorityError(
            "fixed-point authority roles and direct source locations differ"
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
    _validate_unbound_authority_copies(
        core_raw, gpu_raw, occurrences, derived
    )
    return occurrences


def _canonical_json(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def build_receipt(core_raw: bytes, gpu_raw: bytes) -> bytes:
    core_raw = _canonical_repository_text(core_raw, "core source")
    gpu_raw = _canonical_repository_text(gpu_raw, "GPU source")
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
    core_raw = _canonical_repository_text(CORE_PATH.read_bytes(), "core source")
    gpu_raw = _canonical_repository_text(GPU_PATH.read_bytes(), "GPU source")
    ordered_bits = derive_ordered_bits()
    baseline = build_receipt(core_raw, gpu_raw)
    crlf_receipt = build_receipt(
        core_raw.replace(b"\n", b"\r\n"),
        gpu_raw.replace(b"\n", b"\r\n"),
    )
    if crlf_receipt != baseline:
        raise AssertionError("CRLF checkout changed the canonical receipt")
    try:
        build_receipt(core_raw + b"\r", gpu_raw)
    except FixedMathAuthorityError:
        pass
    else:
        raise AssertionError("bare carriage-return mutation passed")
    _validate_checked_receipt_bytes(baseline, baseline + b"\r\n")
    try:
        _validate_checked_receipt_bytes(baseline, baseline + b"\r")
    except FixedMathAuthorityError:
        pass
    else:
        raise AssertionError("checked receipt accepted a bare carriage return")

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
    atan_last = next(
        int(row["bits"])
        for row in ordered_bits
        if row["id"] == f"cordic_atan[{CORDIC_ITERATIONS - 1}]"
    )
    _require_rejected(
        "unbound core CORDIC copy",
        core_raw
        + (
            "\nconst UNBOUND_CORDIC_COPY: Fixed = "
            f"Fixed::from_bits({atan_last});\n"
        ).encode(),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "unbound GPU CORDIC copy",
        core_raw,
        gpu_raw
        + (
            "\nfn unbound_cordic_copy() -> i64 {\n"
            f"    let unbound = {atan_last}i64;\n"
            "    unbound\n"
            "}\n"
        ).encode(),
        ordered_bits,
    )
    _require_rejected(
        "unbound GPU CORDIC expression copy",
        core_raw,
        gpu_raw
        + (
            "\nfn unbound_cordic_expression() -> i64 {\n"
            f"    consume_bits({atan_one}i64)\n"
            "}\n"
        ).encode(),
        ordered_bits,
    )
    _require_rejected(
        "Rust-token whitespace copy",
        core_raw
        + f"\nconst TOKEN_SPACE_COPY: Fixed = Fixed::from_bits ({atan_last});\n".encode(),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "alternate-base core copy",
        core_raw
        + "\nconst BINARY_COPY: Fixed = Fixed::from_bits (\n0b10_i64\n);\n".encode(),
        gpu_raw,
        ordered_bits,
    )
    for label, spelling in (
        ("binary", "0b10i64"),
        ("octal", "0o2_i64"),
        ("hexadecimal", "0x2i64"),
        ("underscored decimal", "2_i64"),
    ):
        _require_rejected(
            f"alternate-base GPU {label} copy",
            core_raw,
            gpu_raw + f"\nfn alternate_copy() -> i64 {{ {spelling} }}\n".encode(),
            ordered_bits,
        )
    _require_rejected(
        "string comment-marker GPU copy",
        core_raw,
        gpu_raw
        + (
            "\nfn marker_copy() -> i64 {\n"
            '    let marker = "//";\n'
            f"    let copied = {atan_last}i64;\n"
            "    copied\n"
            "}\n"
        ).encode(),
        ordered_bits,
    )
    _require_rejected(
        "commented cfg(test) decoy copy",
        core_raw
        + (
            "\n/*\n#[cfg(test)]\nmod decoy {\n*/\n"
            "const CFG_DECOY_COPY: Fixed = "
            f"Fixed::from_bits({atan_last});\n"
            "/*\n}\n*/\n"
        ).encode(),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "raw identifier copy",
        core_raw
        + f"\nconst RAW_COPY: Fixed = Fixed::r#from_bits({atan_last});\n".encode(),
        gpu_raw,
        ordered_bits,
    )
    for label, addition in (
        ("unsuffixed GPU return copy", "\nfn bare_copy() -> i64 { 2 }\n"),
        (
            "unsuffixed GPU typed-binding copy",
            "\nfn typed_copy() -> i64 { let copied: i64 = 2; copied }\n",
        ),
    ):
        _require_rejected(
            label,
            core_raw,
            gpu_raw + addition.encode(),
            ordered_bits,
        )
    word_width_line = b"    let e = msb - 32i64;\n"
    sine_quadrant_line = b"    let out = select(q == 2i64, neg_s, out);\n"
    cosine_quadrant_line = b"    let out = select(q == 2i64, neg_c, out);\n"
    for label, mutated_gpu in (
        (
            "duplicate word-width exemption",
            _replace_once(
                gpu_raw,
                word_width_line,
                word_width_line + word_width_line,
                "word-width exemption duplication",
            ),
        ),
        (
            "removed word-width exemption",
            _replace_once(
                gpu_raw,
                word_width_line,
                b"",
                "word-width exemption removal",
            ),
        ),
        (
            "duplicate sine-quadrant exemption",
            _replace_once(
                gpu_raw,
                sine_quadrant_line,
                sine_quadrant_line + sine_quadrant_line,
                "sine-quadrant exemption duplication",
            ),
        ),
        (
            "removed cosine-quadrant exemption",
            _replace_once(
                gpu_raw,
                cosine_quadrant_line,
                b"",
                "cosine-quadrant exemption removal",
            ),
        ),
    ):
        _require_rejected(
            label,
            core_raw,
            mutated_gpu,
            ordered_bits,
        )
    _require_rejected(
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
            f"Fixed::from_bits({atan_last}) }}\n"
            "}\n"
        ).encode(),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "Unicode whitespace copy",
        core_raw
        + (
            f"\nconst UNICODE_WS_COPY: Fixed = "
            f"Fixed::from_bits\u0085({atan_last});\n"
        ).encode("utf-8"),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "ASCII control-whitespace copy",
        core_raw
        + f"\nconst CONTROL_WS_COPY: Fixed = Fixed::from_bits\v(\f{atan_last});\n".encode(),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "commented core role substitution",
        _replace_once(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits}); // pi\n".encode(),
            (
                "/*\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits}); // pi\n"
                "*/\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi_bits ^ 1}) }};\n"
                "const PI_AUTHORITY_DECOY_COPY: Fixed = "
                f"Fixed::from_bits({pi_bits});\n"
            ).encode(),
            "commented core role substitution",
        ),
        gpu_raw,
        ordered_bits,
    )
    ln2_role_bits = next(
        int(row["bits"]) for row in ordered_bits if row["id"] == "ln2"
    )
    _require_rejected(
        "commented GPU role substitution",
        core_raw,
        _replace_once(
            gpu_raw,
            f"    let ln2 = {ln2_role_bits}i64; // ln 2\n".encode(),
            (
                "    /*\n"
                f"    let ln2 = {ln2_role_bits}i64; // ln 2\n"
                "    */\n"
                f"    let ln2 = {{ {ln2_role_bits ^ 1}i64 }};\n"
                "    let ln2_authority_decoy = "
                f"{ln2_role_bits}i64;\n"
            ).encode(),
            "commented GPU role substitution",
        ),
        ordered_bits,
    )
    _require_rejected(
        "macro token-tree GPU role substitution",
        core_raw,
        _replace_once(
            gpu_raw,
            f"    let ln2 = {ln2_role_bits}i64; // ln 2\n".encode(),
            (
                "    macro_rules! hide_authority_role {\n"
                "        ($($tokens:tt)*) => {};\n"
                "    }\n"
                "    hide_authority_role! {\n"
                f"        let ln2 = {ln2_role_bits}i64;\n"
                "    }\n"
                f"    let  ln2 = {ln2_role_bits ^ 1}i64;\n"
            ).encode(),
            "macro token-tree GPU role substitution",
        ),
        ordered_bits,
    )
    _require_rejected(
        "conditional-compilation core role substitution",
        _replace_once(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits}); // pi\n".encode(),
            (
                "#[cfg(any())]\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits}); // pi\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi_bits ^ 1}) }};\n"
            ).encode(),
            "conditional-compilation core role substitution",
        ),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "macro token-tree GPU exemption substitution",
        core_raw,
        _replace_once(
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
        ordered_bits,
    )
    _require_rejected(
        "tool-attribute core role substitution",
        _replace_once(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits}); // pi\n".encode(),
            (
                "#[rustfmt::authority_decoy(\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits});\n"
                ")]\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi_bits ^ 1}) }};\n"
            ).encode(),
            "tool-attribute core role substitution",
        ),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "raw conditional-attribute core role substitution",
        _replace_once(
            core_raw,
            f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits}); // pi\n".encode(),
            (
                "#[r#cfg(any())]\n"
                f"const PI_BITS: Fixed = Fixed::from_bits({pi_bits}); // pi\n"
                "const PI_BITS: Fixed = { "
                f"Fixed::from_bits({pi_bits ^ 1}) }};\n"
            ).encode(),
            "raw conditional-attribute core role substitution",
        ),
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "cfg! macro",
        core_raw + b"\nconst CFG_DEPENDENT_COPY: bool = cfg!(test);\n",
        gpu_raw,
        ordered_bits,
    )
    _require_rejected(
        "moved GPU word-width exemption",
        core_raw,
        _replace_once(
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
        ordered_bits,
    )
    _require_rejected(
        "same-function GPU word-width decoy",
        core_raw,
        _replace_once(
            _replace_once(
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
        ordered_bits,
    )
    expected_occurrences = parse_and_validate_sources(
        core_raw, gpu_raw, ordered_bits
    )
    non_code_occurrences = parse_and_validate_sources(
        core_raw
        + (
            f"\n// Fixed::from_bits ({atan_last})\n"
            f'const NON_CODE_NOTE: &str = "Fixed::from_bits({atan_last})";\n'
            "#[cfg(test)]\n"
            "mod nested_conditional_note {\n"
            "    #[cfg(any())]\n"
            "    fn disabled_note() {}\n"
            "}\n"
        ).encode(),
        gpu_raw
        + (
            f"\n// {atan_last:#x}i64\n"
            f'fn non_code_note() -> &\'static str {{ "// {atan_last}i64" }}\n'
        ).encode(),
        ordered_bits,
    )
    if non_code_occurrences != expected_occurrences:
        raise AssertionError("non-code authority text changed occurrence coverage")
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


def _validate_checked_receipt_bytes(receipt: bytes, checked: bytes) -> None:
    canonical_checked = _canonical_repository_text(checked, "checked receipt")
    require_exact_agreement(receipt + b"\n", canonical_checked)


def validate_checked_receipt(receipt: bytes) -> None:
    try:
        checked = RECEIPT_PATH.read_bytes()
    except FileNotFoundError as error:
        raise FixedMathAuthorityError(
            "checked-in fixed-math authority receipt is missing; "
            "run with --generate after reviewing both proofs"
        ) from error
    try:
        _validate_checked_receipt_bytes(receipt, checked)
    except FixedMathAuthorityError as error:
        raise FixedMathAuthorityError(
            "checked-in fixed-math authority receipt is stale"
        ) from error


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
