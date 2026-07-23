#!/usr/bin/env python3
"""Validate, list, hash, and run the declarative repository gate inventory.

The inventory stores commands as argument arrays. This runner never evaluates a
command through a shell and never joins arguments into an executable string.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import copy
import dataclasses
import enum
import glob
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_MANIFEST = ROOT / "scripts" / "gates.toml"
SCHEMA_VERSION = 1
ID_RE = re.compile(r"^[a-z0-9]+(?:[._-][a-z0-9]+)*$")
PHASE_SEQUENCE = ("pre", "provenance", "post")
PHASES = set(PHASE_SEQUENCE)
POLICY_DETECTION_MARKER = "civsim.gate-runner.policy-detection.v1"
LEAF_POLICY_DETECTION_MARKER = "civsim.gate-leaf.policy-detection.v1"
CACHE_POLICIES = {"content-hash", "never"}
CACHE_RECEIPT_SCHEMA = 1
CACHE_MAX_RECEIPTS_PER_GATE = 4
CACHE_MAX_AGE_SECONDS = 30 * 24 * 60 * 60
CACHE_ENVIRONMENT_KEYS = (
    "CARGO",
    "CARGO_BUILD_RUSTC",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_HOME",
    "CARGO_NET_OFFLINE",
    "CARGO_TARGET_DIR",
    "CC",
    "CXX",
    "LANG",
    "LC_ALL",
    "LD_LIBRARY_PATH",
    "PYTHONHASHSEED",
    "PYTHONPATH",
    "PATH",
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTDOCFLAGS",
    "RUSTFLAGS",
    "RUSTUP_TOOLCHAIN",
    "TZ",
)
CACHE_DISABLE_ENVIRONMENT_KEYS = ("STONE0_OVERRIDE", "STONE0_SECRETS_PATH")
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

    for tier in tiers:
        tier_phases = [
            PHASE_SEQUENCE.index(gate.phase) for gate in gates if tier in gate.tiers
        ]
        if tier_phases != sorted(tier_phases):
            raise InventoryError(
                f"gate phase drift in tier {tier!r}; required order is "
                + " -> ".join(PHASE_SEQUENCE)
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


def _default_cache_directory(root: Path = ROOT) -> Path:
    override = os.environ.get("CIVSIM_GATE_CACHE_DIR")
    if override:
        return Path(override).expanduser()
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            cwd=root,
            capture_output=True,
            text=True,
            timeout=10,
            check=True,
        )
        common = Path(result.stdout.strip()).resolve(strict=True)
        return common / "civsim-cache" / "gates"
    except (OSError, subprocess.SubprocessError, ValueError):
        return root / ".git" / "civsim-cache" / "gates"


def _tool_fingerprint(argv: Sequence[str]) -> Mapping[str, Any] | None:
    executable = shutil.which(argv[0])
    if executable is None:
        return None
    try:
        resolved = Path(executable).resolve(strict=True)
        metadata = resolved.stat()
    except OSError:
        return None
    fingerprint: dict[str, Any] = {
        "path": resolved.as_posix(),
        "size": metadata.st_size,
        "mtime_ns": metadata.st_mtime_ns,
    }
    program = resolved.name.lower()
    version_commands: list[list[str]] = []
    if program.startswith("cargo") or Path(argv[0]).name.lower() == "cargo":
        version_commands = [
            [os.environ.get("CARGO", argv[0]), "-Vv"],
            [os.environ.get("RUSTC", "rustc"), "-Vv"],
        ]
    elif program.startswith("python") or resolved == Path(sys.executable).resolve():
        fingerprint["python"] = sys.version
    elif program in {"bash", "sh", "zsh"}:
        version_commands = [[argv[0], "--version"]]
        if len(argv) > 1 and Path(argv[1]).name == "cargo_dev.sh":
            version_commands.extend(
                [
                    [os.environ.get("CARGO", "cargo"), "-Vv"],
                    [os.environ.get("RUSTC", "rustc"), "-Vv"],
                ]
            )
    for command in version_commands:
        command_path = shutil.which(command[0])
        if command_path is None:
            return None
        try:
            command_resolved = Path(command_path).resolve(strict=True)
            command_metadata = command_resolved.stat()
            output = subprocess.run(
                [str(command_resolved), *command[1:]],
                capture_output=True,
                text=True,
                timeout=10,
                check=True,
            )
        except (OSError, subprocess.SubprocessError, UnicodeError):
            return None
        fingerprint[" ".join(command)] = {
            "path": command_resolved.as_posix(),
            "size": command_metadata.st_size,
            "mtime_ns": command_metadata.st_mtime_ns,
            "version": output.stdout.strip(),
        }
    return fingerprint


def _execution_cache_key(
    gate: Gate,
    argv: Sequence[str],
    digest: str,
    file_count: int,
) -> tuple[str, Mapping[str, Any]] | None:
    if any(os.environ.get(key) for key in CACHE_DISABLE_ENVIRONMENT_KEYS):
        return None
    tool = _tool_fingerprint(argv)
    if tool is None:
        return None
    environment = {
        key: os.environ[key]
        for key in CACHE_ENVIRONMENT_KEYS
        if key in os.environ
    }
    payload: dict[str, Any] = {
        "schema": CACHE_RECEIPT_SCHEMA,
        "gate_id": gate.gate_id,
        "command": list(argv),
        "input_hash": digest,
        "input_files": file_count,
        "platform": {
            "machine": platform.machine(),
            "os_name": os.name,
            "sys_platform": sys.platform,
        },
        "tool": tool,
        "environment": environment,
    }
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode(
        "utf-8"
    )
    return hashlib.sha256(encoded).hexdigest(), payload


def _cache_receipt_path(cache_dir: Path, gate: Gate, cache_key: str) -> Path:
    return cache_dir / gate.gate_id / f"{cache_key}.json"


def _cache_hit(
    cache_dir: Path,
    gate: Gate,
    cache_key: str,
    expected_payload: Mapping[str, Any],
) -> bool:
    path = _cache_receipt_path(cache_dir, gate, cache_key)
    try:
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 64 * 1024:
            return False
        receipt = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return False
    created = receipt.get("created_unix") if isinstance(receipt, dict) else None
    fresh = (
        isinstance(created, int)
        and 0 <= time.time() - created <= CACHE_MAX_AGE_SECONDS
    )
    return (
        isinstance(receipt, dict)
        and fresh
        and receipt.get("schema") == CACHE_RECEIPT_SCHEMA
        and receipt.get("cache_key") == cache_key
        and receipt.get("success") is True
        and receipt.get("payload") == expected_payload
    )


def _prune_gate_cache(gate_dir: Path) -> None:
    try:
        now = time.time()
        receipts = sorted(
            (
                path
                for path in gate_dir.glob("*.json")
                if path.is_file() and not path.is_symlink()
            ),
            key=lambda path: path.stat().st_mtime_ns,
            reverse=True,
        )
        for index, path in enumerate(receipts):
            stale = now - path.stat().st_mtime > CACHE_MAX_AGE_SECONDS
            if index >= CACHE_MAX_RECEIPTS_PER_GATE or stale:
                path.unlink(missing_ok=True)
    except OSError:
        return


def _write_cache_receipt(
    cache_dir: Path,
    gate: Gate,
    cache_key: str,
    payload: Mapping[str, Any],
) -> None:
    gate_dir = cache_dir / gate.gate_id
    try:
        cache_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
        if cache_dir.is_symlink():
            return
        gate_dir.mkdir(exist_ok=True, mode=0o700)
        if gate_dir.is_symlink():
            return
        receipt = {
            "schema": CACHE_RECEIPT_SCHEMA,
            "cache_key": cache_key,
            "success": True,
            "created_unix": int(time.time()),
            "payload": payload,
        }
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=".receipt-", suffix=".tmp", dir=gate_dir
        )
        temporary = Path(temporary_name)
        try:
            with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
                json.dump(receipt, handle, sort_keys=True, separators=(",", ":"))
                handle.write("\n")
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temporary, _cache_receipt_path(cache_dir, gate, cache_key))
        finally:
            temporary.unlink(missing_ok=True)
        _prune_gate_cache(gate_dir)
    except OSError:
        return


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
    cache_dir: Path | None = None,
    read_cache: bool = False,
    write_cache: bool = False,
) -> GateOutcome:
    argv = _expand_command(command)
    if manifest_path is None:
        digest, file_count = input_hash(gate, ROOT)
    else:
        digest, file_count = input_hash(gate, ROOT, manifest_path=manifest_path)
    cache_key_payload = None
    if gate.cache == "content-hash" and (read_cache or write_cache or dry_run):
        cache_key_payload = _execution_cache_key(gate, argv, digest, file_count)
    if dry_run:
        cache_key = cache_key_payload[0] if cache_key_payload is not None else None
        print(
            json.dumps(
                {
                    "id": gate.gate_id,
                    "input_hash": digest,
                    "input_files": file_count,
                    "command": argv,
                    "cache_policy": gate.cache,
                    "cache_key": cache_key,
                },
                sort_keys=True,
            )
        )
        return GateOutcome.Passed

    selected_cache_dir = cache_dir or _default_cache_directory(ROOT)
    if (
        gate.cache == "content-hash"
        and read_cache
        and cache_key_payload is not None
        and _cache_hit(
            selected_cache_dir,
            gate,
            cache_key_payload[0],
            cache_key_payload[1],
        )
    ):
        if manifest_path is None:
            final_digest, final_file_count = input_hash(gate, ROOT)
        else:
            final_digest, final_file_count = input_hash(
                gate, ROOT, manifest_path=manifest_path
            )
        if (final_digest, final_file_count) != (digest, file_count):
            print(
                f"[FAIL] {gate.gate_id}: declared inputs changed during cache lookup",
                file=sys.stderr,
            )
            return GateOutcome.OperationalFailure
        print(
            f"[CACHED] {gate.gate_id} "
            f"({digest[:12]}, {file_count} input file(s), {cache_key_payload[0][:12]})"
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
    if (
        gate.cache == "content-hash"
        and write_cache
        and cache_key_payload is not None
    ):
        _write_cache_receipt(
            selected_cache_dir,
            gate,
            cache_key_payload[0],
            cache_key_payload[1],
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
    no_cache = getattr(args, "no_cache", False)
    refresh_cache = getattr(args, "refresh_cache", False)
    jobs = max(1, getattr(args, "jobs", 1))
    if getattr(args, "fail_fast", False):
        jobs = 1
    cache_dir = _default_cache_directory(ROOT)
    outcomes: list[GateOutcome] = []
    stop = False
    for phase in PHASE_SEQUENCE:
        phase_gates = tuple(gate for gate in selected if gate.phase == phase)
        if not phase_gates:
            continue
        if jobs > 1 and len(phase_gates) > 1:
            outcomes.extend(
                _execute_gates_parallel(
                    inventory,
                    phase_gates,
                    jobs=jobs,
                    dry_run=args.dry_run,
                    no_cache=no_cache,
                    refresh_cache=refresh_cache,
                )
            )
            continue
        for gate in phase_gates:
            outcome = execute_gate(
                gate,
                gate.command,
                dry_run=args.dry_run,
                manifest_path=inventory.source,
                cache_dir=cache_dir,
                read_cache=not no_cache and not refresh_cache,
                write_cache=not no_cache,
            )
            outcomes.append(outcome)
            if outcome is not GateOutcome.Passed and args.fail_fast:
                stop = True
                break
        if stop:
            break

    for outcome in outcomes:
        policy_detection = policy_detection or outcome is GateOutcome.PolicyDetection
        operational_failure = operational_failure or outcome is GateOutcome.OperationalFailure
    if operational_failure:
        return 2
    if not policy_detection:
        return 0
    print(POLICY_DETECTION_MARKER, file=sys.stderr)
    return 1


def _parallel_gate_process(
    inventory: Inventory,
    gate: Gate,
    *,
    dry_run: bool,
    no_cache: bool,
    refresh_cache: bool,
) -> tuple[GateOutcome, str, str]:
    command = [
        sys.executable,
        str(Path(__file__).resolve()),
        "--manifest",
        str(inventory.source),
        "run",
        "--id",
        gate.gate_id,
        "--jobs",
        "1",
    ]
    if dry_run:
        command.append("--dry-run")
    elif no_cache:
        command.append("--no-cache")
    elif refresh_cache:
        command.append("--refresh-cache")
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            env=os.environ.copy(),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
            timeout=gate.timeout_seconds + 30,
            check=False,
        )
    except (OSError, subprocess.SubprocessError, UnicodeError) as error:
        return (
            GateOutcome.OperationalFailure,
            "",
            f"[FAIL] {gate.gate_id}: parallel worker failed: {error}\n",
        )
    stderr_lines = result.stderr.splitlines()
    has_marker = POLICY_DETECTION_MARKER in stderr_lines
    stderr = "\n".join(
        line for line in stderr_lines if line != POLICY_DETECTION_MARKER
    )
    if stderr:
        stderr += "\n"
    if result.returncode == 0:
        outcome = GateOutcome.Passed
    elif result.returncode == 1 and has_marker:
        outcome = GateOutcome.PolicyDetection
    else:
        outcome = GateOutcome.OperationalFailure
    return outcome, result.stdout, stderr


def _execute_gates_parallel(
    inventory: Inventory,
    gates: Sequence[Gate],
    *,
    jobs: int,
    dry_run: bool,
    no_cache: bool,
    refresh_cache: bool,
) -> list[GateOutcome]:
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=min(jobs, len(gates))
    ) as executor:
        results = list(
            executor.map(
                lambda gate: _parallel_gate_process(
                    inventory,
                    gate,
                    dry_run=dry_run,
                    no_cache=no_cache,
                    refresh_cache=refresh_cache,
                ),
                gates,
            )
        )
    outcomes: list[GateOutcome] = []
    for outcome, stdout, stderr in results:
        if stdout:
            print(stdout.rstrip())
        if stderr:
            print(stderr.rstrip(), file=sys.stderr)
        outcomes.append(outcome)
    return outcomes


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


def _positive_integer(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a positive integer") from error
    if value < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return value


def _default_jobs() -> int:
    raw = os.environ.get("CIVSIM_GATE_JOBS")
    if raw is None:
        return min(4, os.cpu_count() or 1)
    try:
        return _positive_integer(raw)
    except argparse.ArgumentTypeError:
        return 1


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
    run_parser.add_argument(
        "--jobs",
        type=_positive_integer,
        default=_default_jobs(),
        help="run independent read-only gates concurrently (default: CIVSIM_GATE_JOBS or up to 4)",
    )
    cache_group = run_parser.add_mutually_exclusive_group()
    cache_group.add_argument(
        "--no-cache",
        action="store_true",
        help="run without reading or writing successful content receipts",
    )
    cache_group.add_argument(
        "--refresh-cache",
        action="store_true",
        help="ignore prior receipts and replace them after successful execution",
    )

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
