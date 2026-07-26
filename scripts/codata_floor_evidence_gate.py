#!/usr/bin/env python3
"""Bind the measured physical floor to exact CODATA 2018 source rows."""

from __future__ import annotations

import argparse
import decimal
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
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
WATCHDOG_PATH = ROOT / "scripts" / "codata_floor_evidence_watchdog.py"

PAIR_SCHEMA = "civsim.units.codata-2018-floor-evidence-pair.v2"
FACTS_SCHEMA = "civsim.units.codata-2018-floor-facts.v1"
CLAIM_ID = "floor.codata-factual-row-binding"
PRODUCER_IMPLEMENTATION = "civsim.units.codata-fixed-column-producer.v2"
WATCHDOG_IMPLEMENTATION = "civsim.units.codata-record-scan-watchdog.v2"
PRODUCER_CANARY_SCHEMA = "civsim.units.codata-fixed-column-canary-execution.v2"
PRODUCER_CANARY_MUTATION_SCHEMA = (
    "civsim.units.codata-fixed-column-canary-mutation.v1"
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

CANARIES = (
    "facts-bare-carriage-return",
    "facts-schema-mutation",
    "facts-source-id-mutation",
    "facts-source-digest-mutation",
    "facts-row-omission",
    "facts-row-duplication",
    "facts-row-reordering",
    "facts-column-mutation",
    "facts-row-digest-format-mutation",
    "facts-dimension-format-mutation",
    "fixed-column-short-record",
    "fixed-column-decimal-mutation",
    "fixed-column-unit-mutation",
)


class FloorEvidenceError(ValueError):
    """The producer or pair receipt failed closed."""

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
    without_pairs = raw.replace(b"\r\n", b"")
    if b"\r" in without_pairs:
        raise FloorEvidenceError(
            f"{label} contains a bare carriage return",
            code="text.bare-carriage-return",
        )
    return raw.replace(b"\r\n", b"\n")


def _facts_rows(raw: bytes) -> tuple[list[dict[str, str]], bytes]:
    canonical = _canonical_repo_text(raw, "facts")
    try:
        lines = canonical.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        raise FloorEvidenceError("facts are not ASCII") from error
    if len(lines) != 9:
        relation = "short" if len(lines) < 9 else "long"
        raise FloorEvidenceError(
            "facts must contain six headers and three rows",
            code=f"facts.line-count.{relation}",
        )
    required_headers = {
        "schema": FACTS_SCHEMA,
        "source_id": SOURCE_ID,
        "source_sha256": SOURCE_SHA256,
        "source_bytes": str(SOURCE_BYTES),
        "source_line_endings": "LF",
    }
    for line in lines[:5]:
        fields = line.split("\t")
        if len(fields) != 2:
            raise FloorEvidenceError(
                "facts header differs from the source contract",
                code="facts.header.shape",
            )
        key, value = fields
        if key not in required_headers:
            raise FloorEvidenceError(
                "facts header differs from the source contract",
                code="facts.header.key",
            )
        expected = required_headers.pop(key)
        if value != expected:
            raise FloorEvidenceError(
                "facts header differs from the source contract",
                code=f"facts.header.{key}",
            )
    if required_headers:
        raise FloorEvidenceError(
            "facts omit source metadata",
            code="facts.header.omission",
        )
    columns = lines[5].split("\t")
    if columns != ["columns", *EXPECTED_COLUMNS]:
        raise FloorEvidenceError(
            "facts columns differ from the closed schema",
            code="facts.columns",
        )

    rows: list[dict[str, str]] = []
    symbols: set[str] = set()
    for line in lines[6:]:
        fields = line.split("\t")
        if len(fields) != len(EXPECTED_COLUMNS) + 1 or fields[0] != "row":
            raise FloorEvidenceError(
                "facts row has the wrong arity",
                code="facts.row.shape",
            )
        row = dict(zip(EXPECTED_COLUMNS, fields[1:]))
        symbol = row["symbol"]
        if symbol in symbols:
            raise FloorEvidenceError(f"facts duplicate symbol {symbol!r}")
        symbols.add(symbol)
        for field in ("source_line", "byte_offset", "byte_length"):
            if not row[field].isdigit():
                raise FloorEvidenceError(f"facts {field} is not an unsigned integer")
        if re.fullmatch(r"[0-9a-f]{64}", row["row_sha256"]) is None:
            raise FloorEvidenceError(
                "facts row digest is not lowercase SHA-256",
                code="facts.row-digest-format",
            )
        if re.fullmatch(r"-?[0-9]+(?:,-?[0-9]+){6}", row["si_dimension"]) is None:
            raise FloorEvidenceError(
                "facts SI dimension is not a seven-axis vector",
                code="facts.dimension-format",
            )
        rows.append(row)
    if [row["symbol"] for row in rows] != ["alpha", "G", "m_e"]:
        raise FloorEvidenceError(
            "facts floor order differs from the reviewed identity order",
            code="facts.row-order",
        )
    return rows, canonical


def _ungroup_decimal(raw: str) -> str:
    compact = raw.replace(" ", "").replace("E", "e")
    if re.fullmatch(r"[0-9]+(?:\.[0-9]+)?(?:e[+-][0-9]+)?", compact) is None:
        raise FloorEvidenceError(
            f"invalid CODATA decimal {raw!r}",
            code="source.decimal-format",
        )
    return compact


def _absolute_scientific(raw: str) -> str:
    compact = _ungroup_decimal(raw)
    value = decimal.Decimal(compact)
    if value.is_zero():
        return "0"
    sign, digits_tuple, exponent = value.normalize().as_tuple()
    if sign:
        raise FloorEvidenceError("CODATA uncertainty must be nonnegative")
    digits = "".join(str(digit) for digit in digits_tuple)
    scientific_exponent = exponent + len(digits) - 1
    mantissa = digits[0]
    if len(digits) > 1:
        mantissa += "." + digits[1:]
    return f"{mantissa}e{scientific_exponent:+d}".replace("e+", "e")


def _dimension_from_unit(unit: str) -> str:
    dimensions = {
        "": "0,0,0,0,0,0,0",
        "kg": "0,1,0,0,0,0,0",
        "m^3 kg^-1 s^-2": "3,-1,-2,0,0,0,0",
    }
    try:
        return dimensions[unit]
    except KeyError as error:
        raise FloorEvidenceError(
            f"unreviewed source unit {unit!r}",
            code="source.unit",
        ) from error


def _parse_fixed_line(line: bytes) -> dict[str, str]:
    if len(line) < 110:
        raise FloorEvidenceError(
            "CODATA record is shorter than the fixed columns",
            code="source.fixed-record-short",
        )
    try:
        name = line[:60].decode("ascii").rstrip()
        value = line[60:85].decode("ascii").strip()
        uncertainty = line[85:110].decode("ascii").strip()
        unit = line[110:].decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise FloorEvidenceError("CODATA record is not ASCII") from error
    if not name or not value or not uncertainty:
        raise FloorEvidenceError("CODATA fixed-width record has an empty required field")
    return {
        "source_anchor": name,
        "normalized_value": _ungroup_decimal(value),
        "absolute_uncertainty": _absolute_scientific(uncertainty),
        "source_unit": unit or "1",
        "si_dimension": _dimension_from_unit(unit),
    }


def inspect_source(raw: bytes, expected_rows: list[dict[str, str]]) -> list[dict[str, str]]:
    if len(raw) != SOURCE_BYTES:
        raise FloorEvidenceError(
            f"source has {len(raw)} bytes, expected {SOURCE_BYTES}"
        )
    if _sha256(raw) != SOURCE_SHA256:
        raise FloorEvidenceError("source SHA-256 differs from the NIST pin")
    if b"\r" in raw:
        raise FloorEvidenceError("source violates the LF-only byte contract")
    try:
        raw.decode("ascii")
    except UnicodeDecodeError as error:
        raise FloorEvidenceError("source is not ASCII") from error

    expected_by_anchor = {row["source_anchor"]: row for row in expected_rows}
    found: dict[str, dict[str, str]] = {}
    offset = 0
    for line_number, with_ending in enumerate(raw.splitlines(keepends=True), 1):
        if not with_ending.endswith(b"\n"):
            raise FloorEvidenceError("source final record lacks LF")
        line = with_ending[:-1]
        try:
            possible_anchor = line[:60].decode("ascii").rstrip()
        except UnicodeDecodeError as error:
            raise FloorEvidenceError("CODATA record is not ASCII") from error
        parsed = (
            _parse_fixed_line(line)
            if possible_anchor in expected_by_anchor
            else None
        )
        if parsed is not None:
            anchor = parsed["source_anchor"]
            if anchor in found:
                raise FloorEvidenceError(f"source duplicates row {anchor!r}")
            expected = expected_by_anchor[anchor]
            record = {
                "symbol": expected["symbol"],
                "source_line": str(line_number),
                "byte_offset": str(offset),
                "byte_length": str(len(line)),
                "row_sha256": _sha256(line),
                **parsed,
            }
            if record != expected:
                raise FloorEvidenceError(
                    f"source row {anchor!r} differs from the checked facts"
                )
            found[anchor] = record
        offset += len(with_ending)
    if offset != len(raw):
        raise FloorEvidenceError("source fixed-column scan did not consume all bytes")
    missing = [anchor for anchor in expected_by_anchor if anchor not in found]
    if missing:
        raise FloorEvidenceError("source omits required rows: " + ", ".join(missing))
    return [found[row["source_anchor"]] for row in expected_rows]


def evidence_document(rows: list[dict[str, str]]) -> dict[str, Any]:
    return {
        "source_id": SOURCE_ID,
        "source_sha256": SOURCE_SHA256,
        "source_bytes": SOURCE_BYTES,
        "source_text_contract": TEXT_CONTRACT,
        "rows": rows,
    }


def producer_result_digest(evidence_sha256: str) -> str:
    return _sha256(
        _canonical_json(
            {
                "implementation": PRODUCER_IMPLEMENTATION,
                "claim_id": CLAIM_ID,
                "evidence_sha256": evidence_sha256,
                "verdict": "admitted",
            }
        )
    )


def _producer_probe_line(
    value: bytes = b"7.297 352 5693 e-3",
    uncertainty: bytes = b"0.000 000 0011 e-3",
    unit: bytes = b"",
) -> bytes:
    return (
        b"fine-structure constant".ljust(60)
        + value.ljust(25)
        + uncertainty.ljust(25)
        + unit
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


def _mutation_sha256(canary_id: str, input_kind: str, candidate: bytes) -> str:
    return _sha256(
        _canonical_json(
            {
                "schema": PRODUCER_CANARY_MUTATION_SCHEMA,
                "canary_id": canary_id,
                "input_kind": input_kind,
                "candidate_bytes": len(candidate),
                "candidate_sha256": _sha256(candidate),
            }
        )
    )


def _observe_refusal(
    canary_id: str,
    input_kind: str,
    candidate: bytes,
    expected_refusal_code: str,
    operation: Any,
) -> dict[str, str]:
    try:
        operation()
    except FloorEvidenceError as error:
        if error.code != expected_refusal_code:
            raise AssertionError(
                f"canary {canary_id} reached refusal code {error.code!r}, "
                f"expected {expected_refusal_code!r}"
            ) from error
        return {
            "id": canary_id,
            "mutation_sha256": _mutation_sha256(
                canary_id, input_kind, candidate
            ),
            "outcome": "refused",
            "refusal_code": error.code,
            "error_type": type(error).__name__,
            "error_sha256": _sha256(str(error).encode("utf-8")),
        }
    raise AssertionError(f"canary was admitted: {canary_id}")


def producer_canary_observations() -> list[dict[str, str]]:
    facts = FACTS_PATH.read_bytes()
    lines = facts.splitlines()
    if len(lines) != 9:
        raise FloorEvidenceError("checked facts cannot seed the canary suite")

    reordered = lines[:]
    reordered[7], reordered[8] = reordered[8], reordered[7]
    fixed_line = _producer_probe_line()
    cases = (
        (
            CANARIES[0],
            "facts",
            _changed_payload(CANARIES[0], facts, facts + b"\r"),
            "text.bare-carriage-return",
            _facts_rows,
        ),
        (
            CANARIES[1],
            "facts",
            _replace_once(
                CANARIES[1],
                facts,
                FACTS_SCHEMA.encode("ascii"),
                b"changed-schema",
            ),
            "facts.header.schema",
            _facts_rows,
        ),
        (
            CANARIES[2],
            "facts",
            _replace_once(
                CANARIES[2],
                facts,
                SOURCE_ID.encode("ascii"),
                b"changed-source",
            ),
            "facts.header.source_id",
            _facts_rows,
        ),
        (
            CANARIES[3],
            "facts",
            _replace_once(
                CANARIES[3],
                facts,
                SOURCE_SHA256.encode("ascii"),
                b"0" * 64,
            ),
            "facts.header.source_sha256",
            _facts_rows,
        ),
        (
            CANARIES[4],
            "facts",
            _changed_payload(
                CANARIES[4],
                facts,
                b"\n".join(lines[:-1]) + b"\n",
            ),
            "facts.line-count.short",
            _facts_rows,
        ),
        (
            CANARIES[5],
            "facts",
            _changed_payload(
                CANARIES[5],
                facts,
                facts + lines[-1] + b"\n",
            ),
            "facts.line-count.long",
            _facts_rows,
        ),
        (
            CANARIES[6],
            "facts",
            _changed_payload(
                CANARIES[6],
                facts,
                b"\n".join(reordered) + b"\n",
            ),
            "facts.row-order",
            _facts_rows,
        ),
        (
            CANARIES[7],
            "facts",
            _replace_once(
                CANARIES[7],
                facts,
                b"\tbyte_offset\t",
                b"\toffset\t",
            ),
            "facts.columns",
            _facts_rows,
        ),
        (
            CANARIES[8],
            "facts",
            _replace_once(
                CANARIES[8],
                facts,
                b"e8a9302a81c562198cc75c33f9fc8824685fdf1aafb1e25416417257dee6882f",
                b"not-a-sha256",
            ),
            "facts.row-digest-format",
            _facts_rows,
        ),
        (
            CANARIES[9],
            "facts",
            _replace_once(
                CANARIES[9],
                facts,
                b"\t0,0,0,0,0,0,0\n",
                b"\tinvalid-dimension\n",
            ),
            "facts.dimension-format",
            _facts_rows,
        ),
        (
            CANARIES[10],
            "fixed-column-record",
            _changed_payload(CANARIES[10], fixed_line, b"short"),
            "source.fixed-record-short",
            _parse_fixed_line,
        ),
        (
            CANARIES[11],
            "fixed-column-record",
            _changed_payload(
                CANARIES[11],
                fixed_line,
                _producer_probe_line(value=b"not-a-decimal"),
            ),
            "source.decimal-format",
            _parse_fixed_line,
        ),
        (
            CANARIES[12],
            "fixed-column-record",
            _changed_payload(
                CANARIES[12],
                fixed_line,
                _producer_probe_line(unit=b"unreviewed-unit"),
            ),
            "source.unit",
            _parse_fixed_line,
        ),
    )
    observations = [
        _observe_refusal(
            canary_id,
            input_kind,
            candidate,
            expected_refusal_code,
            lambda detector=detector, candidate=candidate: detector(candidate),
        )
        for (
            canary_id,
            input_kind,
            candidate,
            expected_refusal_code,
            detector,
        ) in cases
    ]
    if tuple(observation["id"] for observation in observations) != CANARIES:
        raise AssertionError("producer canary execution order differs from its declaration")
    return observations


def producer_canary_digest() -> str:
    observations = producer_canary_observations()
    return _sha256(
        _canonical_json(
            {
                "schema": PRODUCER_CANARY_SCHEMA,
                "implementation": PRODUCER_IMPLEMENTATION,
                "observations": observations,
            }
        )
    )


def _watchdog_result(source_path: pathlib.Path) -> dict[str, str]:
    completed = subprocess.run(
        [
            sys.executable,
            str(WATCHDOG_PATH),
            "--audit-source",
            str(source_path),
            "--result",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    try:
        result = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise FloorEvidenceError("watchdog did not emit a result receipt") from error
    if not isinstance(result, dict):
        raise FloorEvidenceError("watchdog result is not an object")
    expected_fields = {
        "implementation",
        "evidence_sha256",
        "result_sha256",
        "canary_schema",
        "canary_count",
        "canary_sha256",
    }
    if set(result) != expected_fields:
        raise FloorEvidenceError("watchdog result fields differ from the pair schema")
    return {key: str(value) for key, value in result.items()}


def _pair_digest(receipt: dict[str, Any]) -> str:
    unsigned = dict(receipt)
    unsigned.pop("pair_sha256", None)
    return _sha256(_canonical_json(unsigned))


def build_receipt(
    live_raw: bytes,
    archive_raw: bytes,
    audited_at_utc: str,
    live_path: pathlib.Path,
    archive_path: pathlib.Path,
) -> bytes:
    expected_rows, canonical_facts = _facts_rows(FACTS_PATH.read_bytes())
    live_rows = inspect_source(live_raw, expected_rows)
    archive_rows = inspect_source(archive_raw, expected_rows)
    if live_raw != archive_raw or live_rows != archive_rows:
        raise FloorEvidenceError("live and archived source bytes do not agree")
    evidence_sha256 = _sha256(_canonical_json(evidence_document(live_rows)))
    live_watchdog = _watchdog_result(live_path)
    archive_watchdog = _watchdog_result(archive_path)
    if live_watchdog != archive_watchdog:
        raise FloorEvidenceError("watchdog disagrees across live and archived bytes")
    if (
        live_watchdog["implementation"] != WATCHDOG_IMPLEMENTATION
        or live_watchdog["evidence_sha256"] != evidence_sha256
    ):
        raise FloorEvidenceError("producer and watchdog disagree on source evidence")

    receipt: dict[str, Any] = {
        "schema": PAIR_SCHEMA,
        "claim_id": CLAIM_ID,
        "source": {
            "id": SOURCE_ID,
            "sha256": SOURCE_SHA256,
            "bytes": SOURCE_BYTES,
            "text_contract": TEXT_CONTRACT,
        },
        "facts": {
            "path": FACTS_RELATIVE,
            "schema": FACTS_SCHEMA,
            "sha256": _sha256(canonical_facts),
            "row_count": len(expected_rows),
        },
        "evidence_sha256": evidence_sha256,
        "producer": {
            "implementation": PRODUCER_IMPLEMENTATION,
            "result_sha256": producer_result_digest(evidence_sha256),
            "canary_schema": PRODUCER_CANARY_SCHEMA,
            "canary_count": len(CANARIES),
            "canary_sha256": producer_canary_digest(),
        },
        "watchdog": {
            "implementation": WATCHDOG_IMPLEMENTATION,
            "result_sha256": live_watchdog["result_sha256"],
            "canary_schema": live_watchdog["canary_schema"],
            "canary_count": int(live_watchdog["canary_count"]),
            "canary_sha256": live_watchdog["canary_sha256"],
        },
        "online_audit": {
            "live_url": LIVE_URL,
            "archive_url": ARCHIVE_URL,
            "audited_at_utc": audited_at_utc,
            "live_archive_equal": True,
        },
    }
    receipt["pair_sha256"] = _pair_digest(receipt)
    return json.dumps(receipt, indent=2, sort_keys=True).encode("ascii") + b"\n"


def _required_dict(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise FloorEvidenceError(f"{label} must be an object")
    return value


def validate_checked_receipt(receipt_raw: bytes, facts_raw: bytes) -> dict[str, Any]:
    rows, canonical_facts = _facts_rows(facts_raw)
    canonical_receipt = _canonical_repo_text(receipt_raw, "receipt")
    try:
        receipt = json.loads(canonical_receipt)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise FloorEvidenceError("receipt is not JSON text") from error
    receipt = _required_dict(receipt, "receipt")
    evidence_sha256 = _sha256(_canonical_json(evidence_document(rows)))

    if receipt.get("schema") != PAIR_SCHEMA or receipt.get("claim_id") != CLAIM_ID:
        raise FloorEvidenceError("receipt identity differs from the pair contract")
    if receipt.get("source") != {
        "id": SOURCE_ID,
        "sha256": SOURCE_SHA256,
        "bytes": SOURCE_BYTES,
        "text_contract": TEXT_CONTRACT,
    }:
        raise FloorEvidenceError("receipt source metadata differs from its pin")
    if receipt.get("facts") != {
        "path": FACTS_RELATIVE,
        "schema": FACTS_SCHEMA,
        "sha256": _sha256(canonical_facts),
        "row_count": len(rows),
    }:
        raise FloorEvidenceError("receipt facts binding is stale")
    if receipt.get("evidence_sha256") != evidence_sha256:
        raise FloorEvidenceError("receipt evidence digest is stale")

    producer = _required_dict(receipt.get("producer"), "receipt.producer")
    if producer != {
        "implementation": PRODUCER_IMPLEMENTATION,
        "result_sha256": producer_result_digest(evidence_sha256),
        "canary_schema": PRODUCER_CANARY_SCHEMA,
        "canary_count": len(CANARIES),
        "canary_sha256": producer_canary_digest(),
    }:
        raise FloorEvidenceError("receipt producer result differs from live code")
    watchdog = _required_dict(receipt.get("watchdog"), "receipt.watchdog")
    if watchdog.get("implementation") != WATCHDOG_IMPLEMENTATION:
        raise FloorEvidenceError("receipt watchdog identity differs from its pin")
    if watchdog.get("canary_schema") != (
        "civsim.units.codata-record-scan-canary-execution.v2"
    ) or watchdog.get("canary_count") != 14:
        raise FloorEvidenceError("receipt watchdog canary profile differs from its pin")
    for field in ("result_sha256", "canary_sha256"):
        if re.fullmatch(r"[0-9a-f]{64}", str(watchdog.get(field, ""))) is None:
            raise FloorEvidenceError(f"receipt watchdog {field} is not SHA-256")

    audit = _required_dict(receipt.get("online_audit"), "receipt.online_audit")
    if audit.get("live_url") != LIVE_URL or audit.get("archive_url") != ARCHIVE_URL:
        raise FloorEvidenceError("receipt endpoint identity differs from its pin")
    if audit.get("live_archive_equal") is not True:
        raise FloorEvidenceError("receipt does not attest live/archive equality")
    if re.fullmatch(
        r"20[0-9]{2}-[01][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-5][0-9]Z",
        str(audit.get("audited_at_utc", "")),
    ) is None:
        raise FloorEvidenceError("receipt audit time is not canonical UTC")
    if receipt.get("pair_sha256") != _pair_digest(receipt):
        raise FloorEvidenceError("receipt pair digest is stale")
    return receipt


def _run_watchdog_offline() -> None:
    subprocess.run(
        [sys.executable, str(WATCHDOG_PATH)],
        check=True,
        capture_output=True,
        text=True,
    )


def _fetch(url: str) -> bytes:
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "civsim-floor-evidence-audit/1"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def _source_bytes(path: pathlib.Path | None, url: str) -> bytes:
    return path.read_bytes() if path is not None else _fetch(url)


def _online_receipt(
    live_raw: bytes,
    archive_raw: bytes,
    audited_at_utc: str,
) -> bytes:
    with tempfile.TemporaryDirectory(prefix="civsim-codata-floor-") as directory:
        root = pathlib.Path(directory)
        live_path = root / "live.txt"
        archive_path = root / "archive.txt"
        live_path.write_bytes(live_raw)
        archive_path.write_bytes(archive_raw)
        return build_receipt(
            live_raw,
            archive_raw,
            audited_at_utc,
            live_path,
            archive_path,
        )


def _must_refuse(label: str, operation: Any) -> None:
    try:
        operation()
    except (FloorEvidenceError, json.JSONDecodeError, UnicodeDecodeError):
        return
    raise AssertionError(f"canary was admitted: {label}")


def self_test() -> None:
    facts = FACTS_PATH.read_bytes()
    receipt = RECEIPT_PATH.read_bytes()
    validate_checked_receipt(receipt, facts)
    _run_watchdog_offline()
    if len(producer_canary_observations()) != len(CANARIES):
        raise AssertionError("producer did not execute every declared canary")

    line = _producer_probe_line()
    parsed = _parse_fixed_line(line)
    if parsed["normalized_value"] != "7.2973525693e-3":
        raise AssertionError("fixed-column producer did not parse the known value")
    changed = bytearray(line)
    changed[line.index(b"5693")] ^= 1
    if _parse_fixed_line(bytes(changed)) == parsed:
        raise AssertionError("fixed-column value mutation was not observed")

    changed_facts = facts.replace(b"\t6.67430e-11\t", b"\t6.67431e-11\t", 1)
    _must_refuse(
        "facts value mutation",
        lambda: validate_checked_receipt(receipt, changed_facts),
    )
    changed_receipt = receipt.replace(
        LIVE_URL.encode("ascii"),
        (LIVE_URL + "?changed").encode("ascii"),
        1,
    )
    _must_refuse(
        "endpoint mutation",
        lambda: validate_checked_receipt(changed_receipt, facts),
    )
    bare_cr = facts + b"\r"
    _must_refuse(
        "bare carriage return",
        lambda: validate_checked_receipt(receipt, bare_cr),
    )
    subprocess.run(
        [sys.executable, str(WATCHDOG_PATH), "--self-test"],
        check=True,
        capture_output=True,
        text=True,
    )
    print("CODATA floor evidence producer self-test: PASS")


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--audit-online", action="store_true")
    parser.add_argument("--live-file", type=pathlib.Path)
    parser.add_argument("--archive-file", type=pathlib.Path)
    parser.add_argument("--receipt", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test and (
        args.audit_online
        or args.live_file is not None
        or args.archive_file is not None
        or args.receipt
    ):
        parser.error("--self-test is exclusive")
    if (args.live_file is None) != (args.archive_file is None):
        parser.error("--live-file and --archive-file must be supplied together")
    if args.receipt and not args.audit_online:
        parser.error("--receipt requires --audit-online")
    try:
        if args.self_test:
            self_test()
            return 0
        checked = RECEIPT_PATH.read_bytes()
        checked_document = validate_checked_receipt(checked, FACTS_PATH.read_bytes())
        _run_watchdog_offline()
        if not args.audit_online:
            print("CODATA floor evidence producer and watchdog: PASS")
            return 0

        live_raw = _source_bytes(args.live_file, LIVE_URL)
        archive_raw = _source_bytes(args.archive_file, ARCHIVE_URL)
        audited_at_utc = str(
            _required_dict(
                checked_document.get("online_audit"), "receipt.online_audit"
            )["audited_at_utc"]
        )
        observed = _online_receipt(live_raw, archive_raw, audited_at_utc)
        if json.loads(observed) != checked_document:
            raise FloorEvidenceError(
                "checked receipt differs from the current live/archive audit"
            )
        if args.receipt:
            sys.stdout.buffer.write(observed)
        else:
            print("CODATA floor evidence live/archive audit: PASS")
    except (
        AssertionError,
        FloorEvidenceError,
        OSError,
        subprocess.SubprocessError,
        urllib.error.URLError,
    ) as error:
        print(f"CODATA floor evidence producer and watchdog: FAIL: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
