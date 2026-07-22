#!/usr/bin/env python3
"""Validate, list, hash, and run the declarative repository gate inventory.

The inventory stores commands as argument arrays. This runner never evaluates a
command through a shell and never joins arguments into an executable string.
"""

from __future__ import annotations

import argparse
import copy
import dataclasses
import enum
import glob
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_MANIFEST = ROOT / "scripts" / "gates.toml"
SCHEMA_VERSION = 1
ID_RE = re.compile(r"^[a-z0-9]+(?:[._-][a-z0-9]+)*$")
PHASES = {"pre", "provenance", "post"}
POLICY_DETECTION_MARKER = "civsim.gate-runner.policy-detection.v1"
LEAF_POLICY_DETECTION_MARKER = "civsim.gate-leaf.policy-detection.v1"
CACHE_POLICIES = {"content-hash", "never"}
SHELL_PROGRAMS = {"bash", "sh", "zsh", "cmd", "cmd.exe", "powershell", "pwsh"}
SHELL_EVAL_FLAGS = {"-c", "-lc", "/c", "-command"}
class InventoryError(ValueError):
    """The declarative inventory is absent, malformed, or ambiguous."""


@dataclasses.dataclass(frozen=True)
class Gate:
    gate_id: str
    order: int
    description: str
    tiers: tuple[str, ...]
    phase: str
    command: tuple[str, ...]
    self_test: tuple[str, ...] | None
    no_self_test_reason: str | None
    timeout_seconds: int
    cache: str
    no_cache_reason: str | None
    inputs: tuple[str, ...]
    path_triggers: tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class Inventory:
    source: Path
    tiers: tuple[str, ...]
    gates: tuple[Gate, ...]
    remaining_non_inventory: tuple[str, ...]

    def select(
        self,
        *,
        tier: str | None = None,
        gate_ids: Sequence[str] = (),
        phase: str | None = None,
    ) -> tuple[Gate, ...]:
        if tier is not None and tier not in self.tiers:
            raise InventoryError(
                f"unknown tier {tier!r}; expected one of {', '.join(self.tiers)}"
            )
        requested = set(gate_ids)
        known = {gate.gate_id for gate in self.gates}
        missing = sorted(requested - known)
        if missing:
            raise InventoryError(f"unknown gate id(s): {', '.join(missing)}")
        selected = []
        for gate in self.gates:
            if tier is not None and tier not in gate.tiers:
                continue
            if requested and gate.gate_id not in requested:
                continue
            if phase is not None and gate.phase != phase:
                continue
            selected.append(gate)
        return tuple(selected)


def _require_string(mapping: Mapping[str, Any], key: str, label: str) -> str:
    value = mapping.get(key)
    if not isinstance(value, str) or not value.strip():
        raise InventoryError(f"{label} requires a nonempty string field {key!r}")
    return value


def _require_string_list(
    mapping: Mapping[str, Any], key: str, label: str
) -> tuple[str, ...]:
    value = mapping.get(key)
    if not isinstance(value, list) or not value:
        raise InventoryError(f"{label} requires a nonempty string array {key!r}")
    if any(not isinstance(item, str) or not item for item in value):
        raise InventoryError(f"{label}.{key} must contain only nonempty strings")
    return tuple(value)


def _optional_command(
    mapping: Mapping[str, Any], key: str, label: str
) -> tuple[str, ...] | None:
    if key not in mapping:
        return None
    value = mapping[key]
    if not isinstance(value, list) or not value:
        raise InventoryError(
            f"{label}.{key} must be a nonempty argument array, never a shell string"
        )
    if any(not isinstance(item, str) or not item for item in value):
        raise InventoryError(f"{label}.{key} must contain only nonempty strings")
    return tuple(value)


def _validate_array_command(command: tuple[str, ...], label: str) -> None:
    program = Path(command[0]).name.lower()
    if program in SHELL_PROGRAMS and any(
        argument.lower() in SHELL_EVAL_FLAGS for argument in command[1:]
    ):
        raise InventoryError(
            f"{label} uses a shell evaluation flag; point at a script and pass arguments as an array"
        )


