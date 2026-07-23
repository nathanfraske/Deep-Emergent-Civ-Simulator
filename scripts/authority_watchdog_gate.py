#!/usr/bin/env python3
"""Produce and cross-check the closed authority-inventory receipt."""

from __future__ import annotations

import argparse
import copy
import hashlib
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import tomllib
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "scripts" / "authority_watchdog.toml"
CROSS_CHECKER = ROOT / "scripts" / "authority_registry_watchdog.py"
RECEIPT_SCHEMA = "civsim.authority-inventory-agreement.v3"
SOURCE_TEXT_CONTRACT = "canonical-git-lf-with-crlf-checkout-equivalence"

# This map is an independent, reviewed completeness pin. A registry edit cannot
# add, remove, promote, demote, or relabel a mechanism without changing this
# implementation and the separately coded watchdog.
REQUIRED_PROFILES: dict[str, tuple[str, str | None, str]] = {
    "core.deterministic-math-kernels": ("authority", "scientific", "blocked"),
    "core.deterministic-math-table": ("authority", "scientific", "active"),
    "floor.catalog-admission": ("authority", "scientific", "active"),
    "floor.pi-budget": ("authority", "scientific", "active"),
    "governance.authority-inventory": ("authority", "governance", "active"),
    "governance.external-adverse-claim-release": (
        "authority",
        "governance",
        "active",
    ),
    "governance.stone0-build-wiring": ("authority", "governance", "active"),
    "planet.completed-snapshot": ("authority", "scientific", "blocked"),
    "planet.species-derivation-frontier": ("diagnostic", None, "diagnostic"),
    "planet.species-state-support": ("authority", "scientific", "blocked"),
    "planet.stage1-dimensional-census": ("authority", "scientific", "blocked"),
    "planet.stellar-birth-proof-tokens": (
        "authority",
        "scientific",
        "blocked",
    ),
    "units.certified-formula-projection": (
        "authority",
        "scientific",
        "active",
    ),
    "units.si-execution-table": ("authority", "scientific", "blocked"),
    "units.si-representation-policy": ("authority", "scientific", "blocked"),
    "units.wide-integer-arithmetic": ("authority", "scientific", "blocked"),
}

# Each digest pins the complete canonical JSON encoding of one reviewed TOML
# row. This covers every claim or observation, implementation identity,
# schema, canary, shared primitive or material, owner boundary, guard,
# activation requirement, path, and semantic-closure member. Structural checks
# below explain each field, while this exact pin rejects any free-text or list
# substitution.
REQUIRED_PROFILE_DIGESTS: dict[str, str] = {
    "core.deterministic-math-kernels": "95af1b4ff8ecfd58bb0e0ab9f1f146b329db8a9ded5c2002e8d4d324d3a119fb",
    "core.deterministic-math-table": "081f9af077c3ef40746cf6adb97a2d82b54325dfb36501eb88dd353f467e4a9b",
    "floor.catalog-admission": "1f04f54954ab30f9709553bc4922ec9a2b8759f8f4b72fb2d2db1d7cbcceb389",
    "floor.pi-budget": "3ed6c43390a43cf921e96c600525a89da95cb62c522306acc725af93bc55d2f1",
    "governance.authority-inventory": "c2a6095c66f8dc4d9b2384a3b46c5da06142b27fad99b10ce6fc34239f5c5100",
    "governance.external-adverse-claim-release": "a8a74e0c6032ee15d125b7c2e00e4accf77de2c9ac7f9e675f5848f80f38295d",
    "governance.stone0-build-wiring": "69ffca7a6ad2d2654df9af89a6f50bddb7fb550d682aff2d85df90220bdaf92e",
    "planet.completed-snapshot": "dd72abfaa771ff707b884d66340830b4d3ba5378a36097dac3f13c11d94246ee",
    "planet.species-derivation-frontier": "7991ef98a413bd1ac4893bcc30b8cc2c9746a52b126d1a8864c5c6d1332f8d60",
    "planet.species-state-support": "f91597a9c743f37199e4c375e550fee7f4367c4b980f4e476ad55f11447672ac",
    "planet.stage1-dimensional-census": "21140b26c937f9cca7a8066b98e9fa75e9366f483ca44ffd1cc5206315f2b5dc",
    "planet.stellar-birth-proof-tokens": "68f58f019ab4f620194133e673c64240d4ad066f97404698ba0bfbf8b96935d1",
    "units.certified-formula-projection": "c1317c9000d82dc8507537120ce3c44a4a87886839a56d880c2d347a22266ff4",
    "units.si-execution-table": "cfdc61ffea91745443c5e9b170198caf15c1a118ec92d645de68053193dcd8ae",
    "units.si-representation-policy": "71d0b118afc0a88007ea19267e2277e89d4252f5654241182f22ab2663e51ce8",
    "units.wide-integer-arithmetic": "3bfb94aa0c3f507ad8ba9762e91ca9009284becc9d9752d7b7b896cb9a26ca7d",
}
REQUIRED_COUNTS = {
    "active": 7,
    "blocked": 8,
    "diagnostic": 1,
    "total": 16,
}
META_PAIR = {
    "producer_path": "scripts/authority_watchdog_gate.py",
    "producer_implementation": "civsim.authority-inventory.schema-first-producer.v3",
    "checker_path": "scripts/authority_registry_watchdog.py",
    "checker_implementation": "civsim.authority-inventory.profile-first-watchdog.v3",
    "receipt_schema": RECEIPT_SCHEMA,
}

