#!/usr/bin/env python3
"""Independent graph-traversal watchdog for external-claim releases."""

from __future__ import annotations

import argparse
import copy
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys
import tomllib
from typing import Any, Callable


REPOSITORY = pathlib.Path(__file__).resolve().parent.parent
CLAIMS = REPOSITORY / "sources" / "external_claims.toml"
HEX = frozenset("0123456789abcdef")
LINEAGE_FIELDS = frozenset(
    {
        "id",
        "source_ids",
        "anchors",
        "authors",
        "datasets",
        "apparatus",
        "methods",
        "upstream_roots",
        "custody_sha256",
        "independence_basis",
    }
)
PENDING_FIELDS = frozenset({"id", "state", "subject", "private_dossier_sha256"})
RELEASE_FIELDS = PENDING_FIELDS | frozenset(
    {
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
)
BLOCKED_PHRASES = frozenset(
    {
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
    }
)


class WatchdogFailure(ValueError):
    pass


def string(table: dict[str, Any], field: str, context: str) -> str:
    result = table.get(field)
    if not isinstance(result, str) or result.strip() == "":
        raise WatchdogFailure(f"{context}.{field} is not nonempty text")
    return result


def digest(table: dict[str, Any], field: str, context: str) -> str:
    result = string(table, field, context)
    if len(result) != 64 or set(result) - HEX:
        raise WatchdogFailure(f"{context}.{field} is not lowercase SHA-256")
    return result


def string_list(
    table: dict[str, Any], field: str, context: str, *, empty: bool = False
) -> list[str]:
    result = table.get(field)
    if not isinstance(result, list) or any(
        not isinstance(item, str) or item.strip() == "" for item in result
    ):
        raise WatchdogFailure(f"{context}.{field} is not a text array")
    if not empty and not result:
        raise WatchdogFailure(f"{context}.{field} is empty")
    if result != sorted(result) or len(result) != len(set(result)):
        raise WatchdogFailure(f"{context}.{field} is not sorted and unique")
    return result


def repository_file(name: str, context: str) -> pathlib.Path:
    relative = pathlib.PurePosixPath(name)
    if relative.is_absolute() or ".." in relative.parts:
        raise WatchdogFailure(f"{context} escapes the repository")
    result = REPOSITORY.joinpath(*relative.parts)
    if not result.is_file():
        raise WatchdogFailure(f"{context} is missing: {name}")
    return result


def json_bytes(value: Any) -> bytes:
    text = json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
    return f"{text}\n".encode("utf-8")


def inspect_public_surfaces() -> str:
    roots = [
        REPOSITORY / "crates" / "physics" / "data",
        REPOSITORY / "docs" / "working",
        REPOSITORY / "sources",
    ]
    exception = REPOSITORY / "docs" / "working" / "EXTERNAL_ADVERSE_CLAIM_RULE.md"
    paths = sorted(
        {
            item
            for root in roots
            for item in root.rglob("*")
            if item.is_file()
            and item != exception
            and item.suffix.casefold() in (".md", ".toml", ".txt")
        }
    )
    result = hashlib.sha256()
    for path in paths:
        body = path.read_bytes()
        prose = body.decode("utf-8").casefold()
        relative = path.relative_to(REPOSITORY).as_posix()
        collision = next((phrase for phrase in BLOCKED_PHRASES if phrase in prose), None)
        if collision is not None:
            raise WatchdogFailure(
                f"public surface {relative} contains unreleased phrase '{collision}'"
            )
        name = relative.encode("utf-8")
        for block in (name, body):
            result.update(len(block).to_bytes(8, byteorder="little"))
            result.update(block)
    return result.hexdigest()


def normalize_lineage(table: dict[str, Any], context: str) -> dict[str, Any]:
    if frozenset(table) != LINEAGE_FIELDS:
        raise WatchdogFailure(f"{context} violates the closed lineage schema")
    normalized: dict[str, Any] = {
        "id": string(table, "id", context),
        "source_ids": string_list(table, "source_ids", context),
        "anchors": string_list(table, "anchors", context),
        "authors": string_list(table, "authors", context),
        "datasets": string_list(table, "datasets", context),
        "apparatus": string_list(table, "apparatus", context),
        "methods": string_list(table, "methods", context),
        "upstream_roots": string_list(table, "upstream_roots", context),
        "custody_sha256": string_list(table, "custody_sha256", context),
        "independence_basis": string(table, "independence_basis", context),
    }
    for value in normalized["custody_sha256"]:
        if len(value) != 64 or set(value) - HEX:
            raise WatchdogFailure(f"{context}.custody_sha256 has an invalid digest")
    return normalized


def evidence_roots(lineage: dict[str, Any]) -> frozenset[str]:
    result: set[str] = set()
    fields = (
        ("source_ids", "source"),
        ("authors", "author"),
        ("datasets", "dataset"),
        ("apparatus", "apparatus"),
        ("methods", "method"),
        ("upstream_roots", "upstream"),
        ("custody_sha256", "custody"),
    )
    for field, kind in fields:
        result.update(f"{kind}:{value}" for value in lineage[field])
    return frozenset(result)


def graph_component_count(
    lineages: list[dict[str, Any]], subject_roots: frozenset[str]
) -> int:
    roots = [evidence_roots(lineage) for lineage in lineages]
    adjacency = [set() for _ in lineages]
    for left in range(len(lineages)):
        for right in range(left + 1, len(lineages)):
            if roots[left].intersection(roots[right]):
                adjacency[left].add(right)
                adjacency[right].add(left)
    unseen = set(range(len(lineages)))
    independent = 0
    while unseen:
        start = min(unseen)
        stack = [start]
        component: set[int] = set()
        while stack:
            node = stack.pop()
            if node in component:
                continue
            component.add(node)
            unseen.discard(node)
            stack.extend(adjacency[node] - component)
        combined = set().union(*(roots[node] for node in component))
        if not combined.intersection(subject_roots):
            independent += 1
    return independent


def signed_payload(row: dict[str, Any], namespace: str) -> bytes:
    fields = {
        "schema": "civsim.external-adverse-claim-approval.v1",
        "candidate_id": row["id"],
        "subject": row["subject"],
        "subject_roots": row["subject_roots"],
        "action": row["action"],
        "text_sha256": row["text_sha256"],
        "destination_sha256": row["destination_sha256"],
        "private_dossier_sha256": row["private_dossier_sha256"],
        "lineage_set_sha256": row["lineage_set_sha256"],
        "approver_identity": row["approver_identity"],
        "approval_revoked": row["approval_revoked"],
        "expires_utc": row["expires_utc"],
        "signature_namespace": namespace,
    }
    return json_bytes(fields)


def revoked_payloads(path: pathlib.Path) -> tuple[frozenset[str], str]:
    raw = path.read_bytes()
    document = tomllib.loads(raw.decode("utf-8"))
    if frozenset(document) != frozenset(
        {"schema", "revoked_approval_payload_sha256"}
    ) or document.get("schema") != 1:
        raise WatchdogFailure("revocation registry violates its closed schema")
    values = document["revoked_approval_payload_sha256"]
    if not isinstance(values, list) or values != sorted(values) or len(values) != len(set(values)):
        raise WatchdogFailure("revocation registry must be sorted and unique")
    for value in values:
        if not isinstance(value, str) or len(value) != 64 or set(value) - HEX:
            raise WatchdogFailure("revocation registry contains a non-SHA-256 value")
    return frozenset(values), hashlib.sha256(raw).hexdigest()


def openssh_verify(
    allowed: pathlib.Path,
    namespace: str,
    identity: str,
    signature: pathlib.Path,
    payload: bytes,
) -> bool:
    process = subprocess.run(
        [
            "ssh-keygen",
            "-Y",
            "verify",
            "-f",
            str(allowed),
            "-I",
            identity,
            "-n",
            namespace,
            "-s",
            str(signature),
        ],
        input=payload,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return process.returncode == 0


def audit(
    document: dict[str, Any],
    *,
    clock: datetime.datetime | None = None,
    verifier: Callable[[pathlib.Path, str, str, pathlib.Path, bytes], bool] = openssh_verify,
    revoked: frozenset[str] | None = None,
) -> tuple[list[str], list[str]]:
    header = document.get("inventory")
    if not isinstance(header, dict) or header.get("schema") != 1:
        raise WatchdogFailure("inventory schema is not 1")
    if header.get("minimum_independent_lineages") != 5:
        raise WatchdogFailure("independent-lineage threshold is not 5")
    repository_file(string(header, "rule", "inventory"), "inventory.rule")
    allowed_signers = repository_file(
        string(header, "approvers_file", "inventory"), "inventory.approvers_file"
    )
    namespace = string(header, "signature_namespace", "inventory")
    revocation_path = repository_file(
        string(header, "revocations_file", "inventory"),
        "inventory.revocations_file",
    )
    revoked_set = revoked_payloads(revocation_path)[0] if revoked is None else revoked
    candidates = document.get("candidate", [])
    if not isinstance(candidates, list):
        raise WatchdogFailure("candidate records are not an array")
    ids: list[str] = []
    releases: list[str] = []
    instant = clock or datetime.datetime.now(datetime.timezone.utc)
    for number, candidate in enumerate(candidates):
        context = f"candidate[{number}]"
        if not isinstance(candidate, dict):
            raise WatchdogFailure(f"{context} is not a table")
        identifier = string(candidate, "id", context)
        ids.append(identifier)
        string(candidate, "subject", context)
        state = string(candidate, "state", context)
        fields = frozenset(candidate)
        if state == "internal_review":
            if not fields.issubset(PENDING_FIELDS):
                raise WatchdogFailure(f"{context} exposes release material while pending")
            if "private_dossier_sha256" in candidate:
                digest(candidate, "private_dossier_sha256", context)
            continue
        if state != "release" or fields != RELEASE_FIELDS:
            raise WatchdogFailure(f"{context} is neither a closed pending nor release row")
        releases.append(identifier)
        if string(candidate, "action", context) not in ("contact", "publication"):
            raise WatchdogFailure(f"{context} has an unsupported action")
        for field in (
            "private_dossier_sha256",
            "text_sha256",
            "destination_sha256",
            "lineage_set_sha256",
            "approval_payload_sha256",
        ):
            digest(candidate, field, context)
        if candidate.get("approval_revoked") is not False:
            raise WatchdogFailure(f"{context} approval is revoked")
        subject = frozenset(string_list(candidate, "subject_roots", context))
        valid_prefixes = (
            "source:",
            "author:",
            "dataset:",
            "apparatus:",
            "method:",
            "upstream:",
            "custody:",
        )
        if any(not root.startswith(valid_prefixes) for root in subject):
            raise WatchdogFailure(f"{context} has an untyped subject root")
        raw_lineages = candidate.get("lineage")
        if not isinstance(raw_lineages, list):
            raise WatchdogFailure(f"{context}.lineage is not an array")
        lineages = [
            normalize_lineage(value, f"{context}.lineage[{index}]")
            for index, value in enumerate(raw_lineages)
        ]
        lineage_ids = [lineage["id"] for lineage in lineages]
        if lineage_ids != sorted(lineage_ids) or len(lineage_ids) != len(set(lineage_ids)):
            raise WatchdogFailure(f"{context} lineage ids are not sorted and unique")
        found_lineage_digest = hashlib.sha256(json_bytes(lineages)).hexdigest()
        if found_lineage_digest != candidate["lineage_set_sha256"]:
            raise WatchdogFailure(f"{context} lineage digest mismatch")
        if graph_component_count(lineages, subject) < 5:
            raise WatchdogFailure(f"{context} lacks five independent graph components")
        payload = signed_payload(candidate, namespace)
        if hashlib.sha256(payload).hexdigest() != candidate["approval_payload_sha256"]:
            raise WatchdogFailure(f"{context} approval payload mismatch")
        if candidate["approval_payload_sha256"] in revoked_set:
            raise WatchdogFailure(f"{context} approval payload is revoked")
        try:
            expiry = datetime.datetime.fromisoformat(string(candidate, "expires_utc", context))
        except ValueError as error:
            raise WatchdogFailure(f"{context} has a malformed expiry") from error
        if expiry.tzinfo is None or expiry <= instant:
            raise WatchdogFailure(f"{context} approval is expired or timezone-free")
        signature = repository_file(
            string(candidate, "signature_path", context), f"{context}.signature_path"
        )
        identity = string(candidate, "approver_identity", context)
        if not verifier(allowed_signers, namespace, identity, signature, payload):
            raise WatchdogFailure(f"{context} signature is not owner-valid")
    if ids != sorted(ids) or len(ids) != len(set(ids)):
        raise WatchdogFailure("candidate ids are not sorted and unique")
    return ids, releases


def make_receipt(
    raw: bytes,
    ids: list[str],
    releases: list[str],
    public_surface_sha256: str,
    revocation_registry_sha256: str,
) -> dict[str, Any]:
    return {
        "schema": "civsim.external-adverse-claim-gate.v1",
        "registry_sha256": hashlib.sha256(raw).hexdigest(),
        "candidate_ids": ids,
        "release_ids": releases,
        "public_surface_sha256": public_surface_sha256,
        "revocation_registry_sha256": revocation_registry_sha256,
    }


def fixture() -> dict[str, Any]:
    data = tomllib.loads(CLAIMS.read_text(encoding="utf-8"))
    lineages = []
    for index in range(5):
        lineages.append(
            {
                "id": f"witness-{index}",
                "source_ids": [f"s{index}"],
                "anchors": [f"a{index}"],
                "authors": [f"people{index}"],
                "datasets": [f"data{index}"],
                "apparatus": [f"instrument{index}"],
                "methods": [f"method{index}"],
                "upstream_roots": [f"root{index}"],
                "custody_sha256": [f"{index + 5:x}" * 64],
                "independence_basis": "Synthetic watchdog canary lineage.",
            }
        )
    claim = {
        "id": "watchdog.fixture",
        "state": "release",
        "subject": "neutral synthetic subject",
        "private_dossier_sha256": "a" * 64,
        "action": "publication",
        "text_sha256": "b" * 64,
        "destination_sha256": "c" * 64,
        "subject_roots": ["source:subject"],
        "lineage": lineages,
        "lineage_set_sha256": hashlib.sha256(json_bytes(lineages)).hexdigest(),
        "approver_identity": "owner",
        "expires_utc": "2099-01-01T00:00:00+00:00",
        "approval_revoked": False,
        "signature_path": "sources/external_claim_approvers",
    }
    namespace = data["inventory"]["signature_namespace"]
    claim["approval_payload_sha256"] = hashlib.sha256(
        signed_payload(claim, namespace)
    ).hexdigest()
    data["candidate"] = [claim]
    return data


def self_test() -> None:
    data = fixture()
    yes = lambda *_args: True
    test_time = datetime.datetime(2026, 1, 1, tzinfo=datetime.timezone.utc)
    namespace = data["inventory"]["signature_namespace"]
    audit(data, clock=test_time, verifier=yes, revoked=frozenset())
    mutations: list[tuple[str, dict[str, Any]]] = []
    overlap = copy.deepcopy(data)
    overlap["candidate"][0]["lineage"][1]["upstream_roots"] = ["root0"]
    lineages = overlap["candidate"][0]["lineage"]
    overlap["candidate"][0]["lineage_set_sha256"] = hashlib.sha256(
        json_bytes(lineages)
    ).hexdigest()
    overlap["candidate"][0]["approval_payload_sha256"] = hashlib.sha256(
        signed_payload(overlap["candidate"][0], namespace)
    ).hexdigest()
    mutations.append(("shared ancestry", overlap))
    shared_custody = copy.deepcopy(data)
    shared_custody["candidate"][0]["lineage"][1]["custody_sha256"] = ["5" * 64]
    lineages = shared_custody["candidate"][0]["lineage"]
    shared_custody["candidate"][0]["lineage_set_sha256"] = hashlib.sha256(
        json_bytes(lineages)
    ).hexdigest()
    shared_custody["candidate"][0]["approval_payload_sha256"] = hashlib.sha256(
        signed_payload(shared_custody["candidate"][0], namespace)
    ).hexdigest()
    mutations.append(("shared custody", shared_custody))
    destination = copy.deepcopy(data)
    destination["candidate"][0]["destination_sha256"] = "d" * 64
    mutations.append(("destination substitution", destination))
    revoked = copy.deepcopy(data)
    revoked["candidate"][0]["approval_revoked"] = True
    mutations.append(("revocation", revoked))
    subject = copy.deepcopy(data)
    subject["candidate"][0]["subject_roots"] = ["source:s0"]
    mutations.append(("subject lineage", subject))
    for name, mutation in mutations:
        try:
            audit(mutation, clock=test_time, verifier=yes, revoked=frozenset())
        except WatchdogFailure:
            continue
        raise AssertionError(f"watchdog mutation passed: {name}")
    try:
        audit(
            data,
            clock=test_time,
            verifier=lambda *_args: False,
            revoked=frozenset(),
        )
    except WatchdogFailure:
        pass
    else:
        raise AssertionError("watchdog accepted an unknown signer")
    try:
        audit(
            data,
            clock=test_time,
            verifier=yes,
            revoked=frozenset({data["candidate"][0]["approval_payload_sha256"]}),
        )
    except WatchdogFailure:
        pass
    else:
        raise AssertionError("watchdog accepted a protected revocation")
    if not any(phrase in "paper is wrong" for phrase in BLOCKED_PHRASES):
        raise AssertionError("watchdog adverse phrase canary did not fire")
    print("external adverse claim watchdog self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    options = parser.parse_args()
    try:
        if options.self_test:
            self_test()
            return 0
        raw = CLAIMS.read_bytes()
        document = tomllib.loads(raw.decode("utf-8"))
        ids, releases = audit(document)
        revocation_path = repository_file(
            string(document["inventory"], "revocations_file", "inventory"),
            "inventory.revocations_file",
        )
        _, revocation_digest = revoked_payloads(revocation_path)
        result = make_receipt(
            raw,
            ids,
            releases,
            inspect_public_surfaces(),
            revocation_digest,
        )
        if options.receipt:
            print(json.dumps(result, sort_keys=True, separators=(",", ":")))
        else:
            print(
                "external adverse claim watchdog: PASS "
                f"({len(ids)} candidate(s), {len(releases)} release(s))"
            )
        return 0
    except (
        OSError,
        UnicodeDecodeError,
        tomllib.TOMLDecodeError,
        WatchdogFailure,
        AssertionError,
    ) as error:
        print(f"external adverse claim watchdog: FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