def parse_inventory(data: Mapping[str, Any], source: Path) -> Inventory:
    meta = data.get("inventory")
    if not isinstance(meta, dict):
        raise InventoryError("inventory metadata table is missing")
    if meta.get("schema") != SCHEMA_VERSION:
        raise InventoryError(
            f"inventory.schema must equal {SCHEMA_VERSION}, got {meta.get('schema')!r}"
        )
    tiers = _require_string_list(meta, "tiers", "inventory")
    if len(set(tiers)) != len(tiers):
        raise InventoryError("inventory.tiers contains a duplicate")
    remaining_raw = meta.get("remaining_non_inventory", [])
    if not isinstance(remaining_raw, list) or any(
        not isinstance(item, str) or not item for item in remaining_raw
    ):
        raise InventoryError("inventory.remaining_non_inventory must be a string array")

    raw_gates = data.get("gate")
    if not isinstance(raw_gates, list) or not raw_gates:
        raise InventoryError("inventory requires at least one [[gate]] entry")

    gates: list[Gate] = []
    seen_ids: set[str] = set()
    previous_order = -1
    for index, raw in enumerate(raw_gates, start=1):
        label = f"gate entry {index}"
        if not isinstance(raw, dict):
            raise InventoryError(f"{label} must be a table")
        gate_id = _require_string(raw, "id", label)
        label = f"gate {gate_id!r}"
        if not ID_RE.fullmatch(gate_id):
            raise InventoryError(f"{label} has an invalid stable id")
        if gate_id in seen_ids:
            raise InventoryError(f"duplicate gate id {gate_id!r}")
        seen_ids.add(gate_id)

        order = raw.get("order")
        if not isinstance(order, int) or isinstance(order, bool) or order < 0:
            raise InventoryError(f"{label}.order must be a nonnegative integer")
        if order <= previous_order:
            raise InventoryError(
                f"gate order drift: {gate_id!r} has order {order} after {previous_order}"
            )
        previous_order = order

        description = _require_string(raw, "description", label)
        gate_tiers = _require_string_list(raw, "tiers", label)
        unknown_tiers = sorted(set(gate_tiers) - set(tiers))
        if unknown_tiers:
            raise InventoryError(
                f"{label} names unknown tier(s): {', '.join(unknown_tiers)}"
            )
        if len(set(gate_tiers)) != len(gate_tiers):
            raise InventoryError(f"{label}.tiers contains a duplicate")

        phase = _require_string(raw, "phase", label)
        if phase not in PHASES:
            raise InventoryError(
                f"{label}.phase must be one of {', '.join(sorted(PHASES))}"
            )
        command = _optional_command(raw, "command", label)
        if command is None:
            raise InventoryError(
                f"{label} is missing command; every command must be an argument array"
            )
        _validate_array_command(command, f"{label}.command")

        self_test = _optional_command(raw, "self_test", label)
        no_self_test_reason = raw.get("no_self_test_reason")
        if no_self_test_reason is not None and (
            not isinstance(no_self_test_reason, str) or not no_self_test_reason.strip()
        ):
            raise InventoryError(f"{label}.no_self_test_reason must be nonempty")
        if (self_test is None) == (no_self_test_reason is None):
            raise InventoryError(
                f"{label} requires exactly one of self_test or no_self_test_reason"
            )
        if self_test is not None:
            _validate_array_command(self_test, f"{label}.self_test")

        timeout = raw.get("timeout_seconds")
        if (
            not isinstance(timeout, int)
            or isinstance(timeout, bool)
            or timeout <= 0
            or timeout > 86_400
        ):
            raise InventoryError(
                f"{label}.timeout_seconds must be an integer from 1 through 86400"
            )

        cache = _require_string(raw, "cache", label)
        if cache not in CACHE_POLICIES:
            raise InventoryError(
                f"{label}.cache must be one of {', '.join(sorted(CACHE_POLICIES))}"
            )
        no_cache_reason = raw.get("no_cache_reason")
        if no_cache_reason is not None and (
            not isinstance(no_cache_reason, str) or not no_cache_reason.strip()
        ):
            raise InventoryError(f"{label}.no_cache_reason must be nonempty")
        if cache == "never" and no_cache_reason is None:
            raise InventoryError(f"{label} uses cache=never without no_cache_reason")
        if cache == "content-hash" and no_cache_reason is not None:
            raise InventoryError(
                f"{label} has no_cache_reason but uses the content-hash policy"
            )

        inputs = _require_string_list(raw, "inputs", label)
        triggers = _require_string_list(raw, "path_triggers", label)
        for field_name, patterns in (("inputs", inputs), ("path_triggers", triggers)):
            for pattern in patterns:
                path = Path(pattern)
                if path.is_absolute() or ".." in path.parts:
                    raise InventoryError(
                        f"{label}.{field_name} contains an unsafe path {pattern!r}"
                    )

        gates.append(
            Gate(
                gate_id=gate_id,
                order=order,
                description=description,
                tiers=gate_tiers,
                phase=phase,
                command=command,
                self_test=self_test,
                no_self_test_reason=no_self_test_reason,
                timeout_seconds=timeout,
                cache=cache,
                no_cache_reason=no_cache_reason,
                inputs=inputs,
                path_triggers=triggers,
            )
        )

    return Inventory(
        source=source,
        tiers=tiers,
        gates=tuple(gates),
        remaining_non_inventory=tuple(remaining_raw),
    )


