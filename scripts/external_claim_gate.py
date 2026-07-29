#!/usr/bin/env python3
"""Produce and cross-check release receipts for external adverse claims."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import json
import pathlib
import subprocess
import sys
import tomllib
from typing import Any, Callable


ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "sources" / "external_claims.toml"
WATCHDOG = ROOT / "scripts" / "external_claim_watchdog.py"
SHA256 = set("0123456789abcdef")
LINEAGE_KEYS = {
    "anchors",
    "apparatus",
    "authors",
    "custody_sha256",
    "datasets",
    "id",
    "independence_basis",
    "methods",
    "source_ids",
    "upstream_roots",
}
PENDING_KEYS = {"id", "private_dossier_sha256", "state", "subject"}
RELEASE_KEYS = PENDING_KEYS | {
    "action",
    "approval_payload_sha256",
    "approval_revoked",
    "approver_identity",
    "destination_sha256",
    "expires_utc",
    "lineage",
    "lineage_set_sha256",
    "signature_path",
    "subject_roots",
    "text_sha256",
}
ROOT_PREFIXES = (
    "source:",
    "author:",
    "dataset:",
    "apparatus:",
    "method:",
    "upstream:",
    "custody:",
)
FORBIDDEN_PUBLIC_FRAGMENTS = (
    "authors are wrong",
    "column is wrong",
    "defect in the source",
    "erratum is owed",
    "invalid dataset",
    "paper is wrong",
    "printed slip",
    "source-internal typo",
    "typesetting error",
    "reference is a misprint",
    "incomplete or wrong",
    "wrong against the primary",
)


class ExternalClaimError(ValueError):
    pass


def _text(row: dict[str, Any], key: str, label: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ExternalClaimError(f"{label}.{key} must be nonempty text")
    return value


def _sha(row: dict[str, Any], key: str, label: str) -> str:
    value = _text(row, key, label)
    if len(value) != 64 or any(character not in SHA256 for character in value):
        raise ExternalClaimError(f"{label}.{key} must be lowercase SHA-256")
    return value


def _texts(row: dict[str, Any], key: str, label: str, minimum: int = 1) -> list[str]:
    value = row.get(key)
    if not isinstance(value, list) or any(
        not isinstance(item, str) or not item.strip() for item in value
    ):
        raise ExternalClaimError(f"{label}.{key} must be a text array")
    if len(value) < minimum:
        raise ExternalClaimError(f"{label}.{key} needs at least {minimum} item(s)")
    if value != sorted(set(value)):
        raise ExternalClaimError(f"{label}.{key} must be sorted and unique")
    return value


def _repo_file(value: str, label: str) -> pathlib.Path:
    relative = pathlib.PurePosixPath(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise ExternalClaimError(f"{label} must stay inside the repository")
    path = ROOT.joinpath(*relative.parts)
    if not path.is_file():
        raise ExternalClaimError(f"{label} does not name a file: {value}")
    return path


def _canonical_json(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _public_surface_digest() -> str:
    files: list[pathlib.Path] = []
    for directory in (ROOT / "docs" / "working", ROOT / "sources", ROOT / "crates" / "physics" / "data"):
        files.extend(
            path
            for path in directory.rglob("*")
            if path.is_file() and path.suffix.lower() in {".md", ".toml", ".txt"}
        )
    excluded = {ROOT / "docs" / "working" / "EXTERNAL_ADVERSE_CLAIM_RULE.md"}
    digest = hashlib.sha256()
    for path in sorted(set(files) - excluded):
        raw = path.read_bytes()
        text = raw.decode("utf-8").lower()
        relative = path.relative_to(ROOT).as_posix()
        for fragment in FORBIDDEN_PUBLIC_FRAGMENTS:
            if fragment in text:
                raise ExternalClaimError(
                    f"unreleased adverse phrase '{fragment}' appears in {relative}"
                )
        encoded_path = relative.encode("utf-8")
        digest.update(len(encoded_path).to_bytes(8, "little"))
        digest.update(encoded_path)
        digest.update(len(raw).to_bytes(8, "little"))
        digest.update(raw)
    return digest.hexdigest()


def _lineage_record(row: dict[str, Any], label: str) -> dict[str, Any]:
    if set(row) != LINEAGE_KEYS:
        raise ExternalClaimError(f"{label} fields differ from the closed lineage schema")
    record = {
        "id": _text(row, "id", label),
        "source_ids": _texts(row, "source_ids", label),
        "anchors": _texts(row, "anchors", label),
        "authors": _texts(row, "authors", label),
        "datasets": _texts(row, "datasets", label),
        "apparatus": _texts(row, "apparatus", label),
        "methods": _texts(row, "methods", label),
        "upstream_roots": _texts(row, "upstream_roots", label),
        "custody_sha256": _texts(row, "custody_sha256", label),
        "independence_basis": _text(row, "independence_basis", label),
    }
    for digest in record["custody_sha256"]:
        if len(digest) != 64 or any(character not in SHA256 for character in digest):
            raise ExternalClaimError(f"{label}.custody_sha256 contains a non-SHA-256 value")
    return record


def _roots(lineage: dict[str, Any]) -> set[str]:
    roots: set[str] = set()
    for value in lineage["source_ids"]:
        roots.add(f"source:{value}")
    for key, prefix in [
        ("authors", "author"),
        ("datasets", "dataset"),
        ("apparatus", "apparatus"),
        ("methods", "method"),
        ("upstream_roots", "upstream"),
    ]:
        for value in lineage[key]:
            roots.add(f"{prefix}:{value}")
    for value in lineage["custody_sha256"]:
        roots.add(f"custody:{value}")
    return roots


def _independent_components(lineages: list[dict[str, Any]], subject_roots: set[str]) -> int:
    parent = list(range(len(lineages)))

    def find(index: int) -> int:
        while parent[index] != index:
            parent[index] = parent[parent[index]]
            index = parent[index]
        return index

    def union(left: int, right: int) -> None:
        left_root = find(left)
        right_root = find(right)
        if left_root != right_root:
            parent[right_root] = left_root

    by_root: dict[str, int] = {}
    lineage_roots = [_roots(lineage) for lineage in lineages]
    for index, roots in enumerate(lineage_roots):
        for root in roots:
            prior = by_root.setdefault(root, index)
            union(index, prior)
    excluded = {
        find(index)
        for index, roots in enumerate(lineage_roots)
        if roots & subject_roots
    }
    return len({find(index) for index in range(len(lineages))} - excluded)


def _approval_payload(candidate: dict[str, Any], namespace: str) -> bytes:
    return _canonical_json(
        {
            "schema": "civsim.external-adverse-claim-approval.v1",
            "candidate_id": candidate["id"],
            "subject": candidate["subject"],
            "subject_roots": candidate["subject_roots"],
            "action": candidate["action"],
            "text_sha256": candidate["text_sha256"],
            "destination_sha256": candidate["destination_sha256"],
            "private_dossier_sha256": candidate["private_dossier_sha256"],
            "lineage_set_sha256": candidate["lineage_set_sha256"],
            "approver_identity": candidate["approver_identity"],
            "approval_revoked": candidate["approval_revoked"],
            "expires_utc": candidate["expires_utc"],
            "signature_namespace": namespace,
        }
    )


def _revoked_payloads(path: pathlib.Path) -> tuple[set[str], str]:
    raw = path.read_bytes()
    data = tomllib.loads(raw.decode("utf-8"))
    if data.get("schema") != 1 or set(data) != {
        "schema",
        "revoked_approval_payload_sha256",
    }:
        raise ExternalClaimError("external claim revocation registry has an invalid schema")
    values = data["revoked_approval_payload_sha256"]
    if not isinstance(values, list) or values != sorted(set(values)):
        raise ExternalClaimError("external claim revocations must be sorted and unique")
    for value in values:
        if (
            not isinstance(value, str)
            or len(value) != 64
            or any(character not in SHA256 for character in value)
        ):
            raise ExternalClaimError("external claim revocation is not lowercase SHA-256")
    return set(values), hashlib.sha256(raw).hexdigest()


def _verify_signature(
    approvers: pathlib.Path,
    namespace: str,
    identity: str,
    signature: pathlib.Path,
    payload: bytes,
) -> bool:
    completed = subprocess.run(
        [
            "ssh-keygen",
            "-Y",
            "verify",
            "-f",
            str(approvers),
            "-I",
            identity,
            "-n",
            namespace,
            "-s",
            str(signature),
        ],
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    return completed.returncode == 0


def validate(
    data: dict[str, Any],
    *,
    now: dt.datetime | None = None,
    signature_verifier: Callable[[pathlib.Path, str, str, pathlib.Path, bytes], bool] = _verify_signature,
    revoked_payloads: set[str] | None = None,
) -> tuple[list[str], list[str]]:
    inventory = data.get("inventory")
    if not isinstance(inventory, dict) or inventory.get("schema") != 1:
        raise ExternalClaimError("inventory.schema must equal 1")
    minimum = inventory.get("minimum_independent_lineages")
    if minimum != 5:
        raise ExternalClaimError("minimum_independent_lineages must equal 5")
    _repo_file(_text(inventory, "rule", "inventory"), "inventory.rule")
    approvers = _repo_file(
        _text(inventory, "approvers_file", "inventory"), "inventory.approvers_file"
    )
    namespace = _text(inventory, "signature_namespace", "inventory")
    revocations_path = _repo_file(
        _text(inventory, "revocations_file", "inventory"),
        "inventory.revocations_file",
    )
    revoked = (
        _revoked_payloads(revocations_path)[0]
        if revoked_payloads is None
        else revoked_payloads
    )
    rows = data.get("candidate", [])
    if not isinstance(rows, list):
        raise ExternalClaimError("candidate must be an array of tables")
    candidate_ids: list[str] = []
    release_ids: list[str] = []
    current = now or dt.datetime.now(dt.timezone.utc)
    for index, row in enumerate(rows):
        label = f"candidate[{index}]"
        if not isinstance(row, dict):
            raise ExternalClaimError(f"{label} must be a table")
        candidate_id = _text(row, "id", label)
        candidate_ids.append(candidate_id)
        _text(row, "subject", label)
        state = _text(row, "state", label)
        if state == "internal_review":
            if not set(row) <= PENDING_KEYS:
                raise ExternalClaimError(f"{label} pending row contains release fields")
            if "private_dossier_sha256" in row:
                _sha(row, "private_dossier_sha256", label)
            continue
        if state != "release":
            raise ExternalClaimError(f"{label}.state must be internal_review or release")
        if set(row) != RELEASE_KEYS:
            raise ExternalClaimError(f"{label} fields differ from the closed release schema")
        release_ids.append(candidate_id)
        if _text(row, "action", label) not in {"contact", "publication"}:
            raise ExternalClaimError(f"{label}.action must be contact or publication")
        for key in [
            "private_dossier_sha256",
            "text_sha256",
            "destination_sha256",
            "lineage_set_sha256",
            "approval_payload_sha256",
        ]:
            _sha(row, key, label)
        if row.get("approval_revoked") is not False:
            raise ExternalClaimError(f"{label} approval is revoked or malformed")
        subject_roots = set(_texts(row, "subject_roots", label))
        if any(not root.startswith(ROOT_PREFIXES) for root in subject_roots):
            raise ExternalClaimError(f"{label}.subject_roots contains an untyped root")
        raw_lineages = row.get("lineage")
        if not isinstance(raw_lineages, list):
            raise ExternalClaimError(f"{label}.lineage must be an array of tables")
        lineages = [
            _lineage_record(lineage, f"{label}.lineage[{lineage_index}]")
            for lineage_index, lineage in enumerate(raw_lineages)
        ]
        lineage_ids = [lineage["id"] for lineage in lineages]
        if lineage_ids != sorted(set(lineage_ids)):
            raise ExternalClaimError(f"{label} lineage ids must be sorted and unique")
        lineage_digest = hashlib.sha256(_canonical_json(lineages)).hexdigest()
        if lineage_digest != row["lineage_set_sha256"]:
            raise ExternalClaimError(f"{label} lineage-set digest differs")
        if _independent_components(lineages, subject_roots) < minimum:
            raise ExternalClaimError(f"{label} has fewer than five independent lineages")
        payload = _approval_payload(row, namespace)
        if hashlib.sha256(payload).hexdigest() != row["approval_payload_sha256"]:
            raise ExternalClaimError(f"{label} approval payload digest differs")
        if row["approval_payload_sha256"] in revoked:
            raise ExternalClaimError(f"{label} approval payload is revoked")
        try:
            expiry = dt.datetime.fromisoformat(_text(row, "expires_utc", label))
        except ValueError as error:
            raise ExternalClaimError(f"{label}.expires_utc is not ISO-8601") from error
        if expiry.tzinfo is None or expiry <= current:
            raise ExternalClaimError(f"{label} approval is expired or timezone-free")
        signature = _repo_file(_text(row, "signature_path", label), f"{label}.signature_path")
        identity = _text(row, "approver_identity", label)
        if not signature_verifier(approvers, namespace, identity, signature, payload):
            raise ExternalClaimError(f"{label} human signature did not verify")
    if candidate_ids != sorted(set(candidate_ids)):
        raise ExternalClaimError("candidate ids must be sorted and unique")
    return candidate_ids, release_ids


def receipt(
    raw: bytes,
    candidate_ids: list[str],
    release_ids: list[str],
    public_surface_sha256: str,
    revocation_registry_sha256: str,
) -> dict[str, Any]:
    return {
        "schema": "civsim.external-adverse-claim-gate.v1",
        "registry_sha256": hashlib.sha256(raw).hexdigest(),
        "candidate_ids": candidate_ids,
        "release_ids": release_ids,
        "public_surface_sha256": public_surface_sha256,
        "revocation_registry_sha256": revocation_registry_sha256,
    }


def _synthetic_release(namespace: str) -> dict[str, Any]:
    lineages = []
    zero = "0" * 64
    for index in range(5):
        lineages.append(
            {
                "id": f"lineage-{index}",
                "source_ids": [f"source-{index}"],
                "anchors": [f"table-{index}"],
                "authors": [f"author-{index}"],
                "datasets": [f"dataset-{index}"],
                "apparatus": [f"apparatus-{index}"],
                "methods": [f"method-{index}"],
                "upstream_roots": [f"root-{index}"],
                "custody_sha256": [f"{index + 1:x}" * 64],
                "independence_basis": "Synthetic independent campaign for detector testing.",
            }
        )
    candidate = {
        "id": "test.claim",
        "state": "release",
        "subject": "neutral test subject",
        "private_dossier_sha256": zero,
        "action": "contact",
        "text_sha256": "1" * 64,
        "destination_sha256": "2" * 64,
        "subject_roots": ["source:subject"],
        "lineage": lineages,
        "lineage_set_sha256": hashlib.sha256(_canonical_json(lineages)).hexdigest(),
        "approver_identity": "owner",
        "expires_utc": "2099-01-01T00:00:00+00:00",
        "approval_revoked": False,
        "signature_path": "sources/external_claim_approvers",
    }
    candidate["approval_payload_sha256"] = hashlib.sha256(
        _approval_payload(candidate, namespace)
    ).hexdigest()
    return candidate


def self_test() -> None:
    base = tomllib.loads(REGISTRY.read_text(encoding="utf-8"))
    namespace = base["inventory"]["signature_namespace"]
    base["candidate"] = [_synthetic_release(namespace)]
    verifier = lambda *_args: True
    validate(
        base,
        now=dt.datetime(2026, 1, 1, tzinfo=dt.timezone.utc),
        signature_verifier=verifier,
        revoked_payloads=set(),
    )
    mutations: list[tuple[str, dict[str, Any]]] = []
    four = copy.deepcopy(base)
    four["candidate"][0]["lineage"].pop()
    lineages = four["candidate"][0]["lineage"]
    four["candidate"][0]["lineage_set_sha256"] = hashlib.sha256(
        _canonical_json(lineages)
    ).hexdigest()
    four["candidate"][0]["approval_payload_sha256"] = hashlib.sha256(
        _approval_payload(four["candidate"][0], namespace)
    ).hexdigest()
    mutations.append(("four lineages", four))
    shared = copy.deepcopy(base)
    shared["candidate"][0]["lineage"][1]["datasets"] = ["dataset-0"]
    lineages = shared["candidate"][0]["lineage"]
    shared["candidate"][0]["lineage_set_sha256"] = hashlib.sha256(
        _canonical_json(lineages)
    ).hexdigest()
    shared["candidate"][0]["approval_payload_sha256"] = hashlib.sha256(
        _approval_payload(shared["candidate"][0], namespace)
    ).hexdigest()
    mutations.append(("shared dataset", shared))
    shared_custody = copy.deepcopy(base)
    shared_custody["candidate"][0]["lineage"][1]["custody_sha256"] = ["5" * 64]
    lineages = shared_custody["candidate"][0]["lineage"]
    shared_custody["candidate"][0]["lineage_set_sha256"] = hashlib.sha256(
        _canonical_json(lineages)
    ).hexdigest()
    shared_custody["candidate"][0]["approval_payload_sha256"] = hashlib.sha256(
        _approval_payload(shared_custody["candidate"][0], namespace)
    ).hexdigest()
    mutations.append(("shared custody", shared_custody))
    for name, key, value in [
        ("text substitution", "text_sha256", "3" * 64),
        ("action substitution", "action", "publication"),
        ("revoked approval", "approval_revoked", True),
    ]:
        mutation = copy.deepcopy(base)
        mutation["candidate"][0][key] = value
        mutations.append((name, mutation))
    bypass = copy.deepcopy(base)
    bypass["candidate"][0]["single_witness_reason"] = "waiver"
    mutations.append(("single-witness bypass", bypass))
    for name, mutation in mutations:
        try:
            validate(
                mutation,
                now=dt.datetime(2026, 1, 1, tzinfo=dt.timezone.utc),
                signature_verifier=verifier,
                revoked_payloads=set(),
            )
        except ExternalClaimError:
            continue
        raise AssertionError(f"self-test mutation passed: {name}")
    try:
        validate(
            base,
            now=dt.datetime(2026, 1, 1, tzinfo=dt.timezone.utc),
            signature_verifier=verifier,
            revoked_payloads={base["candidate"][0]["approval_payload_sha256"]},
        )
    except ExternalClaimError:
        pass
    else:
        raise AssertionError("self-test protected revocation passed")
    try:
        lowered = "A DEFECT IN THE SOURCE".lower()
        if any(fragment in lowered for fragment in FORBIDDEN_PUBLIC_FRAGMENTS):
            raise ExternalClaimError("synthetic prohibited phrase")
    except ExternalClaimError:
        pass
    else:
        raise AssertionError("self-test adverse phrase passed")
    print("external adverse claim producer self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    try:
        if args.self_test:
            self_test()
            completed = subprocess.run(
                [sys.executable, str(WATCHDOG), "--self-test"], check=False
            )
            if completed.returncode != 0:
                raise ExternalClaimError("independent watchdog self-test failed")
            return 0
        raw = REGISTRY.read_bytes()
        data = tomllib.loads(raw.decode("utf-8"))
        candidate_ids, release_ids = validate(data)
        revocations = _repo_file(
            _text(data["inventory"], "revocations_file", "inventory"),
            "inventory.revocations_file",
        )
        _, revocations_sha256 = _revoked_payloads(revocations)
        produced = receipt(
            raw,
            candidate_ids,
            release_ids,
            _public_surface_digest(),
            revocations_sha256,
        )
        completed = subprocess.run(
            [sys.executable, str(WATCHDOG), "--receipt"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        if completed.returncode != 0:
            raise ExternalClaimError(
                f"independent watchdog failed: {completed.stderr.strip()}"
            )
        checked = json.loads(completed.stdout)
        if produced != checked:
            raise ExternalClaimError("producer and independent watchdog receipts differ")
        print(
            "external adverse claim gate: PASS "
            f"({len(candidate_ids)} candidate(s), {len(release_ids)} release(s))"
        )
        return 0
    except (
        OSError,
        UnicodeDecodeError,
        tomllib.TOMLDecodeError,
        json.JSONDecodeError,
        ExternalClaimError,
        AssertionError,
    ) as error:
        print(f"external adverse claim gate: FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
