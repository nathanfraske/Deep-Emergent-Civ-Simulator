#!/usr/bin/env python3
"""Independently scan Stone 0's singleton Cargo build topology."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import shutil
import sys
import tempfile


REPOSITORY = pathlib.Path(__file__).resolve().parent.parent
SCHEMA = "civsim.stone0.build-wiring-pair.v1"
CLAIM = "governance.stone0-build-wiring"
FIRST_IMPLEMENTATION = "civsim.stone0.toml-graph-producer.v1"
SECOND_IMPLEMENTATION = "civsim.stone0.section-scan-watchdog.v1"
ANCHOR_PATH = "crates/stone0-build"
CLIENTS = ("crates/planet", "crates/planet-substrate")
ENVIRONMENT_NAME = "CIVSIM_STONE0_GUARD_MARKER"
TOKEN = "civsim.stone0.build-guard.v1"
OBSERVED_PATHS = (
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


class WatchdogError(ValueError):
    """The independent build-wiring scan refused the topology."""


def _bytes(root: pathlib.Path, name: str) -> bytes:
    path = root.joinpath(*name.split("/"))
    try:
        resolved = path.resolve(strict=True)
        resolved.relative_to(root.resolve(strict=True))
    except (FileNotFoundError, OSError, ValueError) as error:
        raise WatchdogError(f"unavailable wiring member: {name}") from error
    if path.is_symlink() or not resolved.is_file():
        raise WatchdogError(f"linked or non-file wiring member: {name}")
    return resolved.read_bytes()


def _text(root: pathlib.Path, name: str) -> str:
    try:
        return _bytes(root, name).decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        raise WatchdogError(f"non-UTF-8 wiring member: {name}") from error


def _section(source: str, heading: str) -> str:
    match = re.search(
        rf"(?ms)^\[{re.escape(heading)}\]\s*$\n(.*?)(?=^\[|\Z)", source
    )
    if match is None:
        raise WatchdogError(f"manifest omits [{heading}]")
    return match.group(1)


def _one(pattern: str, source: str, label: str) -> re.Match[str]:
    matches = list(re.finditer(pattern, source, flags=re.MULTILINE))
    if len(matches) != 1:
        raise WatchdogError(f"{label} must occur exactly once")
    return matches[0]


def inspect(root: pathlib.Path) -> None:
    workspace = _section(_text(root, "Cargo.toml"), "workspace")
    members = _one(r'^members\s*=\s*\[(.*)\]\s*$', workspace, "workspace members").group(1)
    defaults = _one(
        r'^default-members\s*=\s*\[(.*)\]\s*$', workspace, "workspace defaults"
    ).group(1)
    if members.count(f'"{ANCHOR_PATH}"') != 1:
        raise WatchdogError("workspace does not name one anchor")
    if f'"{ANCHOR_PATH}"' in defaults:
        raise WatchdogError("workspace defaults select the anchor")

    anchor_manifest = _text(root, f"{ANCHOR_PATH}/Cargo.toml")
    package = _section(anchor_manifest, "package")
    _one(r'^name\s*=\s*"civsim-stone0-build"\s*$', package, "anchor name")
    _one(r'^build\s*=\s*"build\.rs"\s*$', package, "anchor build path")
    if re.search(r"(?m)^\[dependencies\]\s*$", anchor_manifest):
        raise WatchdogError("anchor exposes runtime dependencies")
    build_dependencies = _section(anchor_manifest, "build-dependencies")
    assignments = [
        line.strip()
        for line in build_dependencies.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    if assignments != ['civsim-stone0 = { path = "../stone0" }']:
        raise WatchdogError("anchor build dependency set changed")

    for client in CLIENTS:
        manifest = _text(root, f"{client}/Cargo.toml")
        _one(
            r'^build\s*=\s*"build\.rs"\s*$',
            _section(manifest, "package"),
            f"{client} build path",
        )
        dependency_lines = [
            line.strip()
            for line in _section(manifest, "build-dependencies").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
        if dependency_lines != [
            'civsim-stone0-build = { path = "../stone0-build" }'
        ]:
            raise WatchdogError(f"{client} does not have the one shared guard dependency")
        build_source = _text(root, f"{client}/build.rs")
        if build_source.count("civsim_stone0_build::assert_guard_linked();") != 1:
            raise WatchdogError(f"{client} sentinel count changed")
        if re.search(r"civsim_stone0::|Mode::|emit_cargo_rerun_inputs", build_source):
            raise WatchdogError(f"{client} regained an independent gate runner")

    build_source = _text(root, f"{ANCHOR_PATH}/build.rs")
    sequence = [
        build_source.find("std::fs::remove_file(&marker)"),
        build_source.find("civsim_stone0::run(civsim_stone0::Mode::Local)"),
        build_source.find("if code != 0"),
        build_source.find("std::fs::write(&marker, MARKER_SOURCE)"),
        build_source.find("cargo:rustc-env=CIVSIM_STONE0_GUARD_MARKER"),
    ]
    if any(position < 0 for position in sequence) or sequence != sorted(sequence):
        raise WatchdogError("anchor run and marker order changed")
    if TOKEN not in build_source:
        raise WatchdogError("anchor marker token changed")

    library = _text(root, f"{ANCHOR_PATH}/src/lib.rs")
    required_library_fragments = (
        f'include!(env!("{ENVIRONMENT_NAME}"))',
        TOKEN,
        "pub fn assert_guard_linked()",
        "generated::token()",
    )
    if any(library.count(fragment) != 1 for fragment in required_library_fragments):
        raise WatchdogError("anchor generated-marker consumer changed")

    canonical_line = _one(
        r"^canonical_packages\s*:=.*$",
        _text(root, "justfile"),
        "canonical package aggregate",
    ).group(0)
    if "civsim-stone0-build" in canonical_line:
        raise WatchdogError("canonical package aggregate selects the anchor")


def issue_receipt(root: pathlib.Path) -> bytes:
    inspect(root)
    files = [
        {
            "path": name,
            "sha256": hashlib.sha256(_bytes(root, name)).hexdigest(),
        }
        for name in sorted(OBSERVED_PATHS)
    ]
    document = {
        "anchor": ANCHOR_PATH,
        "claim_id": CLAIM,
        "consumers": list(CLIENTS),
        "files": files,
        "marker_environment": ENVIRONMENT_NAME,
        "marker_token": TOKEN,
        "producer_implementation": FIRST_IMPLEMENTATION,
        "schema": SCHEMA,
        "watchdog_implementation": SECOND_IMPLEMENTATION,
    }
    return json.dumps(document, sort_keys=True, separators=(",", ":")).encode("ascii")


def self_test() -> None:
    with tempfile.TemporaryDirectory(prefix="stone0-wiring-watchdog-") as name:
        root = pathlib.Path(name)
        for relative in OBSERVED_PATHS:
            destination = root.joinpath(*relative.split("/"))
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REPOSITORY.joinpath(*relative.split("/")), destination)
        issue_receipt(root)
        mutations = (
            ("Cargo.toml", ANCHOR_PATH, "crates/stone0-build-duplicate"),
            (f"{ANCHOR_PATH}/Cargo.toml", "../stone0", "../replacement"),
            (f"{ANCHOR_PATH}/build.rs", "if code != 0", "if false"),
            (f"{ANCHOR_PATH}/src/lib.rs", TOKEN, "replacement-token"),
            ("crates/planet-substrate/build.rs", "assert_guard_linked", "skip_guard"),
        )
        for relative, old, new in mutations:
            path = root.joinpath(*relative.split("/"))
            held = path.read_text(encoding="utf-8")
            path.write_text(held.replace(old, new, 1), encoding="utf-8")
            try:
                inspect(root)
            except WatchdogError:
                pass
            else:
                raise AssertionError(f"watchdog wiring canary survived: {relative}")
            path.write_text(held, encoding="utf-8")
    print("Stone 0 build wiring watchdog self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    options = parser.parse_args()
    try:
        if options.self_test:
            self_test()
            return 0
        output = issue_receipt(REPOSITORY)
        if options.receipt:
            sys.stdout.buffer.write(output + b"\n")
        else:
            print("Stone 0 build wiring watchdog: PASS")
            print(output.decode("ascii"))
    except (AssertionError, OSError, UnicodeError, WatchdogError) as error:
        print(f"Stone 0 build wiring watchdog: FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