def load_inventory(path: Path = DEFAULT_MANIFEST) -> Inventory:
    try:
        with path.open("rb") as handle:
            data = tomllib.load(handle)
    except FileNotFoundError as error:
        raise InventoryError(f"gate inventory is missing at {path}") from error
    except tomllib.TOMLDecodeError as error:
        raise InventoryError(f"gate inventory TOML is invalid: {error}") from error
    return parse_inventory(data, path)


def _expand_command(command: Sequence[str]) -> list[str]:
    return [sys.executable if argument == "{python}" else argument for argument in command]


def _is_reparse_point(path: Path) -> bool:
    try:
        attributes = getattr(path.lstat(), "st_file_attributes", 0)
    except OSError as error:
        raise InventoryError(f"declared gate input cannot be inspected: {path}") from error
    marker = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    return bool(marker and attributes & marker)


def _checked_input_path(path: Path, root: Path) -> Path:
    if path.is_symlink() or _is_reparse_point(path):
        raise InventoryError(f"declared gate input is a link or reparse point: {path}")
    try:
        resolved = path.resolve(strict=True)
        resolved.relative_to(root.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise InventoryError(f"declared gate input escapes the repository: {path}") from error
    return resolved


def _declared_files(root: Path, patterns: Iterable[str]) -> tuple[Path, ...]:
    root = root.resolve(strict=True)
    files: set[Path] = set()
    for pattern in patterns:
        pattern_path = Path(pattern)
        if pattern_path.is_absolute() or ".." in pattern_path.parts:
            raise InventoryError(f"declared gate input pattern escapes the repository: {pattern}")
        raw_matches = [
            raw_match
            for raw_match in glob.glob(str(root / pattern), recursive=True)
            if os.path.lexists(raw_match)
        ]
        if not raw_matches:
            raise InventoryError(f"declared gate input matched no path: {pattern}")
        pattern_files: set[Path] = set()
        for raw_match in raw_matches:
            match = Path(raw_match)
            if match.is_symlink() or _is_reparse_point(match):
                _checked_input_path(match, root)
            elif match.is_file():
                pattern_files.add(_checked_input_path(match, root))
            elif match.is_dir():
                _checked_input_path(match, root)
                for child in match.rglob("*"):
                    if child.is_symlink() or _is_reparse_point(child):
                        _checked_input_path(child, root)
                    elif child.is_file():
                        pattern_files.add(_checked_input_path(child, root))
        if not pattern_files:
            raise InventoryError(f"declared gate input matched no regular file: {pattern}")
        files.update(pattern_files)
    return tuple(sorted(files, key=lambda path: path.relative_to(root).as_posix()))


def input_hash(
    gate: Gate,
    root: Path = ROOT,
    *,
    manifest_path: Path | None = None,
) -> tuple[str, int]:
    root = root.resolve(strict=True)
    manifest = manifest_path or root / "scripts" / "gates.toml"
    try:
        manifest_relative = manifest.resolve(strict=True).relative_to(root).as_posix()
    except (OSError, ValueError) as error:
        raise InventoryError("gate inventory must be a regular file inside the repository") from error
    patterns = tuple(gate.inputs) + (manifest_relative, "scripts/gate_runner.py")
    files = _declared_files(root, patterns)
    digest = hashlib.sha256()
    for path in files:
        relative = path.relative_to(root).as_posix().encode("utf-8")
        digest.update(relative)
        digest.update(b"\0")
        digest.update(b"file\0")
        content = hashlib.sha256()
        try:
            digest.update(str(stat.S_IMODE(path.stat().st_mode)).encode("ascii"))
            digest.update(b"\0")
            with path.open("rb") as handle:
                for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                    content.update(chunk)
        except OSError as error:
            raise InventoryError(f"declared gate input cannot be read: {path}") from error
        digest.update(content.digest())
        digest.update(b"\0")
    return digest.hexdigest(), len(files)


def _print_process_output(output: subprocess.CompletedProcess[str]) -> None:
    if output.stdout:
        print(output.stdout.rstrip())
    if output.stderr:
        print(output.stderr.rstrip(), file=sys.stderr)


class GateOutcome(enum.Enum):
    Passed = "passed"
    PolicyDetection = "policy_detection"
    OperationalFailure = "operational_failure"


def execute_gate(
    gate: Gate,
    command: Sequence[str],
    *,
    dry_run: bool,
    manifest_path: Path | None = None,
) -> GateOutcome:
    argv = _expand_command(command)
    if manifest_path is None:
        digest, file_count = input_hash(gate, ROOT)
    else:
        digest, file_count = input_hash(gate, ROOT, manifest_path=manifest_path)
    if dry_run:
        print(
            json.dumps(
                {
                    "id": gate.gate_id,
                    "input_hash": digest,
                    "input_files": file_count,
                    "command": argv,
                },
                sort_keys=True,
            )
        )
        return GateOutcome.Passed

    started = time.monotonic()
    environment = os.environ.copy()
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    try:
        raw_output = subprocess.run(
            argv,
            cwd=ROOT,
            env=environment,
            capture_output=True,
            timeout=gate.timeout_seconds,
            check=False,
        )
    except FileNotFoundError as error:
        print(
            f"[FAIL] {gate.gate_id}: command program is unavailable: {error}",
            file=sys.stderr,
        )
        return GateOutcome.OperationalFailure
    except subprocess.TimeoutExpired as error:
        print(
            f"[FAIL] {gate.gate_id}: timed out after {gate.timeout_seconds} seconds",
            file=sys.stderr,
        )
        if error.stdout:
            print(str(error.stdout).rstrip())
        if error.stderr:
            print(str(error.stderr).rstrip(), file=sys.stderr)
        return GateOutcome.OperationalFailure

    try:
        output = subprocess.CompletedProcess(
            raw_output.args,
            raw_output.returncode,
            raw_output.stdout.decode("utf-8", errors="strict"),
            raw_output.stderr.decode("utf-8", errors="strict"),
        )
    except UnicodeDecodeError as error:
        print(
            f"[FAIL] {gate.gate_id}: command output is not valid UTF-8: {error}",
            file=sys.stderr,
        )
        return GateOutcome.OperationalFailure

    try:
        if manifest_path is None:
            final_digest, final_file_count = input_hash(gate, ROOT)
        else:
            final_digest, final_file_count = input_hash(
                gate, ROOT, manifest_path=manifest_path
            )
    except InventoryError as error:
        print(
            f"[FAIL] {gate.gate_id}: declared inputs changed during execution: {error}",
            file=sys.stderr,
        )
        return GateOutcome.OperationalFailure
    if (final_digest, final_file_count) != (digest, file_count):
        print(
            f"[FAIL] {gate.gate_id}: declared inputs changed during execution",
            file=sys.stderr,
        )
        return GateOutcome.OperationalFailure

    elapsed = time.monotonic() - started
    _print_process_output(output)
    if output.returncode != 0:
        leaf_policy_detection = (
            output.returncode == 1
            and any(
                line.strip() == LEAF_POLICY_DETECTION_MARKER
                for line in (*output.stdout.splitlines(), *output.stderr.splitlines())
            )
        )
        print(
            f"[FAIL] {gate.gate_id}: exit {output.returncode}; "
            f"input {digest[:12]} over {file_count} file(s)",
            file=sys.stderr,
        )
        if leaf_policy_detection:
            return GateOutcome.PolicyDetection
        return GateOutcome.OperationalFailure
    print(
        f"[PASS] {gate.gate_id} ({elapsed:.2f}s, {digest[:12]}, {file_count} input file(s))"
    )
    return GateOutcome.Passed


def _select_from_args(inventory: Inventory, args: argparse.Namespace) -> tuple[Gate, ...]:
    phase = getattr(args, "phase", None)
    if phase is not None and phase not in PHASES:
        raise InventoryError(f"unknown phase {phase!r}")
    return inventory.select(
        tier=getattr(args, "tier", None),
        gate_ids=getattr(args, "gate_id", ()) or (),
        phase=phase,
    )


def command_list(inventory: Inventory, args: argparse.Namespace) -> int:
    selected = _select_from_args(inventory, args)
    if args.json:
        records = []
        for gate in selected:
            record: dict[str, Any] = {
                "id": gate.gate_id,
                "order": gate.order,
                "tiers": gate.tiers,
                "phase": gate.phase,
                "timeout_seconds": gate.timeout_seconds,
                "cache": gate.cache,
                "command": _expand_command(gate.command),
                "inputs": gate.inputs,
                "path_triggers": gate.path_triggers,
                "self_test": _expand_command(gate.self_test)
                if gate.self_test is not None
                else None,
                "no_self_test_reason": gate.no_self_test_reason,
            }
            if args.hashes:
                digest, count = input_hash(gate, manifest_path=inventory.source)
                record["input_hash"] = digest
                record["input_files"] = count
            records.append(record)
        print(json.dumps(records, indent=2, sort_keys=True))
        return 0

    for gate in selected:
        if args.ids_only:
            print(gate.gate_id)
            continue
        suffix = ""
        if args.hashes:
            digest, count = input_hash(gate, manifest_path=inventory.source)
            suffix = f"  {digest}  {count} file(s)"
        print(f"{gate.order:04d}  {gate.phase:10s}  {gate.gate_id}{suffix}")
    return 0


def command_run(inventory: Inventory, args: argparse.Namespace) -> int:
    selected = _select_from_args(inventory, args)
    if not selected:
        raise InventoryError("gate selection is empty")
    policy_detection = False
    operational_failure = False
    for gate in selected:
        outcome = execute_gate(
            gate,
            gate.command,
            dry_run=args.dry_run,
            manifest_path=inventory.source,
        )
        policy_detection = policy_detection or outcome is GateOutcome.PolicyDetection
        operational_failure = operational_failure or outcome is GateOutcome.OperationalFailure
        if outcome is not GateOutcome.Passed and args.fail_fast:
            break
    if operational_failure:
        return 2
    if not policy_detection:
        return 0
    print(POLICY_DETECTION_MARKER, file=sys.stderr)
    return 1


def command_self_tests(inventory: Inventory, args: argparse.Namespace) -> int:
    selected = _select_from_args(inventory, args)
    if not selected:
        raise InventoryError("gate selection is empty")
    tested = 0
    ok = True
    for gate in selected:
        if gate.self_test is None:
            continue
        tested += 1
        outcome = execute_gate(
            gate,
            gate.self_test,
            dry_run=args.dry_run,
            manifest_path=inventory.source,
        )
        passed = outcome is GateOutcome.Passed
        ok = passed and ok
        if not passed and args.fail_fast:
            break
    if tested == 0:
        raise InventoryError("selected gates expose no self-test commands")
    return 0 if ok else 1


def _expect_invalid(data: Mapping[str, Any], needle: str) -> None:
    try:
        parse_inventory(data, Path("synthetic-gates.toml"))
    except InventoryError as error:
        if needle not in str(error):
            raise AssertionError(
                f"expected validation error containing {needle!r}, got {error!s}"
            ) from error
        return
    raise AssertionError(f"synthetic invalid inventory passed: expected {needle!r}")


def internal_self_test(inventory: Inventory) -> int:
    gate = {
        "id": "test.one",
        "order": 10,
        "description": "synthetic gate",
        "tiers": ["canonical"],
        "phase": "provenance",
        "command": ["{python}", "scripts/example.py"],
        "self_test": ["{python}", "scripts/example.py", "--self-test"],
        "timeout_seconds": 10,
        "cache": "content-hash",
        "inputs": ["scripts/example.py"],
        "path_triggers": ["scripts/example.py"],
    }
    base = {
        "inventory": {"schema": 1, "tiers": ["canonical"]},
        "gate": [gate],
    }
    parse_inventory(copy.deepcopy(base), Path("synthetic-gates.toml"))

    missing = copy.deepcopy(base)
    del missing["gate"][0]["command"]
    _expect_invalid(missing, "missing command")

    duplicate = copy.deepcopy(base)
    second = copy.deepcopy(gate)
    second["order"] = 20
    duplicate["gate"].append(second)
    _expect_invalid(duplicate, "duplicate gate id")

    order_drift = copy.deepcopy(base)
    second = copy.deepcopy(gate)
    second["id"] = "test.two"
    second["order"] = 5
    order_drift["gate"].append(second)
    _expect_invalid(order_drift, "gate order drift")

    shell_string = copy.deepcopy(base)
    shell_string["gate"][0]["command"] = "python scripts/example.py"
    _expect_invalid(shell_string, "argument array")

    shell_eval = copy.deepcopy(base)
    shell_eval["gate"][0]["command"] = ["bash", "-lc", "python script.py"]
    _expect_invalid(shell_eval, "shell evaluation flag")

    boundary = next(
        gate for gate in inventory.gates if gate.gate_id == "canonical.planet-boundary"
    )
    required_boundary_inputs = {
        "scripts/planet_boundary_gate.py",
        "Cargo.toml",
        "Cargo.lock",
        "crates",
        "parked/Cargo.toml",
        "parked/Cargo.lock",
        "parked/crates",
    }
    if not required_boundary_inputs.issubset(boundary.inputs):
        raise AssertionError("planet boundary input declaration is incomplete")
    digest, count = input_hash(boundary)
    if len(digest) != 64 or count == 0:
        raise AssertionError("planet boundary input hash was not produced")

    print(
        "gate runner self-test: PASS "
        "(missing, duplicate, order drift, shell string, and boundary hashing canaries fired)"
    )
    return 0


def _add_selection_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--tier")
    parser.add_argument("--id", dest="gate_id", action="append", default=[])
    parser.add_argument("--phase", choices=sorted(PHASES))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=DEFAULT_MANIFEST,
        help="gate inventory path",
    )
    subparsers = parser.add_subparsers(dest="subcommand", required=True)

    subparsers.add_parser("validate", help="validate the gate inventory")
    subparsers.add_parser("self-test", help="run gate-runner synthetic canaries")

    list_parser = subparsers.add_parser("list", help="print gates in authority order")
    _add_selection_arguments(list_parser)
    list_parser.add_argument("--ids-only", action="store_true")
    list_parser.add_argument("--hashes", action="store_true")
    list_parser.add_argument("--json", action="store_true")

    run_parser = subparsers.add_parser("run", help="run selected live gates")
    _add_selection_arguments(run_parser)
    run_parser.add_argument("--dry-run", action="store_true")
    run_parser.add_argument("--fail-fast", action="store_true")

    tests_parser = subparsers.add_parser(
        "self-tests", help="run the selected gates' declared self-tests"
    )
    _add_selection_arguments(tests_parser)
    tests_parser.add_argument("--dry-run", action="store_true")
    tests_parser.add_argument("--fail-fast", action="store_true")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    raw = list(sys.argv[1:] if argv is None else argv)
    if raw == ["--self-test"]:
        raw = ["self-test"]
    parser = build_parser()
    args = parser.parse_args(raw)
    try:
        manifest = args.manifest
        if not manifest.is_absolute():
            manifest = (ROOT / manifest).resolve()
        inventory = load_inventory(manifest)
        if args.subcommand == "validate":
            print(
                f"gate inventory valid: {len(inventory.gates)} gate(s), "
                f"{len(inventory.tiers)} tier(s)"
            )
            return 0
        if args.subcommand == "self-test":
            return internal_self_test(inventory)
        if args.subcommand == "list":
            return command_list(inventory, args)
        if args.subcommand == "run":
            return command_run(inventory, args)
        if args.subcommand == "self-tests":
            return command_self_tests(inventory, args)
        raise AssertionError(f"unhandled subcommand {args.subcommand}")
    except (InventoryError, AssertionError) as error:
        print(f"gate runner: FAIL: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
