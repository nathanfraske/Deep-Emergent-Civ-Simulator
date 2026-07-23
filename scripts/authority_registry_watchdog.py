#!/usr/bin/env python3
"""Independently validate and receipt the closed authority inventory."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import pathlib
import sys
import tempfile
import tomllib
from typing import Any


REPOSITORY = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_INVENTORY = REPOSITORY / "scripts" / "authority_watchdog.toml"
OUTPUT_SCHEMA = "civsim.authority-inventory-agreement.v3"
TEXT_BYTE_RULE = "canonical-git-lf-with-crlf-checkout-equivalence"

# Kept separate from authority_watchdog_gate.py on purpose. This validator
# starts from a closed ordered profile and then checks each row against it.
EXPECTED_PROFILE: tuple[tuple[str, str, str | None, str, str], ...] = (
    (
        "core.deterministic-math-kernels",
        "authority",
        "scientific",
        "blocked",
        "95af1b4ff8ecfd58bb0e0ab9f1f146b329db8a9ded5c2002e8d4d324d3a119fb",
    ),
    (
        "core.deterministic-math-table",
        "authority",
        "scientific",
        "active",
        "2082968dcb55ee6abc1f8af83c4d1e7fb4955ca4fb44af7e36e2f7d5fefa745c",
    ),
    (
        "floor.catalog-admission",
        "authority",
        "scientific",
        "active",
        "1f04f54954ab30f9709553bc4922ec9a2b8759f8f4b72fb2d2db1d7cbcceb389",
    ),
    (
        "floor.pi-budget",
        "authority",
        "scientific",
        "active",
        "3ed6c43390a43cf921e96c600525a89da95cb62c522306acc725af93bc55d2f1",
    ),
    (
        "governance.authority-inventory",
        "authority",
        "governance",
        "active",
        "c2a6095c66f8dc4d9b2384a3b46c5da06142b27fad99b10ce6fc34239f5c5100",
    ),
    (
        "governance.external-adverse-claim-release",
        "authority",
        "governance",
        "active",
        "a8a74e0c6032ee15d125b7c2e00e4accf77de2c9ac7f9e675f5848f80f38295d",
    ),
    (
        "governance.stone0-build-wiring",
        "authority",
        "governance",
        "active",
        "69ffca7a6ad2d2654df9af89a6f50bddb7fb550d682aff2d85df90220bdaf92e",
    ),
    (
        "planet.completed-snapshot",
        "authority",
        "scientific",
        "blocked",
        "dd72abfaa771ff707b884d66340830b4d3ba5378a36097dac3f13c11d94246ee",
    ),
    (
        "planet.species-derivation-frontier",
        "diagnostic",
        None,
        "diagnostic",
        "7991ef98a413bd1ac4893bcc30b8cc2c9746a52b126d1a8864c5c6d1332f8d60",
    ),
    (
        "planet.species-state-support",
        "authority",
        "scientific",
        "blocked",
        "f91597a9c743f37199e4c375e550fee7f4367c4b980f4e476ad55f11447672ac",
    ),
    (
        "planet.stage1-dimensional-census",
        "authority",
        "scientific",
        "blocked",
        "21140b26c937f9cca7a8066b98e9fa75e9366f483ca44ffd1cc5206315f2b5dc",
    ),
    (
        "planet.stellar-birth-proof-tokens",
        "authority",
        "scientific",
        "blocked",
        "68f58f019ab4f620194133e673c64240d4ad066f97404698ba0bfbf8b96935d1",
    ),
    (
        "units.certified-formula-projection",
        "authority",
        "scientific",
        "active",
        "c1317c9000d82dc8507537120ce3c44a4a87886839a56d880c2d347a22266ff4",
    ),
    (
        "units.si-execution-table",
        "authority",
        "scientific",
        "blocked",
        "cfdc61ffea91745443c5e9b170198caf15c1a118ec92d645de68053193dcd8ae",
    ),
    (
        "units.si-representation-policy",
        "authority",
        "scientific",
        "blocked",
        "71d0b118afc0a88007ea19267e2277e89d4252f5654241182f22ab2663e51ce8",
    ),
    (
        "units.wide-integer-arithmetic",
        "authority",
        "scientific",
        "blocked",
        "3bfb94aa0c3f507ad8ba9762e91ca9009284becc9d9752d7b7b896cb9a26ca7d",
    ),
)
EXPECTED_COUNTS = (7, 8, 1, 16)

META_EXPECTATIONS = (
    ("producer_path", "scripts/authority_watchdog_gate.py"),
    (
        "producer_implementation",
        "civsim.authority-inventory.schema-first-producer.v3",
    ),
    ("checker_path", "scripts/authority_registry_watchdog.py"),
    (
        "checker_implementation",
        "civsim.authority-inventory.profile-first-watchdog.v3",
    ),
    ("receipt_schema", OUTPUT_SCHEMA),
)

HEADER_KEYS = frozenset(("closed_world", "description", "rule", "schema"))
AUTHORITY_BASE = frozenset(
    ("claim", "domain", "id", "kind", "owner_boundary", "producer_path", "status")
)
ACTIVE_KEYS = frozenset(
    (
        "checker_implementation",
        "checker_path",
        "canaries",
        "producer_implementation",
        "receipt_schema",
        "semantic_closure",
        "shared_primitives",
        "shared_semantic_helpers",
    )
)
BLOCKED_KEYS = frozenset(
    (
        "activation_guard",
        "open_cross_checker_requirements",
        "refusal_path",
        "semantic_closure",
    )
)
DIAGNOSTIC_KEYS = frozenset(
    (
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
    )
)


class RegistryWatchdogFailure(ValueError):
    """The independent registry observation could not be certified."""


def _word(table: dict[str, Any], field: str, context: str) -> str:
    candidate = table.get(field)
    if not isinstance(candidate, str) or candidate.strip() == "":
        raise RegistryWatchdogFailure(f"{context}.{field} is not nonempty text")
    return candidate


def _word_vector(table: dict[str, Any], field: str, context: str) -> tuple[str, ...]:
    candidate = table.get(field)
    if not isinstance(candidate, list):
        raise RegistryWatchdogFailure(f"{context}.{field} is not a text array")
    output: list[str] = []
    for item in candidate:
        if not isinstance(item, str) or item.strip() == "":
            raise RegistryWatchdogFailure(
                f"{context}.{field} contains a non-text or empty member"
            )
        output.append(item)
    if len(output) != len(frozenset(output)):
        raise RegistryWatchdogFailure(f"{context}.{field} repeats a member")
    return tuple(output)


def _schema(table: dict[str, Any], required: frozenset[str], context: str) -> None:
    found = frozenset(table)
    if found == required:
        return
    absent = sorted(required - found)
    foreign = sorted(found - required)
    message: list[str] = []
    if absent:
        message.append("absent " + ", ".join(absent))
    if foreign:
        message.append("foreign " + ", ".join(foreign))
    raise RegistryWatchdogFailure(
        f"{context} violates the closed field set: {'; '.join(message)}"
    )


def _open_repository_file(
    relative: str,
    context: str,
    repository: pathlib.Path = REPOSITORY,
) -> pathlib.Path:
    if (
        relative == ""
        or relative.startswith("/")
        or "\\" in relative
        or any(piece in ("", ".", "..") for piece in relative.split("/"))
    ):
        raise RegistryWatchdogFailure(
            f"{context} is not a canonical repository-relative path"
        )
    base = repository.resolve()
    try:
        candidate = (repository / pathlib.PurePosixPath(relative)).resolve(strict=True)
        candidate.relative_to(base)
    except (FileNotFoundError, OSError, ValueError) as error:
        raise RegistryWatchdogFailure(
            f"{context} is absent or leaves the repository: {relative}"
        ) from error
    if not candidate.is_file():
        raise RegistryWatchdogFailure(f"{context} is not a file: {relative}")
    return candidate


def _profile_fingerprint(entry: dict[str, Any]) -> str:
    canonical = json.dumps(
        entry, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(canonical).hexdigest()


def _semantic_files(
    entry: dict[str, Any],
    context: str,
    required: frozenset[str],
    repository: pathlib.Path,
) -> None:
    closure = _word_vector(entry, "semantic_closure", context)
    if len(closure) == 0:
        raise RegistryWatchdogFailure(f"{context} has an empty semantic closure")
    absent = sorted(required - frozenset(closure))
    if absent:
        raise RegistryWatchdogFailure(
            f"{context} semantic closure misses declared path(s): {', '.join(absent)}"
        )
    for position, relative in enumerate(closure):
        _open_repository_file(
            relative,
            f"{context}.semantic_closure[{position}]",
            repository,
        )


def decode_inventory(raw: bytes) -> dict[str, Any]:
    try:
        decoded = raw.decode("utf-8")
        document = tomllib.loads(decoded)
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise RegistryWatchdogFailure(f"inventory syntax refused: {error}") from error
    if type(document) is not dict:
        raise RegistryWatchdogFailure("inventory document is not a table")
    return document


def inspect_inventory(
    document: dict[str, Any],
    repository: pathlib.Path = REPOSITORY,
) -> None:
    if tuple(sorted(document)) != ("inventory", "mechanism"):
        raise RegistryWatchdogFailure(
            "top-level inventory shape is not the closed two-key schema"
        )
    header = document["inventory"]
    entries = document["mechanism"]
    if type(header) is not dict:
        raise RegistryWatchdogFailure("inventory header is not a table")
    _schema(header, HEADER_KEYS, "inventory")
    if header.get("schema") != 3:
        raise RegistryWatchdogFailure("inventory schema is not version 3")
    if header.get("closed_world") is not True:
        raise RegistryWatchdogFailure("inventory does not declare a closed world")
    if _word(header, "description", "inventory") != (
        "Closed inventory of authority-bearing mechanisms and explicitly "
        "non-authoritative diagnostics."
    ):
        raise RegistryWatchdogFailure(
            "inventory description differs from its independent exact pin"
        )
    if _word(header, "rule", "inventory") != (
        "docs/working/INDEPENDENT_AUTHORITY_RULE.md"
    ):
        raise RegistryWatchdogFailure(
            "inventory rule path differs from its independent exact pin"
        )
    _open_repository_file(
        header["rule"],
        "inventory.rule",
        repository,
    )
    if not isinstance(entries, list):
        raise RegistryWatchdogFailure("mechanism collection is not an array")
    if len(entries) != len(EXPECTED_PROFILE):
        raise RegistryWatchdogFailure(
            "mechanism count differs from the independent required profile"
        )

    identifiers: list[str] = []
    active_count = 0
    blocked_count = 0
    diagnostic_count = 0
    for position, (entry, expected) in enumerate(zip(entries, EXPECTED_PROFILE)):
        context = f"mechanism[{position}]"
        if type(entry) is not dict:
            raise RegistryWatchdogFailure(f"{context} is not a table")
        (
            expected_id,
            expected_kind,
            expected_domain,
            expected_status,
            expected_fingerprint,
        ) = expected
        identifier = _word(entry, "id", context)
        identifiers.append(identifier)
        if identifier != expected_id:
            raise RegistryWatchdogFailure(
                f"{context}.id is {identifier}, expected {expected_id}"
            )
        if entry.get("kind") != expected_kind:
            raise RegistryWatchdogFailure(
                f"{context}.kind changed from pinned {expected_kind}"
            )
        if entry.get("status") != expected_status:
            raise RegistryWatchdogFailure(
                f"{context}.status changed from pinned {expected_status}"
            )
        _word(entry, "owner_boundary", context)
        if expected_kind == "diagnostic":
            diagnostic_count += 1
            _inspect_diagnostic(entry, context, repository)
        else:
            if entry.get("domain") != expected_domain:
                raise RegistryWatchdogFailure(
                    f"{context}.domain changed from pinned {expected_domain}"
                )
            _word(entry, "claim", context)
            if expected_status == "active":
                active_count += 1
                _inspect_active(entry, context, expected_id, repository)
            else:
                blocked_count += 1
                _inspect_blocked(entry, context, repository)
        if _profile_fingerprint(entry) != expected_fingerprint:
            raise RegistryWatchdogFailure(
                f"{context} differs from its full reviewed semantic profile"
            )
    if len(identifiers) != len(frozenset(identifiers)):
        raise RegistryWatchdogFailure("mechanism identifiers are not unique")
    if (
        active_count,
        blocked_count,
        diagnostic_count,
        len(entries),
    ) != EXPECTED_COUNTS:
        raise RegistryWatchdogFailure(
            "mechanism counts differ from the independent exact count pin"
        )


def _inspect_active(
    entry: dict[str, Any],
    context: str,
    mechanism_id: str,
    repository: pathlib.Path,
) -> None:
    _schema(entry, AUTHORITY_BASE | ACTIVE_KEYS, context)
    producer_name = _word(entry, "producer_path", context)
    checker_name = _word(entry, "checker_path", context)
    producer_file = _open_repository_file(
        producer_name, f"{context}.producer_path", repository
    )
    checker_file = _open_repository_file(
        checker_name, f"{context}.checker_path", repository
    )
    if producer_file.samefile(checker_file):
        raise RegistryWatchdogFailure(f"{context} aliases one file for both sides")
    if _word(entry, "producer_implementation", context) == _word(
        entry, "checker_implementation", context
    ):
        raise RegistryWatchdogFailure(
            f"{context} repeats one implementation identity"
        )
    if _word_vector(entry, "shared_semantic_helpers", context) != ():
        raise RegistryWatchdogFailure(
            f"{context} declares a shared semantic decision helper"
        )
    if len(_word_vector(entry, "shared_primitives", context)) == 0:
        raise RegistryWatchdogFailure(f"{context} omits shared primitives")
    if len(_word_vector(entry, "canaries", context)) < 3:
        raise RegistryWatchdogFailure(f"{context} has fewer than three canaries")
    _word(entry, "receipt_schema", context)
    _semantic_files(
        entry,
        context,
        frozenset((producer_name, checker_name)),
        repository,
    )
    if mechanism_id == "governance.authority-inventory":
        for field, pinned in META_EXPECTATIONS:
            if entry.get(field) != pinned:
                raise RegistryWatchdogFailure(
                    f"{context}.{field} differs from its meta-pair pin"
                )


def _inspect_blocked(
    entry: dict[str, Any],
    context: str,
    repository: pathlib.Path,
) -> None:
    _schema(entry, AUTHORITY_BASE | BLOCKED_KEYS, context)
    producer_name = _word(entry, "producer_path", context)
    refusal_name = _word(entry, "refusal_path", context)
    _open_repository_file(
        producer_name,
        f"{context}.producer_path",
        repository,
    )
    _open_repository_file(
        refusal_name,
        f"{context}.refusal_path",
        repository,
    )
    _word(entry, "activation_guard", context)
    if len(_word_vector(entry, "open_cross_checker_requirements", context)) == 0:
        raise RegistryWatchdogFailure(
            f"{context} omits its cross-checker requirements"
        )
    _semantic_files(
        entry,
        context,
        frozenset((producer_name, refusal_name)),
        repository,
    )


def _inspect_diagnostic(
    entry: dict[str, Any],
    context: str,
    repository: pathlib.Path,
) -> None:
    _schema(entry, DIAGNOSTIC_KEYS, context)
    if entry.get("authority_effect") != "none":
        raise RegistryWatchdogFailure(
            f"{context} diagnostic attempts an authority effect"
        )
    _word(entry, "observation", context)
    _word(entry, "non_authority_guard", context)
    producer_name = _word(entry, "diagnostic_producer_path", context)
    checker_name = _word(entry, "diagnostic_checker_path", context)
    producer = _open_repository_file(
        producer_name,
        f"{context}.diagnostic_producer_path",
        repository,
    )
    checker = _open_repository_file(
        checker_name,
        f"{context}.diagnostic_checker_path",
        repository,
    )
    if producer.samefile(checker):
        raise RegistryWatchdogFailure(
            f"{context} aliases one diagnostic implementation file"
        )
    if _word(entry, "diagnostic_producer_implementation", context) == _word(
        entry, "diagnostic_checker_implementation", context
    ):
        raise RegistryWatchdogFailure(
            f"{context} repeats one diagnostic implementation identity"
        )
    _word(entry, "diagnostic_schema", context)
    if len(_word_vector(entry, "shared_material", context)) == 0:
        raise RegistryWatchdogFailure(
            f"{context} does not disclose shared diagnostic material"
        )
    if len(_word_vector(entry, "canaries", context)) == 0:
        raise RegistryWatchdogFailure(f"{context} has no diagnostic canaries")
    _semantic_files(
        entry,
        context,
        frozenset((producer_name, checker_name)),
        repository,
    )


def _files_observed(
    document: dict[str, Any],
    repository: pathlib.Path,
) -> list[dict[str, str]]:
    names = {
        relative
        for entry in document["mechanism"]
        for relative in entry["semantic_closure"]
    }
    observed: list[dict[str, str]] = []
    for name in sorted(names):
        checkout = _open_repository_file(
            name, f"semantic closure path {name}", repository
        ).read_bytes()
        content = _repository_lf_bytes(checkout, name)
        observed.append({"path": name, "sha256": hashlib.sha256(content).hexdigest()})
    return observed


def _repository_lf_bytes(checkout: bytes, label: str) -> bytes:
    """Independently decode CRLF checkout text into repository LF bytes."""

    normalized = bytearray()
    position = 0
    while position < len(checkout):
        value = checkout[position]
        if value != 13:
            normalized.append(value)
            position += 1
            continue
        if position + 1 >= len(checkout) or checkout[position + 1] != 10:
            raise RegistryWatchdogFailure(
                f"{label} has a carriage return not paired with line feed"
            )
        normalized.append(10)
        position += 2
    return bytes(normalized)


def issue_receipt(
    raw: bytes,
    repository: pathlib.Path = REPOSITORY,
) -> bytes:
    canonical_inventory = _repository_lf_bytes(raw, "authority inventory")
    document = decode_inventory(canonical_inventory)
    inspect_inventory(document, repository)
    profiles: list[dict[str, str | None]] = []
    active = 0
    blocked = 0
    diagnostics = 0
    for entry in document["mechanism"]:
        profiles.append(
            {
                "domain": entry.get("domain"),
                "id": entry["id"],
                "kind": entry["kind"],
                "semantic_profile_sha256": _profile_fingerprint(entry),
                "status": entry["status"],
            }
        )
        if entry["kind"] == "diagnostic":
            diagnostics += 1
        elif entry["status"] == "active":
            active += 1
        else:
            blocked += 1
    payload = {
        "active_authority_count": active,
        "blocked_authority_count": blocked,
        "closed_world": document["inventory"]["closed_world"],
        "diagnostic_count": diagnostics,
        "inventory_schema": document["inventory"]["schema"],
        "mechanism_count": len(document["mechanism"]),
        "profiles": profiles,
        "semantic_closure_files": _files_observed(document, repository),
        "registry_sha256": hashlib.sha256(canonical_inventory).hexdigest(),
        "schema": OUTPUT_SCHEMA,
        "source_text_contract": TEXT_BYTE_RULE,
    }
    return json.dumps(
        payload, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")


def _receipts_match(first: bytes, second: bytes) -> None:
    if first != second:
        raise RegistryWatchdogFailure("canonical inventory receipts do not match")


def _mandatory_corruptions(
    document: dict[str, Any],
) -> tuple[tuple[str, dict[str, Any]], ...]:
    positions = {
        row["id"]: number for number, row in enumerate(document["mechanism"])
    }
    authority_position = next(
        number
        for number, row in enumerate(document["mechanism"])
        if row["kind"] == "authority" and row["status"] == "blocked"
    )
    diagnostic_position = next(
        number
        for number, row in enumerate(document["mechanism"])
        if row["kind"] == "diagnostic"
    )

    short = copy.deepcopy(document)
    del short["mechanism"][0]

    disguised = copy.deepcopy(document)
    disguised["mechanism"][authority_position]["kind"] = "diagnostic"

    minting = copy.deepcopy(document)
    minting["mechanism"][diagnostic_position]["authority_effect"] = "mint"

    same_file = copy.deepcopy(document)
    active_position = next(
        number
        for number, row in enumerate(document["mechanism"])
        if row["kind"] == "authority" and row["status"] == "active"
    )
    same_file["mechanism"][active_position]["checker_path"] = same_file["mechanism"][
        active_position
    ]["producer_path"]

    semantic = copy.deepcopy(document)
    semantic["mechanism"][active_position]["shared_semantic_helpers"] = [
        "decision_boolean"
    ]

    missing = copy.deepcopy(document)
    missing["mechanism"][active_position]["checker_path"] = (
        "scripts/authority_registry_watchdog_missing.py"
    )

    canary_shortage = copy.deepcopy(document)
    canary_shortage["mechanism"][active_position]["canaries"] = ["first", "second"]

    promoted = copy.deepcopy(document)
    promoted["mechanism"][authority_position]["status"] = "active"

    broad = copy.deepcopy(document)
    broad["mechanism"][positions["units.certified-formula-projection"]][
        "claim"
    ] += " This authorizes the complete SI table."

    implementation = copy.deepcopy(document)
    implementation["mechanism"][
        positions["units.certified-formula-projection"]
    ]["producer_implementation"] = "civsim.units.replacement-producer.v1"

    schema = copy.deepcopy(document)
    schema["mechanism"][positions["units.certified-formula-projection"]][
        "receipt_schema"
    ] = "civsim.units.replacement-schema.v1"

    canary = copy.deepcopy(document)
    canary["mechanism"][positions["units.certified-formula-projection"]][
        "canaries"
    ][-1] = "replacement canary"

    owner = copy.deepcopy(document)
    owner["mechanism"][positions["floor.pi-budget"]]["owner_boundary"] = (
        "The checker decides scientific completeness."
    )

    guard = copy.deepcopy(document)
    guard["mechanism"][positions["units.si-representation-policy"]][
        "activation_guard"
    ] = "A replacement activation guard."

    requirement = copy.deepcopy(document)
    requirement["mechanism"][positions["core.deterministic-math-kernels"]][
        "open_cross_checker_requirements"
    ][-1] = "a replacement requirement"

    observation = copy.deepcopy(document)
    observation["mechanism"][positions["planet.species-derivation-frontier"]][
        "observation"
    ] = "The diagnostic now closes Stage 1."

    material = copy.deepcopy(document)
    material["mechanism"][positions["planet.species-derivation-frontier"]][
        "shared_material"
    ][-1] = "replacement shared material"

    adapter_missing = copy.deepcopy(document)
    adapter_missing["mechanism"][
        positions["units.certified-formula-projection"]
    ]["semantic_closure"].remove("crates/units/src/compute.rs")

    orchestrator_changed = copy.deepcopy(document)
    meta_closure = orchestrator_changed["mechanism"][
        positions["governance.authority-inventory"]
    ]["semantic_closure"]
    meta_closure[meta_closure.index("scripts/gates.toml")] = "justfile"

    header = copy.deepcopy(document)
    header["inventory"]["description"] = "A replacement inventory description."

    return (
        ("truncated inventory", short),
        ("authority reclassification", disguised),
        ("diagnostic authority mint", minting),
        ("same implementation path", same_file),
        ("shared semantic helper", semantic),
        ("missing referenced path", missing),
        ("insufficient active canaries", canary_shortage),
        ("unpaired blocked mechanism promotion", promoted),
        ("claim broadening", broad),
        ("implementation substitution", implementation),
        ("schema substitution", schema),
        ("canary substitution", canary),
        ("owner boundary substitution", owner),
        ("activation guard substitution", guard),
        ("activation requirement substitution", requirement),
        ("diagnostic observation substitution", observation),
        ("shared material substitution", material),
        ("semantic closure adapter omission", adapter_missing),
        ("semantic closure orchestrator substitution", orchestrator_changed),
        ("inventory header substitution", header),
    )


def _check_closure_content_binding(
    raw: bytes,
    document: dict[str, Any],
) -> None:
    paths = frozenset(
        member
        for entry in document["mechanism"]
        for member in entry["semantic_closure"]
    )
    adapter = "crates/units/src/certified_projection/mod.rs"
    orchestrator = "scripts/gates.toml"
    with tempfile.TemporaryDirectory(prefix="civsim-authority-watchdog-") as name:
        sandbox = pathlib.Path(name)
        for member in paths:
            destination = sandbox / pathlib.PurePosixPath(member)
            destination.parent.mkdir(parents=True, exist_ok=True)
            source = _open_repository_file(member, member)
            destination.write_bytes(source.read_bytes())

        baseline = issue_receipt(raw, sandbox)
        for role, member in (("adapter", adapter), ("orchestrator", orchestrator)):
            path = sandbox / pathlib.PurePosixPath(member)
            held = path.read_bytes()
            path.write_bytes(held + f"\n# watchdog {role} canary\n".encode("ascii"))
            if issue_receipt(raw, sandbox) == baseline:
                raise AssertionError(
                    f"watchdog receipt ignored {role} semantic closure mutation"
                )
            path.write_bytes(held)

        missing = sandbox / pathlib.PurePosixPath(orchestrator)
        missing.unlink()
        try:
            issue_receipt(raw, sandbox)
        except RegistryWatchdogFailure:
            pass
        else:
            raise AssertionError(
                "watchdog accepted an omitted semantic closure orchestrator"
            )
        missing.write_bytes(
            _open_repository_file(orchestrator, orchestrator).read_bytes()
        )

        portable = sandbox / pathlib.PurePosixPath(adapter)
        canonical = _repository_lf_bytes(portable.read_bytes(), adapter)
        portable.write_bytes(canonical)
        canonical_receipt = issue_receipt(raw, sandbox)
        portable.write_bytes(canonical.replace(b"\n", b"\r\n"))
        if issue_receipt(raw, sandbox) != canonical_receipt:
            raise AssertionError("watchdog did not equate LF and CRLF closure text")
        portable.write_bytes(canonical + b"\r")
        try:
            issue_receipt(raw, sandbox)
        except RegistryWatchdogFailure:
            pass
        else:
            raise AssertionError("watchdog accepted bare CR closure text")


def self_test() -> None:
    original = DEFAULT_INVENTORY.read_bytes()
    document = decode_inventory(original)
    inspect_inventory(document)
    for label, corrupt in _mandatory_corruptions(document):
        try:
            inspect_inventory(corrupt)
        except RegistryWatchdogFailure:
            pass
        else:
            raise AssertionError(f"watchdog corruption survived: {label}")
    _check_closure_content_binding(original, document)

    baseline = issue_receipt(original)
    canonical_inventory = _repository_lf_bytes(original, "authority inventory")
    crlf_inventory = canonical_inventory.replace(b"\n", b"\r\n")
    if issue_receipt(crlf_inventory) != baseline:
        raise AssertionError("watchdog did not equate LF and CRLF registry text")
    try:
        issue_receipt(canonical_inventory + b"\r")
    except RegistryWatchdogFailure:
        pass
    else:
        raise AssertionError("watchdog accepted bare CR registry text")
    changed_input = original + b"\n# registry watchdog input mutation canary\n"
    if issue_receipt(changed_input) == baseline:
        raise AssertionError("watchdog receipt ignored an exact input mutation")

    changed_payload = json.loads(baseline)
    changed_payload["mechanism_count"] += 1
    counterfeit = json.dumps(
        changed_payload, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    try:
        _receipts_match(baseline, counterfeit)
    except RegistryWatchdogFailure:
        pass
    else:
        raise AssertionError("watchdog accepted a mutated receipt")
    print("authority registry watchdog self-test: PASS")


def main() -> int:
    arguments = argparse.ArgumentParser()
    arguments.add_argument("--self-test", action="store_true")
    arguments.add_argument("--receipt", action="store_true")
    arguments.add_argument("--registry", type=pathlib.Path, default=DEFAULT_INVENTORY)
    options = arguments.parse_args()
    try:
        if options.self_test:
            self_test()
            return 0
        receipt = issue_receipt(options.registry.read_bytes())
        if options.receipt:
            sys.stdout.buffer.write(receipt + b"\n")
        else:
            print("authority registry watchdog: PASS")
            print(receipt.decode("ascii"))
    except (
        AssertionError,
        json.JSONDecodeError,
        OSError,
        RegistryWatchdogFailure,
    ) as error:
        if options.receipt:
            print(f"authority registry watchdog: FAIL: {error}", file=sys.stderr)
        else:
            print(f"authority registry watchdog: FAIL: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
