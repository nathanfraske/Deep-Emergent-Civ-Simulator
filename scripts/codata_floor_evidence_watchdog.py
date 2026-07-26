#!/usr/bin/env python3
"""Independently scan the CODATA 2018 rows that back the measured floor."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import sys
from typing import Any, Iterable


ROOT = pathlib.Path(__file__).resolve().parent.parent
FACTS_PATH = (
    ROOT / "crates" / "units" / "data" / "codata_2018_floor_facts.tsv"
)
RECEIPT_PATH = (
    ROOT
    / "crates"
    / "units"
    / "data"
    / "codata_2018_floor_evidence_receipt.json"
)

PAIR_SCHEMA = "civsim.units.codata-2018-floor-evidence-pair.v2"
FACTS_SCHEMA = "civsim.units.codata-2018-floor-facts.v1"
CLAIM_ID = "floor.codata-factual-row-binding"
PRODUCER_IMPLEMENTATION = "civsim.units.codata-fixed-column-producer.v2"
WATCHDOG_IMPLEMENTATION = "civsim.units.codata-record-scan-watchdog.v2"
WATCHDOG_CANARY_SCHEMA = "civsim.units.codata-record-scan-canary-execution.v2"
WATCHDOG_CANARY_MUTATION_SCHEMA = (
    "civsim.units.codata-record-scan-canary-mutation.v1"
)
SOURCE_ID = "nist_codata_2018_ascii"
SOURCE_SHA256 = (
    "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1"
)
SOURCE_BYTES = 40_689
LIVE_URL = (
    "https://physics.nist.gov/cuu/Constants/ArchiveASCII/allascii_2018.txt"
)
ARCHIVE_URL = (
    "https://web.archive.org/web/20241219074053id_/"
    "https://physics.nist.gov/cuu/Constants/ArchiveASCII/allascii_2018.txt"
)
TEXT_CONTRACT = "ascii-lf-no-carriage-return"
FACTS_RELATIVE = "crates/units/data/codata_2018_floor_facts.tsv"

EXPECTED_COLUMNS = (
    "symbol",
    "source_line",
    "byte_offset",
    "byte_length",
    "row_sha256",
    "source_anchor",
    "normalized_value",
    "absolute_uncertainty",
    "source_unit",
    "si_dimension",
)

# This is an independent reviewed pin, not data loaded from the facts file.
# The watchdog scans records separated by whitespace runs rather than using
# the producer's fixed-width CODATA columns.
EXPECTED_ROWS: tuple[dict[str, str | int], ...] = (
    {
        "symbol": "alpha",
        "source_line": 135,
        "byte_offset": 14391,
        "byte_length": 110,
        "row_sha256": (
            "e8a9302a81c562198cc75c33f9fc8824685fdf1aafb1e25416417257dee6882f"
        ),
        "source_anchor": "fine-structure constant",
        "normalized_value": "7.2973525693e-3",
        "absolute_uncertainty": "1.1e-12",
        "source_unit": "1",
        "si_dimension": "0,0,0,0,0,0,0",
    },
    {
        "symbol": "G",
        "source_line": 263,
        "byte_offset": 28935,
        "byte_length": 124,
        "row_sha256": (
            "1d28f15a0fda0e5df2a023367b72bbd14a412cacf497e508fa77592b11a32304"
        ),
        "source_anchor": "Newtonian constant of gravitation",
        "normalized_value": "6.67430e-11",
        "absolute_uncertainty": "1.5e-15",
        "source_unit": "m^3 kg^-1 s^-2",
        "si_dimension": "3,-1,-2,0,0,0,0",
    },
    {
        "symbol": "m_e",
        "source_line": 106,
        "byte_offset": 11120,
        "byte_length": 112,
        "row_sha256": (
            "c1cf07efdb813c1162a5d4ed19e93c7a48c5dbccacf4030ef88b05ed39c3f039"
        ),
        "source_anchor": "electron mass",
        "normalized_value": "9.1093837015e-31",
        "absolute_uncertainty": "2.8e-40",
        "source_unit": "kg",
        "si_dimension": "0,1,0,0,0,0,0",
    },
)

CANARIES = (
    "facts-bare-carriage-return",
    "facts-schema-mutation",
    "facts-source-digest-mutation",
    "facts-row-omission",
    "facts-row-duplication",
    "facts-row-reordering",
    "facts-source-line-mutation",
    "facts-row-offset-mutation",
    "facts-row-span-digest-mutation",
    "facts-value-mutation",
    "facts-uncertainty-mutation",
    "facts-unit-mutation",
    "facts-dimension-mutation",
    "facts-symbol-mutation",
)


class FloorEvidenceWatchdogError(ValueError):
    """The independent factual-row checker refused the evidence."""

    def __init__(self, message: str, *, code: str = "unclassified") -> None:
        super().__init__(message)
        self.code = code


def _canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _canonical_repo_text(raw: bytes, label: str) -> bytes:
    output = bytearray()
    cursor = 0
    while cursor < len(raw):
        value = raw[cursor]
        if value != 13:
            output.append(value)
            cursor += 1
            continue
        if cursor + 1 >= len(raw) or raw[cursor + 1] != 10:
            raise FloorEvidenceWatchdogError(
                f"{label} contains a non-CRLF carriage return",
                code="text.bare-carriage-return",
            )
        output.append(10)
        cursor += 2
    return bytes(output)


def _facts_rows(raw: bytes) -> tuple[list[dict[str, str]], bytes]:
    canonical = _canonical_repo_text(raw, "facts")
    try:
        lines = canonical.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        raise FloorEvidenceWatchdogError("facts are not ASCII") from error
    if len(lines) != 9:
        relation = "short" if len(lines) < 9 else "long"
        raise FloorEvidenceWatchdogError(
            "facts must contain six headers and three rows",
            code=f"facts.line-count.{relation}",
        )
    expected_headers = (
        ("schema", FACTS_SCHEMA),
        ("source_id", SOURCE_ID),
        ("source_sha256", SOURCE_SHA256),
        ("source_bytes", str(SOURCE_BYTES)),
        ("source_line_endings", "LF"),
    )
    for line, expected in zip(lines[:5], expected_headers):
        fields = tuple(line.split("\t"))
        if len(fields) != 2 or fields[0] != expected[0]:
            raise FloorEvidenceWatchdogError(
                f"facts header {expected[0]!r} differs from its independent pin",
                code="facts.header.shape-or-order",
            )
        if fields[1] != expected[1]:
            raise FloorEvidenceWatchdogError(
                f"facts header {expected[0]!r} differs from its independent pin",
                code=f"facts.header.{expected[0]}",
            )
    columns = lines[5].split("\t")
    if not columns or columns[0] != "columns":
        raise FloorEvidenceWatchdogError("facts omit the column declaration")
    if tuple(columns[1:]) != EXPECTED_COLUMNS:
        raise FloorEvidenceWatchdogError(
            "facts columns differ from the closed schema",
            code="facts.columns",
        )

    rows: list[dict[str, str]] = []
    for line in lines[6:]:
        fields = line.split("\t")
        if len(fields) != len(EXPECTED_COLUMNS) + 1 or fields[0] != "row":
            raise FloorEvidenceWatchdogError(
                "facts row has the wrong arity",
                code="facts.row.shape",
            )
        rows.append(dict(zip(EXPECTED_COLUMNS, fields[1:])))
    expected_as_text = [
        {key: str(value) for key, value in expected.items()}
        for expected in EXPECTED_ROWS
    ]
    observed_symbols = [row["symbol"] for row in rows]
    expected_symbols = [row["symbol"] for row in expected_as_text]
    if observed_symbols != expected_symbols:
        refusal_code = (
            "facts.row-order"
            if sorted(observed_symbols) == sorted(expected_symbols)
            else "facts.row.symbol"
        )
        raise FloorEvidenceWatchdogError(
            "facts rows differ from the independent reviewed row profile",
            code=refusal_code,
        )
    field_codes = {
        "source_line": "facts.row.source-line",
        "byte_offset": "facts.row.byte-offset",
        "byte_length": "facts.row.byte-length",
        "row_sha256": "facts.row.span-digest",
        "source_anchor": "facts.row.source-anchor",
        "normalized_value": "facts.row.value",
        "absolute_uncertainty": "facts.row.uncertainty",
        "source_unit": "facts.row.unit",
        "si_dimension": "facts.row.dimension",
    }
    for row, expected in zip(rows, expected_as_text):
        for field in EXPECTED_COLUMNS:
            if row[field] != expected[field]:
                raise FloorEvidenceWatchdogError(
                    "facts rows differ from the independent reviewed row profile",
                    code=field_codes.get(field, f"facts.row.{field}"),
                )
    return rows, canonical


def _decimal_without_grouping(raw: str) -> str:
    compact = raw.replace(" ", "").replace("E", "e")
    if not re.fullmatch(r"[0-9]+(?:\.[0-9]+)?(?:e[+-][0-9]+)?", compact):
        raise FloorEvidenceWatchdogError(f"invalid CODATA decimal {raw!r}")
    return compact


def _absolute_scientific(raw: str) -> str:
    compact = _decimal_without_grouping(raw)
    if "e" in compact:
        coefficient, exponent_text = compact.split("e", 1)
        exponent = int(exponent_text)
    else:
        coefficient = compact
        exponent = 0
    whole, dot, fraction = coefficient.partition(".")
    digits = whole + fraction
    decimal_places = len(fraction) if dot else 0
    first = next((index for index, digit in enumerate(digits) if digit != "0"), None)
    if first is None:
        return "0"
    significant = digits[first:].rstrip("0")
    scientific_exponent = exponent + len(whole) - first - 1
    mantissa = significant[0]
    if len(significant) > 1:
        mantissa += "." + significant[1:]
    return f"{mantissa}e{scientific_exponent:+d}".replace("e+", "e")


def _dimension_from_unit(unit: str) -> str:
    if not unit:
        return "0,0,0,0,0,0,0"
    axes = {"m": 0, "kg": 1, "s": 2, "A": 3, "K": 4, "mol": 5, "cd": 6}
    exponents = [0] * 7
    for token in unit.split():
        match = re.fullmatch(r"([A-Za-z]+)(?:\^(-?[0-9]+))?", token)
        if match is None or match.group(1) not in axes:
            raise FloorEvidenceWatchdogError(f"unrecognized CODATA unit token {token!r}")
        exponents[axes[match.group(1)]] += int(match.group(2) or "1")
    return ",".join(str(value) for value in exponents)


def _source_lines(raw: bytes) -> list[tuple[int, int, bytes]]:
    if len(raw) != SOURCE_BYTES:
        raise FloorEvidenceWatchdogError(
            f"source has {len(raw)} bytes, expected {SOURCE_BYTES}"
        )
    if _sha256(raw) != SOURCE_SHA256:
        raise FloorEvidenceWatchdogError("source SHA-256 differs from the NIST pin")
    if b"\r" in raw:
        raise FloorEvidenceWatchdogError("source violates the LF-only byte contract")
    try:
        raw.decode("ascii")
    except UnicodeDecodeError as error:
        raise FloorEvidenceWatchdogError("source is not ASCII") from error

    records: list[tuple[int, int, bytes]] = []
    offset = 0
    for line_number, with_ending in enumerate(raw.splitlines(keepends=True), 1):
        if not with_ending.endswith(b"\n"):
            raise FloorEvidenceWatchdogError("source final record lacks LF")
        line = with_ending[:-1]
        records.append((line_number, offset, line))
        offset += len(with_ending)
    if offset != len(raw):
        raise FloorEvidenceWatchdogError("source record scan did not consume all bytes")
    return records


def inspect_source(raw: bytes) -> list[dict[str, str]]:
    by_anchor = {
        str(expected["source_anchor"]): expected for expected in EXPECTED_ROWS
    }
    found: dict[str, dict[str, str]] = {}
    for line_number, offset, line in _source_lines(raw):
        fields = re.split(rb" {2,}", line.rstrip(b" "))
        if len(fields) not in (3, 4):
            continue
        try:
            anchor = fields[0].decode("ascii")
        except UnicodeDecodeError:
            continue
        expected = by_anchor.get(anchor)
        if expected is None:
            continue
        if anchor in found:
            raise FloorEvidenceWatchdogError(f"source duplicates row {anchor!r}")
        value = _decimal_without_grouping(fields[1].decode("ascii"))
        uncertainty = _absolute_scientific(fields[2].decode("ascii"))
        unit = fields[3].decode("ascii") if len(fields) == 4 else ""
        source_unit = unit or "1"
        row = {
            "symbol": str(expected["symbol"]),
            "source_line": str(line_number),
            "byte_offset": str(offset),
            "byte_length": str(len(line)),
            "row_sha256": _sha256(line),
            "source_anchor": anchor,
            "normalized_value": value,
            "absolute_uncertainty": uncertainty,
            "source_unit": source_unit,
            "si_dimension": _dimension_from_unit(unit),
        }
        expected_text = {key: str(item) for key, item in expected.items()}
        if row != expected_text:
            raise FloorEvidenceWatchdogError(
                f"source row {anchor!r} differs from the independent row profile"
            )
        found[anchor] = row
    missing = [anchor for anchor in by_anchor if anchor not in found]
    if missing:
        raise FloorEvidenceWatchdogError(
            "source omits required rows: " + ", ".join(missing)
        )
    return [found[str(expected["source_anchor"])] for expected in EXPECTED_ROWS]


def evidence_document(rows: list[dict[str, str]]) -> dict[str, Any]:
    return {
        "source_id": SOURCE_ID,
        "source_sha256": SOURCE_SHA256,
        "source_bytes": SOURCE_BYTES,
        "source_text_contract": TEXT_CONTRACT,
        "rows": rows,
    }


def result_digest(evidence_sha256: str) -> str:
    return _sha256(
        _canonical_json(
            {
                "implementation": WATCHDOG_IMPLEMENTATION,
                "claim_id": CLAIM_ID,
                "evidence_sha256": evidence_sha256,
                "verdict": "admitted",
            }
        )
    )


def _changed_payload(canary_id: str, baseline: bytes, candidate: bytes) -> bytes:
    if candidate == baseline:
        raise AssertionError(f"canary did not mutate its input: {canary_id}")
    return candidate


def _replace_once(
    canary_id: str,
    baseline: bytes,
    old: bytes,
    new: bytes,
) -> bytes:
    if baseline.count(old) != 1:
        raise AssertionError(
            f"canary replacement anchor is not unique: {canary_id}"
        )
    return _changed_payload(
        canary_id,
        baseline,
        baseline.replace(old, new, 1),
    )


def _mutation_sha256(canary_id: str, candidate: bytes) -> str:
    return _sha256(
        _canonical_json(
            {
                "schema": WATCHDOG_CANARY_MUTATION_SCHEMA,
                "canary_id": canary_id,
                "input_kind": "facts",
                "candidate_bytes": len(candidate),
                "candidate_sha256": _sha256(candidate),
            }
        )
    )


def _observe_refusal(
    canary_id: str,
    candidate: bytes,
    expected_refusal_code: str,
    operation: Any,
) -> dict[str, str]:
    try:
        operation()
    except FloorEvidenceWatchdogError as error:
        if error.code != expected_refusal_code:
            raise AssertionError(
                f"canary {canary_id} reached refusal code {error.code!r}, "
                f"expected {expected_refusal_code!r}"
            ) from error
        return {
            "id": canary_id,
            "mutation_sha256": _mutation_sha256(canary_id, candidate),
            "outcome": "refused",
            "refusal_code": error.code,
            "error_type": type(error).__name__,
            "error_sha256": _sha256(str(error).encode("utf-8")),
        }
    raise AssertionError(f"canary was admitted: {canary_id}")


def canary_observations() -> list[dict[str, str]]:
    facts = FACTS_PATH.read_bytes()
    lines = facts.splitlines()
    if len(lines) != 9:
        raise FloorEvidenceWatchdogError(
            "checked facts cannot seed the watchdog canary suite"
        )

    reordered = lines[:]
    reordered[7], reordered[8] = reordered[8], reordered[7]
    cases = (
        (
            CANARIES[0],
            _changed_payload(CANARIES[0], facts, facts + b"\r"),
            "text.bare-carriage-return",
        ),
        (
            CANARIES[1],
            _replace_once(
                CANARIES[1],
                facts,
                FACTS_SCHEMA.encode("ascii"),
                b"changed-schema",
            ),
            "facts.header.schema",
        ),
        (
            CANARIES[2],
            _replace_once(
                CANARIES[2],
                facts,
                SOURCE_SHA256.encode("ascii"),
                b"0" * 64,
            ),
            "facts.header.source_sha256",
        ),
        (
            CANARIES[3],
            _changed_payload(
                CANARIES[3],
                facts,
                b"\n".join(lines[:-1]) + b"\n",
            ),
            "facts.line-count.short",
        ),
        (
            CANARIES[4],
            _changed_payload(
                CANARIES[4],
                facts,
                facts + lines[-1] + b"\n",
            ),
            "facts.line-count.long",
        ),
        (
            CANARIES[5],
            _changed_payload(
                CANARIES[5],
                facts,
                b"\n".join(reordered) + b"\n",
            ),
            "facts.row-order",
        ),
        (
            CANARIES[6],
            _replace_once(
                CANARIES[6],
                facts,
                b"\t135\t",
                b"\t136\t",
            ),
            "facts.row.source-line",
        ),
        (
            CANARIES[7],
            _replace_once(
                CANARIES[7],
                facts,
                b"\t14391\t",
                b"\t14392\t",
            ),
            "facts.row.byte-offset",
        ),
        (
            CANARIES[8],
            _replace_once(
                CANARIES[8],
                facts,
                b"e8a9302a81c562198cc75c33f9fc8824685fdf1aafb1e25416417257dee6882f",
                b"08a9302a81c562198cc75c33f9fc8824685fdf1aafb1e25416417257dee6882f",
            ),
            "facts.row.span-digest",
        ),
        (
            CANARIES[9],
            _replace_once(
                CANARIES[9],
                facts,
                b"\t7.2973525693e-3\t",
                b"\t7.2973525694e-3\t",
            ),
            "facts.row.value",
        ),
        (
            CANARIES[10],
            _replace_once(
                CANARIES[10],
                facts,
                b"\t1.1e-12\t",
                b"\t1.2e-12\t",
            ),
            "facts.row.uncertainty",
        ),
        (
            CANARIES[11],
            _replace_once(
                CANARIES[11],
                facts,
                b"\t1\t0,0,0,0,0,0,0\n",
                b"\tkg\t0,0,0,0,0,0,0\n",
            ),
            "facts.row.unit",
        ),
        (
            CANARIES[12],
            _replace_once(
                CANARIES[12],
                facts,
                b"\t0,0,0,0,0,0,0\n",
                b"\t0,1,0,0,0,0,0\n",
            ),
            "facts.row.dimension",
        ),
        (
            CANARIES[13],
            _replace_once(
                CANARIES[13],
                facts,
                b"row\talpha\t",
                b"row\tbeta\t",
            ),
            "facts.row.symbol",
        ),
    )
    observations = [
        _observe_refusal(
            canary_id,
            candidate,
            expected_refusal_code,
            lambda candidate=candidate: _facts_rows(candidate),
        )
        for canary_id, candidate, expected_refusal_code in cases
    ]
    if tuple(observation["id"] for observation in observations) != CANARIES:
        raise AssertionError("watchdog canary execution order differs from its declaration")
    return observations


def canary_digest() -> str:
    observations = canary_observations()
    return _sha256(
        _canonical_json(
            {
                "schema": WATCHDOG_CANARY_SCHEMA,
                "implementation": WATCHDOG_IMPLEMENTATION,
                "observations": observations,
            }
        )
    )


def _required_dict(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise FloorEvidenceWatchdogError(f"{label} must be an object")
    return value


def _required_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise FloorEvidenceWatchdogError(f"{label} must be nonempty text")
    return value


def _pair_digest(receipt: dict[str, Any]) -> str:
    unsigned = dict(receipt)
    unsigned.pop("pair_sha256", None)
    return _sha256(_canonical_json(unsigned))


def validate_receipt(receipt_raw: bytes, facts_raw: bytes) -> None:
    rows, canonical_facts = _facts_rows(facts_raw)
    canonical_receipt = _canonical_repo_text(receipt_raw, "receipt")
    try:
        receipt = json.loads(canonical_receipt)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise FloorEvidenceWatchdogError("receipt is not canonical JSON text") from error
    receipt = _required_dict(receipt, "receipt")

    evidence_sha256 = _sha256(_canonical_json(evidence_document(rows)))
    expected_top = {
        "schema",
        "claim_id",
        "source",
        "facts",
        "evidence_sha256",
        "producer",
        "watchdog",
        "online_audit",
        "pair_sha256",
    }
    if set(receipt) != expected_top:
        raise FloorEvidenceWatchdogError("receipt fields differ from the closed schema")
    if receipt["schema"] != PAIR_SCHEMA or receipt["claim_id"] != CLAIM_ID:
        raise FloorEvidenceWatchdogError("receipt identity differs from the pair contract")
    if receipt["evidence_sha256"] != evidence_sha256:
        raise FloorEvidenceWatchdogError("receipt evidence digest is stale")

    source = _required_dict(receipt["source"], "receipt.source")
    if source != {
        "id": SOURCE_ID,
        "sha256": SOURCE_SHA256,
        "bytes": SOURCE_BYTES,
        "text_contract": TEXT_CONTRACT,
    }:
        raise FloorEvidenceWatchdogError("receipt source metadata differs from its pin")
    facts = _required_dict(receipt["facts"], "receipt.facts")
    if facts != {
        "path": FACTS_RELATIVE,
        "schema": FACTS_SCHEMA,
        "sha256": _sha256(canonical_facts),
        "row_count": len(EXPECTED_ROWS),
    }:
        raise FloorEvidenceWatchdogError("receipt facts binding is stale")

    producer = _required_dict(receipt["producer"], "receipt.producer")
    if set(producer) != {
        "implementation",
        "result_sha256",
        "canary_schema",
        "canary_count",
        "canary_sha256",
    }:
        raise FloorEvidenceWatchdogError("receipt producer fields differ from the schema")
    if producer["implementation"] != PRODUCER_IMPLEMENTATION:
        raise FloorEvidenceWatchdogError("receipt producer identity differs from its pin")
    if (
        producer["canary_schema"]
        != "civsim.units.codata-fixed-column-canary-execution.v2"
        or producer["canary_count"] != 13
    ):
        raise FloorEvidenceWatchdogError(
            "receipt producer canary profile differs from its pin"
        )
    for field in ("result_sha256", "canary_sha256"):
        if not re.fullmatch(r"[0-9a-f]{64}", _required_text(producer[field], field)):
            raise FloorEvidenceWatchdogError(f"receipt producer {field} is not SHA-256")

    watchdog = _required_dict(receipt["watchdog"], "receipt.watchdog")
    if watchdog != {
        "implementation": WATCHDOG_IMPLEMENTATION,
        "result_sha256": result_digest(evidence_sha256),
        "canary_schema": WATCHDOG_CANARY_SCHEMA,
        "canary_count": len(CANARIES),
        "canary_sha256": canary_digest(),
    }:
        raise FloorEvidenceWatchdogError("receipt watchdog result differs from live code")

    audit = _required_dict(receipt["online_audit"], "receipt.online_audit")
    if set(audit) != {
        "live_url",
        "archive_url",
        "audited_at_utc",
        "live_archive_equal",
    }:
        raise FloorEvidenceWatchdogError("receipt online-audit fields differ from schema")
    if audit["live_url"] != LIVE_URL or audit["archive_url"] != ARCHIVE_URL:
        raise FloorEvidenceWatchdogError("receipt endpoint identity differs from its pin")
    if audit["live_archive_equal"] is not True:
        raise FloorEvidenceWatchdogError("receipt does not attest live/archive equality")
    if re.fullmatch(
        r"20[0-9]{2}-[01][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-5][0-9]Z",
        _required_text(audit["audited_at_utc"], "audited_at_utc"),
    ) is None:
        raise FloorEvidenceWatchdogError("receipt audit time is not canonical UTC")
    if receipt["pair_sha256"] != _pair_digest(receipt):
        raise FloorEvidenceWatchdogError("receipt pair digest is stale")


def _must_refuse(label: str, operation: Any) -> None:
    try:
        operation()
    except (FloorEvidenceWatchdogError, json.JSONDecodeError, UnicodeDecodeError):
        return
    raise AssertionError(f"canary was admitted: {label}")


def self_test() -> None:
    facts = FACTS_PATH.read_bytes()
    receipt = RECEIPT_PATH.read_bytes()
    validate_receipt(receipt, facts)
    if len(canary_observations()) != len(CANARIES):
        raise AssertionError("watchdog did not execute every declared canary")

    changed_facts = facts.replace(b"\t1.1e-12\t", b"\t1.2e-12\t", 1)
    _must_refuse(
        "uncertainty mutation",
        lambda: validate_receipt(receipt, changed_facts),
    )
    changed_offset = facts.replace(b"\t14391\t", b"\t14392\t", 1)
    _must_refuse(
        "offset mutation",
        lambda: validate_receipt(receipt, changed_offset),
    )
    changed_receipt = bytearray(receipt)
    changed_receipt[len(changed_receipt) // 2] ^= 1
    _must_refuse(
        "receipt byte mutation",
        lambda: validate_receipt(bytes(changed_receipt), facts),
    )
    bare_cr = facts + b"\r"
    _must_refuse(
        "bare carriage return",
        lambda: validate_receipt(receipt, bare_cr),
    )
    print("CODATA floor evidence watchdog self-test: PASS")


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--audit-source", type=pathlib.Path)
    parser.add_argument("--result", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test and (args.audit_source is not None or args.result):
        parser.error("--self-test is exclusive")
    if args.result and args.audit_source is None:
        parser.error("--result requires --audit-source")
    try:
        if args.self_test:
            self_test()
            return 0
        if args.audit_source is not None:
            rows = inspect_source(args.audit_source.read_bytes())
            evidence_sha256 = _sha256(_canonical_json(evidence_document(rows)))
            result = {
                "implementation": WATCHDOG_IMPLEMENTATION,
                "evidence_sha256": evidence_sha256,
                "result_sha256": result_digest(evidence_sha256),
                "canary_schema": WATCHDOG_CANARY_SCHEMA,
                "canary_count": len(CANARIES),
                "canary_sha256": canary_digest(),
            }
            if args.result:
                print(json.dumps(result, sort_keys=True, separators=(",", ":")))
            else:
                print("CODATA floor evidence watchdog source audit: PASS")
            return 0
        validate_receipt(RECEIPT_PATH.read_bytes(), FACTS_PATH.read_bytes())
        print("CODATA floor evidence watchdog: PASS")
    except (
        AssertionError,
        FloorEvidenceWatchdogError,
        json.JSONDecodeError,
        OSError,
    ) as error:
        print(f"CODATA floor evidence watchdog: FAIL: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