INVENTORY_FIELDS = {"closed_world", "description", "rule", "schema"}
REQUIRED_INVENTORY_HEADER = {
    "schema": 3,
    "closed_world": True,
    "rule": "docs/working/INDEPENDENT_AUTHORITY_RULE.md",
    "description": "Closed inventory of authority-bearing mechanisms and explicitly non-authoritative diagnostics.",
}
AUTHORITY_COMMON_FIELDS = {
    "claim",
    "domain",
    "id",
    "kind",
    "owner_boundary",
    "producer_path",
    "status",
}
ACTIVE_AUTHORITY_FIELDS = {
    "checker_implementation",
    "checker_path",
    "canaries",
    "producer_implementation",
    "receipt_schema",
    "semantic_closure",
    "shared_primitives",
    "shared_semantic_helpers",
}
BLOCKED_AUTHORITY_FIELDS = {
    "activation_guard",
    "open_cross_checker_requirements",
    "refusal_path",
    "semantic_closure",
}
DIAGNOSTIC_FIELDS = {
    "authority_effect",
    "canaries",
    "diagnostic_checker_implementation",
    "diagnostic_checker_path",
    "diagnostic_producer_implementation",
    "diagnostic_producer_path",
    "diagnostic_schema",
    "id",
    "kind",
    "non_authority_guard",
    "observation",
    "owner_boundary",
    "semantic_closure",
    "shared_material",
    "status",
}
class AuthorityInventoryError(ValueError):
    """A closed inventory or pair receipt violated its contract."""


