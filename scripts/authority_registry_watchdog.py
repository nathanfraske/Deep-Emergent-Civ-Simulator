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
        "081f9af077c3ef40746cf6adb97a2d82b54325dfb36501eb88dd353f467e4a9b",
    ),
    (
        "floor.catalog-admission",
        "authority",
        "scientific",
        "active",
        "e1b8cfd716ffaa4a184aa4656b5f9aa1553e5452d19009593099b525daabc0b8",
    ),
    (
        "floor.codata-factual-row-binding",
        "authority",
        "scientific",
        "active",
        "3151bd9388f1bed1601724c421a4cdcd1e0a8c7de06d6127aeec4af48bbadd1b",
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
        "dd2fd45fa3257bfff9105cfb11d2a329915c310e3e384bbba5ebcb9075cd9005",
    ),
    (
        "planet.charged-profile-admission",
        "authority",
        "scientific",
        "active",
        "bf25f00fdf77165403022377797dd2b320a825849bbb449a7d51a880a347b4c2",
    ),
    (
        "planet.completed-snapshot",
        "authority",
        "scientific",
        "blocked",
        "5c09a8d95b7de145110a9f97f773bb0e0a63d6d6dc0f964eeb8beb62efff5575",
    ),
    (
        "planet.confining-profile-admission",
        "authority",
        "scientific",
        "active",
        "ebb6f778511e67a8ea2794cacbf296f03ec0912d904d84185948ca472baa48aa",
    ),
    (
        "planet.derived-law-premise-eps0",
        "authority",
        "scientific",
        "active",
        "9ce5e04b3cf043616148062cb4f7eada303fe03a421aef004fa02970d41e26c2",
    ),
    (
        "planet.law-premise-admission-route",
        "authority",
        "scientific",
        "blocked",
        "ad77326e8ac1004ad8c086f6d6b3eaaa7f39c618e5ef9a8375765498e817b505",
    ),
    (
        "planet.neutral-bound-profile-admission",
        "authority",
        "scientific",
        "active",
        "68a9224196e287ac8250037d21e6d3f2c0ba928532286ae63f9e6c67b593eec2",
    ),
    (
        "planet.physical-vocabulary-partition",
        "diagnostic",
        None,
        "diagnostic",
        "d858bcf769f9e3e0eaa9e9d19d047f5abfd6fac3a0092f64811b4d8d4cdb5c1f",
    ),
    (
        "planet.primitive-profile-admission",
        "authority",
        "scientific",
        "active",
        "fa4afeaab276f1f3302dc6b8e34647b620a856f0523d51b66cd46082e7bd20c1",
    ),
    (
        "planet.species-derivation-frontier",
        "diagnostic",
        None,
        "diagnostic",
        "8c29fdf5dd04d3ef6ada48c3685154aac9d6f059a82a6fd66b3826ec9103dd2c",
    ),
    (
        "planet.species-state-support",
        "authority",
        "scientific",
        "blocked",
        "d537f116e5d0d665fd74a8308a8868aa4700c59b13ccd1e42ca2b3d5476bea6b",
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
        "planet.stellar-species-floor-coordinate-projection",
        "authority",
        "scientific",
        "active",
        "95a30ad85a29d7d549693c657d4a869187f3ce60bdad2b97fb77d792eefafd92",
    ),
    (
        "planet.symmetry-operator-exclusion",
        "authority",
        "scientific",
        "blocked",
        "31359d3cda1a4d74b0519dd7d589c843b60949719f311e2d95e0443519e10747",
    ),
    (
        "planet.symmetry-scope-applicability",
        "authority",
        "scientific",
        "blocked",
        "1ab66defe77dd4532c3a306f905fdfb09d867427db48b1d0ce4ba0e1dc625298",
    ),
    (
        "planet.theory-profile-protocol",
        "authority",
        "scientific",
        "active",
        "dbb1f8d7f2db02b3ec3aa8d466d5af836c64a6100f53904740c6bffa9db452b0",
    ),
    (
        "units.certified-formula-projection",
        "authority",
        "scientific",
        "active",
        "7ee2a9a60f102d0ea34bc5700d2070b079390c050d10c2d685e1525a0f091005",
    ),
    (
        "units.si-execution-table",
        "authority",
        "scientific",
        "blocked",
        "2eca0ae6da0ad4483e3038e5aa6bbf1cd3d158bf3ca49df78aab7224b8927d5e",
    ),
    (
        "units.si-representation-policy",
        "authority",
        "scientific",
        "blocked",
        "db7449a52dbf7f56b598828fe3888d0505e833c170c89de99a6b390a8057698b",
    ),
    (
        "units.wide-integer-arithmetic",
        "authority",
        "scientific",
        "blocked",
        "3bfb94aa0c3f507ad8ba9762e91ca9009284becc9d9752d7b7b896cb9a26ca7d",
    ),
)
EXPECTED_COUNTS = (15, 11, 2, 28)

