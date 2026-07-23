#!/usr/bin/env python3
"""Validate Stone 0's singleton Cargo build wiring and cross-check its receipt."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import tomllib
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parent.parent
WATCHDOG = ROOT / "scripts" / "stone0_build_wiring_watchdog.py"
SCHEMA = "civsim.stone0.build-wiring-pair.v1"
CLAIM_ID = "governance.stone0-build-wiring"
PRODUCER = "civsim.stone0.toml-graph-producer.v1"
CHECKER = "civsim.stone0.section-scan-watchdog.v1"
ANCHOR = "crates/stone0-build"
CONSUMERS = ("crates/planet", "crates/planet-substrate")
MARKER_ENV = "CIVSIM_STONE0_GUARD_MARKER"
MARKER_TOKEN = "civsim.stone0.build-guard.v1"
FILES = (
    "Cargo.toml",
    "justfile",
    "crates/stone0-build/Cargo.toml",
    "crates/stone0-build/build.rs",
    "crates/stone0-build/src/lib.rs",
    "crates/planet/Cargo.toml",
    "crates/planet/build.rs",
    "crates/planet-substrate/Cargo.toml",
    "crates/planet-substrate/build.rs",
)


class WiringError(ValueError):
    """The singleton build topology or pair receipt failed closed."""


def _regular(root: pathlib.Path, relative: str) -> pathlib.Path:
    path = root / pathlib.PurePosixPath(relative)
    try:
        resolved = path.resolve(strict=True)
        resolved.relative_to(root.resolve(strict=True))
    except (FileNotFoundError, OSError, ValueError) as error:
        raise WiringError(f"missing or external wiring file: {relative}") from error
    if path.is_symlink() or not resolved.is_file():
        raise WiringError(f"wiring path is not a plain file: {relative}")
    return resolved


def _manifest(root: pathlib.Path, relative: str) -> dict[str, Any]:
    raw = _regular(root, relative).read_bytes()
    try:
        parsed = tomllib.load(io.BytesIO(raw))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as error:
        raise WiringError(f"invalid Cargo manifest: {relative}") from error
    if not isinstance(parsed, dict):
        raise WiringError(f"Cargo manifest is not a table: {relative}")
    return parsed


def _validate_anchor_sources(root: pathlib.Path) -> None:
    build = _regular(root, f"{ANCHOR}/build.rs").read_text(encoding="utf-8")
    ordered = (
        "std::fs::remove_file(&marker)",
        "civsim_stone0::run(civsim_stone0::Mode::Local)",
        "if code != 0",
        "std::fs::write(&marker, MARKER_SOURCE)",
        "cargo:rustc-env=CIVSIM_STONE0_GUARD_MARKER",
    )
    positions: list[int] = []
    for needle in ordered:
        if build.count(needle) != 1:
            raise WiringError(f"anchor build script must contain one {needle!r}")
        positions.append(build.index(needle))
    if positions != sorted(positions):
        raise WiringError("anchor marker may be emitted only after a clean Stone 0 run")
    if MARKER_TOKEN not in build:
        raise WiringError("anchor build script changed its linkage token")

    library = _regular(root, f"{ANCHOR}/src/lib.rs").read_text(encoding="utf-8")
    for needle in (
        f'include!(env!("{MARKER_ENV}"))',
        MARKER_TOKEN,
        "pub fn assert_guard_linked()",
        "generated::token()",
    ):
        if library.count(needle) != 1:
            raise WiringError(f"anchor library must contain one {needle!r}")

    for consumer in CONSUMERS:
        source = _regular(root, f"{consumer}/build.rs").read_text(encoding="utf-8")
        if source.count("civsim_stone0_build::assert_guard_linked();") != 1:
            raise WiringError(f"{consumer} must call one shared guard sentinel")
        for forbidden in ("civsim_stone0::", "Mode::", "emit_cargo_rerun_inputs"):
            if forbidden in source:
                raise WiringError(f"{consumer} retains a duplicate Stone 0 owner")


def validate(root: pathlib.Path) -> None:
    workspace = _manifest(root, "Cargo.toml").get("workspace")
    if not isinstance(workspace, dict):
        raise WiringError("root workspace table is absent")
    members = workspace.get("members")
    defaults = workspace.get("default-members")
    if not isinstance(members, list) or members.count(ANCHOR) != 1:
        raise WiringError("workspace must contain exactly one Stone 0 build anchor")
    if not isinstance(defaults, list) or ANCHOR in defaults:
        raise WiringError("Stone 0 build anchor must not be a default member")

    anchor = _manifest(root, f"{ANCHOR}/Cargo.toml")
    package = anchor.get("package")
    if not isinstance(package, dict) or package.get("name") != "civsim-stone0-build":
        raise WiringError("anchor package identity changed")
    if package.get("build") != "build.rs":
        raise WiringError("anchor must explicitly require build.rs")
    if "dependencies" in anchor:
        raise WiringError("anchor may not have a runtime dependency table")
    if anchor.get("build-dependencies") != {
        "civsim-stone0": {"path": "../stone0"}
    }:
        raise WiringError("anchor must have exactly one Stone 0 build dependency")

    for consumer in CONSUMERS:
        manifest = _manifest(root, f"{consumer}/Cargo.toml")
        consumer_package = manifest.get("package")
        if not isinstance(consumer_package, dict) or consumer_package.get("build") != "build.rs":
            raise WiringError(f"{consumer} must explicitly require build.rs")
        if manifest.get("build-dependencies") != {
            "civsim-stone0-build": {"path": "../stone0-build"}
        }:
            raise WiringError(f"{consumer} must depend only on the shared build guard")

    justfile = _regular(root, "justfile").read_text(encoding="utf-8")
    assignment = next(
        (line for line in justfile.splitlines() if line.startswith("canonical_packages :=")),
        None,
    )
    if assignment is None or "civsim-stone0-build" in assignment:
        raise WiringError("canonical package aggregate must not select the build anchor")
    _validate_anchor_sources(root)


def receipt(root: pathlib.Path) -> bytes:
    validate(root)
    observed = [
        {
            "path": relative,
            "sha256": hashlib.sha256(_regular(root, relative).read_bytes()).hexdigest(),
        }
        for relative in sorted(FILES)
    ]
    payload = {
        "anchor": ANCHOR,
        "claim_id": CLAIM_ID,
        "consumers": list(CONSUMERS),
        "files": observed,
        "marker_environment": MARKER_ENV,
        "marker_token": MARKER_TOKEN,
        "producer_implementation": PRODUCER,
        "schema": SCHEMA,
        "watchdog_implementation": CHECKER,
    }
    return json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("ascii")


def _fixture() -> tuple[tempfile.TemporaryDirectory[str], pathlib.Path]:
    temporary = tempfile.TemporaryDirectory(prefix="stone0-wiring-producer-")
    root = pathlib.Path(temporary.name)
    for relative in FILES:
        destination = root / pathlib.PurePosixPath(relative)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / pathlib.PurePosixPath(relative), destination)
    return temporary, root


def self_test() -> None:
    temporary, root = _fixture()
    try:
        receipt(root)
        mutations = (
            ("Cargo.toml", ANCHOR, "crates/stone0-build-missing"),
            (f"{ANCHOR}/Cargo.toml", "../stone0", "../not-stone0"),
            (f"{ANCHOR}/build.rs", "if code != 0", "if false"),
            (f"{ANCHOR}/src/lib.rs", MARKER_ENV, "UNBOUND_MARKER"),
            ("crates/planet/build.rs", "assert_guard_linked", "guard_was_skipped"),
        )
        for relative, old, new in mutations:
            path = root / pathlib.PurePosixPath(relative)
            held = path.read_text(encoding="utf-8")
            path.write_text(held.replace(old, new, 1), encoding="utf-8")
            try:
                validate(root)
            except WiringError:
                pass
            else:
                raise AssertionError(f"producer wiring canary survived: {relative}")
            path.write_text(held, encoding="utf-8")

        result = subprocess.run(
            [sys.executable, str(WATCHDOG), "--self-test"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        if result.returncode != 0:
            raise AssertionError(result.stdout + result.stderr)
    finally:
        temporary.cleanup()
    print("Stone 0 build wiring pair self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    options = parser.parse_args()
    try:
        if options.self_test:
            self_test()
            return 0
        producer = receipt(ROOT)
        result = subprocess.run(
            [sys.executable, str(WATCHDOG), "--receipt"],
            cwd=ROOT,
            capture_output=True,
            timeout=60,
            check=False,
        )
        if result.returncode != 0:
            raise WiringError(result.stderr.decode("utf-8", errors="replace").strip())
        checker = result.stdout.rstrip(b"\r\n")
        if producer != checker:
            raise WiringError("producer and watchdog wiring receipts differ")
        print("Stone 0 build wiring pair: PASS")
        print(producer.decode("ascii"))
    except (AssertionError, OSError, UnicodeError, WiringError) as error:
        print(f"Stone 0 build wiring pair: FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