def _required_text(row: dict[str, Any], key: str, label: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value.strip():
        raise AuthorityInventoryError(f"{label}.{key} must be nonempty text")
    return value


def _required_text_list(row: dict[str, Any], key: str, label: str) -> list[str]:
    value = row.get(key)
    if not isinstance(value, list) or any(
        not isinstance(item, str) or not item.strip() for item in value
    ):
        raise AuthorityInventoryError(f"{label}.{key} must be a text array")
    if len(value) != len(set(value)):
        raise AuthorityInventoryError(f"{label}.{key} contains a duplicate")
    return value


def _exact_fields(row: dict[str, Any], expected: set[str], label: str) -> None:
    missing = sorted(expected - set(row))
    surplus = sorted(set(row) - expected)
    if not missing and not surplus:
        return
    details: list[str] = []
    if missing:
        details.append(f"missing {', '.join(missing)}")
    if surplus:
        details.append(f"unexpected {', '.join(surplus)}")
    raise AuthorityInventoryError(
        f"{label} does not match its closed schema: {'; '.join(details)}"
    )


def _repo_file(root: pathlib.Path, value: str, label: str) -> pathlib.Path:
    components = value.split("/")
    if (
        not value
        or "\\" in value
        or value.startswith("/")
        or any(component in {"", ".", ".."} for component in components)
    ):
        raise AuthorityInventoryError(f"{label} must be a canonical repository path")
    root_resolved = root.resolve()
    try:
        path = root.joinpath(*components).resolve(strict=True)
        path.relative_to(root_resolved)
    except (FileNotFoundError, OSError, ValueError) as error:
        raise AuthorityInventoryError(
            f"{label} does not name a repository file: {value}"
        ) from error
    if not path.is_file():
        raise AuthorityInventoryError(
            f"{label} does not name a repository file: {value}"
        )
    return path


def _profile_digest(row: dict[str, Any]) -> str:
    encoded = json.dumps(
        row, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    return hashlib.sha256(encoded).hexdigest()


def _validate_semantic_closure(
    row: dict[str, Any],
    root: pathlib.Path,
    label: str,
    required_paths: set[str],
) -> None:
    closure = _required_text_list(row, "semantic_closure", label)
    if not closure:
        raise AuthorityInventoryError(f"{label}.semantic_closure must be nonempty")
    missing = sorted(required_paths - set(closure))
    if missing:
        raise AuthorityInventoryError(
            f"{label}.semantic_closure omits declared path(s): {', '.join(missing)}"
        )
    for index, relative in enumerate(closure):
        _repo_file(
            root,
            relative,
            f"{label}.semantic_closure[{index}]",
        )


def _validate_active(
    row: dict[str, Any], root: pathlib.Path, label: str, mechanism_id: str
) -> None:
    _exact_fields(row, AUTHORITY_COMMON_FIELDS | ACTIVE_AUTHORITY_FIELDS, label)
    producer = _repo_file(
        root, _required_text(row, "producer_path", label), f"{label}.producer_path"
    )
    checker = _repo_file(
        root, _required_text(row, "checker_path", label), f"{label}.checker_path"
    )
    if producer == checker:
        raise AuthorityInventoryError(
            f"{label} producer and checker paths must differ"
        )
    if _required_text(row, "producer_implementation", label) == _required_text(
        row, "checker_implementation", label
    ):
        raise AuthorityInventoryError(
            f"{label} producer and checker implementation identities must differ"
        )
    if _required_text_list(row, "shared_semantic_helpers", label):
        raise AuthorityInventoryError(
            f"{label} active pair may not share semantic decision helpers"
        )
    if not _required_text_list(row, "shared_primitives", label):
        raise AuthorityInventoryError(
            f"{label} must state its shared low-level primitives"
        )
    if len(_required_text_list(row, "canaries", label)) < 3:
        raise AuthorityInventoryError(
            f"{label} active pair requires at least three mutation canaries"
        )
    _required_text(row, "receipt_schema", label)
    _validate_semantic_closure(
        row,
        root,
        label,
        {
            _required_text(row, "producer_path", label),
            _required_text(row, "checker_path", label),
        },
    )
    if mechanism_id == "governance.authority-inventory":
        for key, expected in META_PAIR.items():
            if row.get(key) != expected:
                raise AuthorityInventoryError(
                    f"{label}.{key} must remain pinned to {expected}"
                )


def _validate_blocked(
    row: dict[str, Any], root: pathlib.Path, label: str
) -> None:
    _exact_fields(row, AUTHORITY_COMMON_FIELDS | BLOCKED_AUTHORITY_FIELDS, label)
    _repo_file(
        root, _required_text(row, "producer_path", label), f"{label}.producer_path"
    )
    _repo_file(
        root, _required_text(row, "refusal_path", label), f"{label}.refusal_path"
    )
    _required_text(row, "activation_guard", label)
    if not _required_text_list(row, "open_cross_checker_requirements", label):
        raise AuthorityInventoryError(
            f"{label} must name its open cross-checker requirements"
        )
    _validate_semantic_closure(
        row,
        root,
        label,
        {
            _required_text(row, "producer_path", label),
            _required_text(row, "refusal_path", label),
        },
    )


def _validate_diagnostic(
    row: dict[str, Any], root: pathlib.Path, label: str
) -> None:
    _exact_fields(row, DIAGNOSTIC_FIELDS, label)
    if row.get("authority_effect") != "none":
        raise AuthorityInventoryError(f"{label}.authority_effect must equal none")
    _required_text(row, "observation", label)
    _required_text(row, "non_authority_guard", label)
    producer = _repo_file(
        root,
        _required_text(row, "diagnostic_producer_path", label),
        f"{label}.diagnostic_producer_path",
    )
    checker = _repo_file(
        root,
        _required_text(row, "diagnostic_checker_path", label),
        f"{label}.diagnostic_checker_path",
    )
    if producer == checker:
        raise AuthorityInventoryError(
            f"{label} diagnostic producer and checker paths must differ"
        )
    if _required_text(row, "diagnostic_producer_implementation", label) == (
        _required_text(row, "diagnostic_checker_implementation", label)
    ):
        raise AuthorityInventoryError(
            f"{label} diagnostic implementation identities must differ"
        )
    _required_text(row, "diagnostic_schema", label)
    if not _required_text_list(row, "shared_material", label):
        raise AuthorityInventoryError(
            f"{label} must disclose shared diagnostic material"
        )
    if not _required_text_list(row, "canaries", label):
        raise AuthorityInventoryError(
            f"{label} diagnostic requires mutation canaries"
        )
    _validate_semantic_closure(
        row,
        root,
        label,
        {
            _required_text(row, "diagnostic_producer_path", label),
            _required_text(row, "diagnostic_checker_path", label),
        },
    )


def parse_registry(raw: bytes) -> dict[str, Any]:
    try:
        data = tomllib.load(io.BytesIO(raw))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as error:
        raise AuthorityInventoryError(f"registry is not valid TOML: {error}") from error
    if not isinstance(data, dict):
        raise AuthorityInventoryError("registry root must be a table")
    return data


def validate_inventory(data: dict[str, Any], root: pathlib.Path) -> None:
    if set(data) != {"inventory", "mechanism"}:
        raise AuthorityInventoryError(
            "registry top-level fields must equal inventory and mechanism"
        )
    inventory = data.get("inventory")
    if not isinstance(inventory, dict):
        raise AuthorityInventoryError("inventory must be a table")
    _exact_fields(inventory, INVENTORY_FIELDS, "inventory")
    if inventory != REQUIRED_INVENTORY_HEADER:
        raise AuthorityInventoryError(
            "inventory header differs from its reviewed exact pin"
        )
    if inventory.get("schema") != 3:
        raise AuthorityInventoryError("inventory.schema must equal 3")
    if inventory.get("closed_world") is not True:
        raise AuthorityInventoryError("inventory.closed_world must be true")
    _required_text(inventory, "description", "inventory")
    _repo_file(
        root,
        _required_text(inventory, "rule", "inventory"),
        "inventory.rule",
    )

    rows = data.get("mechanism")
    if not isinstance(rows, list) or not rows:
        raise AuthorityInventoryError("mechanism inventory must be nonempty")
    identifiers: list[str] = []
    for index, row in enumerate(rows):
        label = f"mechanism[{index}]"
        if not isinstance(row, dict):
            raise AuthorityInventoryError(f"{label} must be a table")
        mechanism_id = _required_text(row, "id", label)
        identifiers.append(mechanism_id)
        profile = REQUIRED_PROFILES.get(mechanism_id)
        if profile is None:
            raise AuthorityInventoryError(
                f"{label}.id is outside the closed required-mechanism pin"
            )
        required_kind, required_domain, required_status = profile
        if row.get("kind") != required_kind:
            raise AuthorityInventoryError(
                f"{label}.kind must remain {required_kind} for {mechanism_id}"
            )
        if row.get("status") != required_status:
            raise AuthorityInventoryError(
                f"{label}.status must remain {required_status} for {mechanism_id}"
            )
        _required_text(row, "owner_boundary", label)
        if required_kind == "authority":
            if row.get("domain") != required_domain:
                raise AuthorityInventoryError(
                    f"{label}.domain must remain {required_domain} for {mechanism_id}"
                )
            _required_text(row, "claim", label)
            if required_status == "active":
                _validate_active(row, root, label, mechanism_id)
            else:
                _validate_blocked(row, root, label)
        else:
            _validate_diagnostic(row, root, label)
        found_digest = _profile_digest(row)
        expected_digest = REQUIRED_PROFILE_DIGESTS[mechanism_id]
        if found_digest != expected_digest:
            raise AuthorityInventoryError(
                f"{label} semantic profile differs from its reviewed exact pin"
            )

    if len(identifiers) != len(set(identifiers)):
        raise AuthorityInventoryError("mechanism inventory contains a duplicate id")
    if set(identifiers) != set(REQUIRED_PROFILES):
        missing = sorted(set(REQUIRED_PROFILES) - set(identifiers))
        extra = sorted(set(identifiers) - set(REQUIRED_PROFILES))
        details: list[str] = []
        if missing:
            details.append(f"missing {', '.join(missing)}")
        if extra:
            details.append(f"extra {', '.join(extra)}")
        raise AuthorityInventoryError(
            "mechanism inventory does not match the closed required-mechanism pin: "
            + "; ".join(details)
        )
    if identifiers != sorted(identifiers):
        raise AuthorityInventoryError("mechanism ids must be sorted")
    if set(REQUIRED_PROFILE_DIGESTS) != set(REQUIRED_PROFILES):
        raise AuthorityInventoryError(
            "required profile digest pin and classification pin differ"
        )
    observed_counts = {
        "active": sum(
            row["kind"] == "authority" and row["status"] == "active"
            for row in rows
        ),
        "blocked": sum(
            row["kind"] == "authority" and row["status"] == "blocked"
            for row in rows
        ),
        "diagnostic": sum(row["kind"] == "diagnostic" for row in rows),
        "total": len(rows),
    }
    if observed_counts != REQUIRED_COUNTS:
        raise AuthorityInventoryError(
            "mechanism counts differ from the reviewed exact count pin"
        )


def _semantic_closure_files(
    data: dict[str, Any], root: pathlib.Path
) -> list[dict[str, str]]:
    relative_paths = {
        relative
        for row in data["mechanism"]
        for relative in row["semantic_closure"]
    }
    result: list[dict[str, str]] = []
    for relative in sorted(relative_paths):
        path = _repo_file(root, relative, f"receipt path {relative}")
        content = _canonical_repository_text(path.read_bytes(), relative)
        result.append(
            {"path": relative, "sha256": hashlib.sha256(content).hexdigest()}
        )
    return result


def _canonical_repository_text(raw: bytes, label: str) -> bytes:
    """Map a repository text checkout to Git LF bytes and reject bare CR."""

    without_pairs = raw.replace(b"\r\n", b"")
    if b"\r" in without_pairs:
        raise AuthorityInventoryError(
            f"{label} contains a bare carriage return outside the text contract"
        )
    return raw.replace(b"\r\n", b"\n")


def build_receipt(raw: bytes, root: pathlib.Path = ROOT) -> bytes:
    canonical_registry = _canonical_repository_text(raw, "authority registry")
    data = parse_registry(canonical_registry)
    validate_inventory(data, root)
    profiles = [
        {
            "domain": row.get("domain"),
            "id": row["id"],
            "kind": row["kind"],
            "semantic_profile_sha256": _profile_digest(row),
            "status": row["status"],
        }
        for row in data["mechanism"]
    ]
    receipt = {
        "active_authority_count": sum(
            row["kind"] == "authority" and row["status"] == "active"
            for row in data["mechanism"]
        ),
        "blocked_authority_count": sum(
            row["kind"] == "authority" and row["status"] == "blocked"
            for row in data["mechanism"]
        ),
        "closed_world": data["inventory"]["closed_world"],
        "diagnostic_count": sum(
            row["kind"] == "diagnostic" for row in data["mechanism"]
        ),
        "inventory_schema": data["inventory"]["schema"],
        "mechanism_count": len(data["mechanism"]),
        "profiles": profiles,
        "semantic_closure_files": _semantic_closure_files(data, root),
        "registry_sha256": hashlib.sha256(canonical_registry).hexdigest(),
        "schema": RECEIPT_SCHEMA,
        "source_text_contract": SOURCE_TEXT_CONTRACT,
    }
    return json.dumps(
        receipt, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def require_receipt_agreement(local: bytes, independent: bytes) -> None:
    if local != independent:
        raise AuthorityInventoryError(
            "schema-first producer and profile-first watchdog receipts differ"
        )


def cross_check_receipt(
    local_receipt: bytes,
    registry: pathlib.Path = REGISTRY,
    root: pathlib.Path = ROOT,
) -> None:
    completed = subprocess.run(
        [
            sys.executable,
            str(CROSS_CHECKER),
            "--receipt",
            "--registry",
            str(registry),
        ],
        cwd=root,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise AuthorityInventoryError(
            f"independent registry watchdog failed with {completed.returncode}: {detail}"
        )
    if completed.stderr:
        raise AuthorityInventoryError(
            "independent registry watchdog emitted unexpected stderr"
        )
    expected_stdout = local_receipt + b"\n"
    if completed.stdout != expected_stdout:
        raise AuthorityInventoryError(
            "independent registry watchdog did not emit the exact canonical receipt"
        )
    require_receipt_agreement(local_receipt, completed.stdout[:-1])


def run_cross_checker_self_test(root: pathlib.Path = ROOT) -> None:
    completed = subprocess.run(
        [sys.executable, str(CROSS_CHECKER), "--self-test"],
        cwd=root,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if completed.returncode != 0:
        detail = (completed.stdout + completed.stderr).decode(
            "utf-8", errors="replace"
        ).strip()
        raise AuthorityInventoryError(
            f"independent registry watchdog self-test failed: {detail}"
        )
    if completed.stderr:
        raise AuthorityInventoryError(
            "independent registry watchdog self-test emitted unexpected stderr"
        )
    if completed.stdout.decode("utf-8", errors="strict").splitlines() != [
        "authority registry watchdog self-test: PASS"
    ]:
        raise AuthorityInventoryError(
            "independent registry watchdog self-test emitted unexpected output"
        )


def _required_mutations(data: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    rows = data["mechanism"]
    row_index = {row["id"]: index for index, row in enumerate(rows)}
    blocked_index = next(
        index
        for index, row in enumerate(rows)
        if row["kind"] == "authority" and row["status"] == "blocked"
    )
    diagnostic_index = next(
        index for index, row in enumerate(rows) if row["kind"] == "diagnostic"
    )

    truncated = copy.deepcopy(data)
    truncated["mechanism"].pop()

    reclassified = copy.deepcopy(data)
    reclassified["mechanism"][blocked_index]["kind"] = "diagnostic"

    diagnostic_mint = copy.deepcopy(data)
    diagnostic_mint["mechanism"][diagnostic_index]["authority_effect"] = "mint"

    same_identity = copy.deepcopy(data)
    active_index = next(
        index
        for index, row in enumerate(rows)
        if row["kind"] == "authority" and row["status"] == "active"
    )
    same_identity["mechanism"][active_index]["checker_implementation"] = (
        same_identity["mechanism"][active_index]["producer_implementation"]
    )

    semantic_helper = copy.deepcopy(data)
    semantic_helper["mechanism"][active_index]["shared_semantic_helpers"] = [
        "expected_answer"
    ]

    missing_path = copy.deepcopy(data)
    missing_path["mechanism"][active_index]["checker_path"] = (
        "scripts/authority_registry_watchdog_missing.py"
    )

    too_few_canaries = copy.deepcopy(data)
    too_few_canaries["mechanism"][active_index]["canaries"] = ["one", "two"]

    promoted = copy.deepcopy(data)
    promoted["mechanism"][blocked_index]["status"] = "active"

    claim_broadening = copy.deepcopy(data)
    claim_broadening["mechanism"][
        row_index["units.certified-formula-projection"]
    ]["claim"] += " This also authorizes every SI table row."

    implementation_substitution = copy.deepcopy(data)
    implementation_substitution["mechanism"][
        row_index["units.certified-formula-projection"]
    ]["checker_implementation"] = "civsim.units.substituted-watchdog.v1"

    schema_substitution = copy.deepcopy(data)
    schema_substitution["mechanism"][
        row_index["units.certified-formula-projection"]
    ]["receipt_schema"] = "civsim.units.substituted-receipt.v1"

    canary_substitution = copy.deepcopy(data)
    canary_substitution["mechanism"][
        row_index["units.certified-formula-projection"]
    ]["canaries"][0] = "substituted canary"

    owner_substitution = copy.deepcopy(data)
    owner_substitution["mechanism"][row_index["floor.pi-budget"]][
        "owner_boundary"
    ] += " The checker now decides completeness."

    guard_substitution = copy.deepcopy(data)
    guard_substitution["mechanism"][
        row_index["units.si-execution-table"]
    ]["activation_guard"] = "A substituted guard."

    requirement_substitution = copy.deepcopy(data)
    requirement_substitution["mechanism"][
        row_index["core.deterministic-math-kernels"]
    ]["open_cross_checker_requirements"][0] = "a weaker table check"

    observation_substitution = copy.deepcopy(data)
    observation_substitution["mechanism"][
        row_index["planet.species-derivation-frontier"]
    ]["observation"] += " The same material now closes the proof."

    shared_material_substitution = copy.deepcopy(data)
    shared_material_substitution["mechanism"][
        row_index["planet.species-derivation-frontier"]
    ]["shared_material"][0] = "substituted floor view"

    adapter_omission = copy.deepcopy(data)
    adapter_omission["mechanism"][
        row_index["units.certified-formula-projection"]
    ]["semantic_closure"].remove("crates/units/src/compute.rs")

    orchestrator_substitution = copy.deepcopy(data)
    governance_closure = orchestrator_substitution["mechanism"][
        row_index["governance.authority-inventory"]
    ]["semantic_closure"]
    governance_closure[governance_closure.index("scripts/gates.toml")] = "justfile"

    header_substitution = copy.deepcopy(data)
    header_substitution["inventory"]["description"] = "A looser inventory."

    return [
        ("truncated inventory", truncated),
        ("authority reclassification", reclassified),
        ("diagnostic authority mint", diagnostic_mint),
        ("same implementation identity", same_identity),
        ("shared semantic helper", semantic_helper),
        ("missing referenced path", missing_path),
        ("insufficient active canaries", too_few_canaries),
        ("unpaired blocked mechanism promotion", promoted),
        ("claim broadening", claim_broadening),
        ("implementation substitution", implementation_substitution),
        ("receipt schema substitution", schema_substitution),
        ("canary substitution", canary_substitution),
        ("owner boundary substitution", owner_substitution),
        ("activation guard substitution", guard_substitution),
        ("activation requirement substitution", requirement_substitution),
        ("diagnostic observation substitution", observation_substitution),
        ("shared material substitution", shared_material_substitution),
        ("semantic closure adapter omission", adapter_omission),
        ("semantic closure orchestrator substitution", orchestrator_substitution),
        ("inventory header substitution", header_substitution),
    ]


def _exercise_semantic_closure_hashes(raw: bytes, data: dict[str, Any]) -> None:
    adapter = "crates/units/src/certified_projection/mod.rs"
    orchestrator = "scripts/gates.toml"
    closure_paths = {
        relative
        for row in data["mechanism"]
        for relative in row["semantic_closure"]
    }
    with tempfile.TemporaryDirectory(prefix="civsim-authority-producer-") as directory:
        temporary_root = pathlib.Path(directory)
        for relative in closure_paths:
            target = temporary_root.joinpath(*relative.split("/"))
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(_repo_file(ROOT, relative, relative).read_bytes())

        baseline = build_receipt(raw, temporary_root)
        for label, relative in (
            ("adapter", adapter),
            ("orchestrator", orchestrator),
        ):
            target = temporary_root.joinpath(*relative.split("/"))
            original = target.read_bytes()
            target.write_bytes(original + f"\n# {label} mutation canary\n".encode())
            if build_receipt(raw, temporary_root) == baseline:
                raise AssertionError(
                    f"semantic closure ignored {label} content mutation"
                )
            target.write_bytes(original)

        omitted = temporary_root.joinpath(*adapter.split("/"))
        omitted.unlink()
        try:
            build_receipt(raw, temporary_root)
        except AuthorityInventoryError:
            pass
        else:
            raise AssertionError("semantic closure accepted an omitted adapter file")
        omitted.write_bytes(_repo_file(ROOT, adapter, adapter).read_bytes())

        portable = temporary_root.joinpath(*orchestrator.split("/"))
        canonical = _canonical_repository_text(
            portable.read_bytes(), "semantic closure portability canary"
        )
        portable.write_bytes(canonical)
        canonical_receipt = build_receipt(raw, temporary_root)
        portable.write_bytes(canonical.replace(b"\n", b"\r\n"))
        if build_receipt(raw, temporary_root) != canonical_receipt:
            raise AssertionError("CRLF closure checkout changed the canonical receipt")
        portable.write_bytes(canonical + b"\r")
        try:
            build_receipt(raw, temporary_root)
        except AuthorityInventoryError:
            pass
        else:
            raise AssertionError("semantic closure accepted a bare carriage return")


def self_test() -> None:
    raw = REGISTRY.read_bytes()
    data = parse_registry(raw)
    validate_inventory(data, ROOT)

    for name, mutation in _required_mutations(data):
        try:
            validate_inventory(mutation, ROOT)
        except AuthorityInventoryError:
            continue
        raise AssertionError(f"self-test mutation passed: {name}")

    baseline_receipt = build_receipt(raw)
    canonical_registry = _canonical_repository_text(raw, "authority registry")
    crlf_registry = canonical_registry.replace(b"\n", b"\r\n")
    if build_receipt(crlf_registry) != baseline_receipt:
        raise AssertionError("CRLF registry checkout changed the canonical receipt")
    try:
        build_receipt(canonical_registry + b"\r")
    except AuthorityInventoryError:
        pass
    else:
        raise AssertionError("registry accepted a bare carriage return")
    input_mutation = raw + b"\n# authority inventory input mutation canary\n"
    mutated_input_receipt = build_receipt(input_mutation)
    if mutated_input_receipt == baseline_receipt:
        raise AssertionError("registry input mutation did not change the receipt")
    _exercise_semantic_closure_hashes(raw, data)

    decoded = json.loads(baseline_receipt)
    decoded["registry_sha256"] = "0" * 64
    mutated_receipt = json.dumps(
        decoded, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    try:
        require_receipt_agreement(baseline_receipt, mutated_receipt)
    except AuthorityInventoryError:
        pass
    else:
        raise AssertionError("receipt mutation passed")

    cross_check_receipt(baseline_receipt)
    run_cross_checker_self_test()
    print("authority inventory producer self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    try:
        if args.self_test:
            self_test()
        else:
            receipt = build_receipt(REGISTRY.read_bytes())
            cross_check_receipt(receipt)
            print("authority inventory producer and watchdog: PASS")
            print(receipt.decode("ascii"))
    except (
        AssertionError,
        AuthorityInventoryError,
        json.JSONDecodeError,
        OSError,
        subprocess.SubprocessError,
    ) as error:
        print(f"authority inventory producer and watchdog: FAIL: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