# This checker derives its own lookup from the independently ordered profile.
# Only these four reviewed consumers may name the separately enrolled protocol
# pair; every other active pair must declare no semantic dependency.
EXPECTED_ENROLLED_UPSTREAM_HELPERS: dict[str, tuple[str, ...]] = {
    "planet.charged-profile-admission": ("planet.theory-profile-protocol",),
    "planet.confining-profile-admission": ("planet.theory-profile-protocol",),
    "planet.neutral-bound-profile-admission": ("planet.theory-profile-protocol",),
    "planet.primitive-profile-admission": ("planet.theory-profile-protocol",),
}
EXPECTED_PROFILES_BY_ID = {
    identifier: (kind, domain, status)
    for identifier, kind, domain, status, _fingerprint in EXPECTED_PROFILE
}

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
    *,
    verify_file: bool = True,
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
    if not verify_file:
        return repository / pathlib.PurePosixPath(relative)
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
    *,
    verify_files: bool,
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
            verify_file=verify_files,
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
    *,
    verify_files: bool = True,
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
        verify_file=verify_files,
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
            _inspect_diagnostic(
                entry,
                context,
                repository,
                verify_files=verify_files,
            )
        else:
            if entry.get("domain") != expected_domain:
                raise RegistryWatchdogFailure(
                    f"{context}.domain changed from pinned {expected_domain}"
                )
            _word(entry, "claim", context)
            if expected_status == "active":
                active_count += 1
                _inspect_active(
                    entry,
                    context,
                    expected_id,
                    repository,
                    verify_files=verify_files,
                )
            else:
                blocked_count += 1
                _inspect_blocked(
                    entry,
                    context,
                    repository,
                    verify_files=verify_files,
                )
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
    *,
    verify_files: bool,
) -> None:
    _schema(entry, AUTHORITY_BASE | ACTIVE_KEYS, context)
    producer_name = _word(entry, "producer_path", context)
    checker_name = _word(entry, "checker_path", context)
    producer_file = _open_repository_file(
        producer_name,
        f"{context}.producer_path",
        repository,
        verify_file=verify_files,
    )
    checker_file = _open_repository_file(
        checker_name,
        f"{context}.checker_path",
        repository,
        verify_file=verify_files,
    )
    if producer_file.samefile(checker_file):
        raise RegistryWatchdogFailure(f"{context} aliases one file for both sides")
    if _word(entry, "producer_implementation", context) == _word(
        entry, "checker_implementation", context
    ):
        raise RegistryWatchdogFailure(
            f"{context} repeats one implementation identity"
        )
    helpers = _word_vector(entry, "shared_semantic_helpers", context)
    if helpers != EXPECTED_ENROLLED_UPSTREAM_HELPERS.get(mechanism_id, ()):
        raise RegistryWatchdogFailure(
            f"{context} has an unreviewed semantic-helper enrollment"
        )
    for helper in helpers:
        if EXPECTED_PROFILES_BY_ID.get(helper) != ("authority", "scientific", "active"):
            raise RegistryWatchdogFailure(
                f"{context} names a helper outside the active authority profile"
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
        verify_files=verify_files,
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
    *,
    verify_files: bool,
) -> None:
    _schema(entry, AUTHORITY_BASE | BLOCKED_KEYS, context)
    producer_name = _word(entry, "producer_path", context)
    refusal_name = _word(entry, "refusal_path", context)
    _open_repository_file(
        producer_name,
        f"{context}.producer_path",
        repository,
        verify_file=verify_files,
    )
    _open_repository_file(
        refusal_name,
        f"{context}.refusal_path",
        repository,
        verify_file=verify_files,
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
        verify_files=verify_files,
    )


def _inspect_diagnostic(
    entry: dict[str, Any],
    context: str,
    repository: pathlib.Path,
    *,
    verify_files: bool,
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
        verify_file=verify_files,
    )
    checker = _open_repository_file(
        checker_name,
        f"{context}.diagnostic_checker_path",
        repository,
        verify_file=verify_files,
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
        verify_files=verify_files,
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

    root_refusal_schema_missing = copy.deepcopy(document)
    root_refusal_schema_missing["mechanism"][
        positions["planet.stellar-species-floor-coordinate-projection"]
    ]["receipt_schema"] = (
        "civsim.planet.stellar-birth-repository-physical-root-receipt.v5"
    )

    canary = copy.deepcopy(document)
    canary["mechanism"][positions["units.certified-formula-projection"]][
        "canaries"
    ][-1] = "replacement canary"

    root_refusal_canary = copy.deepcopy(document)
    root_refusal_canaries = root_refusal_canary["mechanism"][
        positions["planet.stellar-species-floor-coordinate-projection"]
    ]["canaries"]
    root_refusal_canaries[
        root_refusal_canaries.index("refusal stage substitution")
    ] = "replacement refusal-stage check"

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

    root_capability_requirement = copy.deepcopy(document)
    root_capability_requirements = root_capability_requirement["mechanism"][
        positions["planet.species-state-support"]
    ]["open_cross_checker_requirements"]
    root_capability_matches = [
        number
        for number, value in enumerate(root_capability_requirements)
        if "claim-specific independent capability" in value
        and "non-root admission" in value
    ]
    if len(root_capability_matches) != 1:
        raise AssertionError(
            "species-state support must carry exactly one non-root capability requirement"
        )
    root_capability_position = root_capability_matches[0]
    root_capability_requirements[root_capability_position] = (
        "a replacement downstream root-capability check"
    )

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

    live_root_missing = copy.deepcopy(document)
    live_root_missing["mechanism"][
        positions["planet.species-derivation-frontier"]
    ]["semantic_closure"].remove(
        "crates/planet/src/canonical/stellar_birth_species/physical_registry/repository_roots/mod.rs"
    )

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
        ("root refusal receipt schema omission", root_refusal_schema_missing),
        ("canary substitution", canary),
        ("root refusal canary substitution", root_refusal_canary),
        ("owner boundary substitution", owner),
        ("activation guard substitution", guard),
        ("activation requirement substitution", requirement),
        ("species root-capability requirement substitution", root_capability_requirement),
        ("diagnostic observation substitution", observation),
        ("shared material substitution", material),
        ("semantic closure adapter omission", adapter_missing),
        ("live species frontier root omission", live_root_missing),
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
        # The baseline above verifies every repository path. Complete profile
        # fingerprints still pin every corruption. Rewalk only the corruption
        # whose purpose is to exercise missing-file refusal.
        try:
            inspect_inventory(
                corrupt,
                verify_files=label == "missing referenced path",
            )
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
